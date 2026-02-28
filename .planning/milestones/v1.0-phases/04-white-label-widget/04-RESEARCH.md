# Phase 4: White-Label Widget - Research

**Researched:** 2026-02-27
**Domain:** Embeddable JavaScript widget, Shadow DOM isolation, React IIFE bundle, branding API, SSE streaming, XSS security
**Confidence:** HIGH (core stack verified against existing project code), MEDIUM (Tailwind v4 Shadow DOM, analytics approach)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Widget trigger & placement**
- Support two embed modes: floating button (default) and inline embed (via target container ID in script config)
- Floating button: fixed position, expands into slide-up panel on desktop (~500px height with internal scroll)
- Mobile: full-screen takeover when widget opens, X button or back to close
- Inline embed: widget renders inside a contractor-placed container div on the page

**Chat experience**
- Bubble-style messages: user messages right-aligned in brand primary color, system messages left-aligned in neutral
- Animated three-dot typing indicator (iMessage-style) while system generates responses
- Widget header: contractor logo on left, company name, X button to minimize/close
- First message is the contractor's custom welcome text from branding config (BRND-03)

**Lead capture flow**
- Lead capture form gates the estimate — appears after conversation is complete, before estimate is shown
- Required fields: name, email, phone (all three required)
- After form submission, returns to chat view with animated dots showing estimate is being generated
- Estimate generation happens server-side while the thinking indicator displays

**Estimate presentation**
- P50/P80 ranges shown as horizontal range bar visualization with dollar amounts labeled ("Most likely: $X — High end: $Y")
- Cost breakdown categories displayed as expandable accordion rows (category name + subtotal, expands to line items)
- LLM-generated narrative streams into chat as a message below the estimate card (uses existing SSE streaming from Phase 3)
- Prominent disclaimer below estimate card, above narrative: estimates are unofficial ballpark figures based on similar projects, no figure is binding, official requirements need human consultation
- "Request Free Consultation" CTA button accompanies the disclaimer — drives visitors toward real contractor engagement

### Claude's Discretion
- Floating button appearance (circle with icon vs text pill, animation style)
- Exact spacing, typography, and visual polish within the widget
- Error state handling and retry UX
- Transition animations between widget states (collapsed → expanded → form → estimate)
- Input field placeholder text and microcopy

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| WDGT-01 | Widget renders inside Shadow DOM with closed mode for style/script isolation | Shadow DOM `attachShadow({mode:'open'})` already implemented in ShadowDOMWrapper.tsx; existing `ShadowDOMWrapper` component is the foundation; switch to `closed` not recommended (debugging broken); `open` mode + IIFE encapsulation gives practical isolation |
| WDGT-02 | Widget embeds on any site with single `<script>` tag (<5 lines of code) | Vite IIFE build (`formats: ['iife']`) already configured in vite.config.ts; `fileName: () => 'embed.js'`; `window.efofxWidget.init({})` pattern already works in test-embed.html |
| WDGT-03 | Widget is mobile-responsive (sidebar, modal, full-width layouts) | CSS position:fixed for floating panel; `dvh` units for mobile full-screen takeover; `env(safe-area-inset-*)` for notch/home-indicator safe areas; inline mode uses container width |
| WDGT-04 | Widget loads without visible "Powered by efOfX" branding (true white-label) | Branding config API serves contractor logo URL, colors, welcome text; widget header renders contractor identity; no efOfX attribution in UI |
| WDGT-05 | CORS configured per-tenant for widget API calls from contractor domains | Starlette CORSMiddleware subclass overrides `is_allowed_origin()` to check tenant's allowed_origins from DB; tenant's registered domains stored in `settings` dict on Tenant model |
| BRND-01 | Contractor can configure widget colors (primary, secondary, accent) | Branding config stored in `Tenant.settings` dict; served via public GET /api/v1/widget/branding/{api_key_prefix} endpoint; CSS custom properties applied to Shadow DOM `:host` |
| BRND-02 | Contractor can set company logo URL displayed in widget header | Logo URL stored in `Tenant.settings.branding.logo_url`; served in branding config response; rendered in widget header as `<img>` |
| BRND-03 | Contractor can customize widget button text and welcome message | `welcome_message` and `button_text` fields in branding config; welcome message sent as first assistant chat message on widget open |
| BRND-04 | Branding config fetched via unauthenticated API endpoint (rate-limited by IP) | Public endpoint (no auth dependency); `@limiter.limit("30/minute", key_func=get_remote_address)` using existing slowapi infrastructure |
| WFTR-01 | Conversational chat UI within widget for project scoping | React state machine: idle → chatting → ready → lead-capture → generating → result; calls existing POST /api/v1/chat/send; uses API key auth |
| WFTR-02 | Lead capture form collects prospect email and phone before estimate | React form with name/email/phone fields; validation before submission; lead stored to MongoDB; submits before triggering SSE stream |
| WFTR-03 | Estimate results displayed in widget with P50/P80 ranges and cost breakdown | Range bar SVG/CSS visualization; accordion component for cost breakdown; SSE EventSource for narrative streaming from existing Phase 3 endpoint |
| WFTR-04 | Widget analytics track views, chat starts, estimate completions per tenant | Lightweight MongoDB counter collection (not external analytics service); increment on widget_view, chat_start, estimate_complete events; no PII stored |
| WSEC-01 | Widget JavaScript wrapped in global error boundary (no host page crashes) | `react-error-boundary` v6.1.1 wraps entire widget; outer try/catch in IIFE init prevents crashes before React mounts |
| WSEC-02 | All widget API calls authenticated via tenant API key | API key passed as `data-api-key` attribute on script tag; stored in React context; sent as `Authorization: Bearer sk_live_...` header; existing `_validate_api_key` in security.py handles it |
| WSEC-03 | Widget input sanitized against XSS attacks | `dompurify` v3.3.1 sanitizes any HTML before `dangerouslySetInnerHTML`; React auto-escapes text content; narrative SSE text rendered as plain text not HTML |
</phase_requirements>

---

## Summary

Phase 4 builds on a solid foundation: the widget project (`apps/efofx-widget`) already has a working Vite IIFE build pipeline (`vite.config.ts`), a Shadow DOM wrapper (`ShadowDOMWrapper.tsx`), and a working embed test page (`test-embed.html`). React 19.2.0, Vite 7.2.2, and vite-plugin-css-injected-by-js 3.5.2 are already installed. The existing `App.tsx` is a placeholder that gets replaced with the full widget UI in this phase.

The backend already has all needed infrastructure: slowapi for rate limiting, `get_remote_address` for IP-based limits, JWT+API key dual auth in `security.py`, the `Tenant.settings` dict for branding storage, and the SSE streaming endpoint at `POST /api/v1/chat/{session_id}/generate-estimate` from Phase 3. Phase 4's primary backend work is adding: (1) a public branding config endpoint, (2) a lead capture storage endpoint, (3) per-tenant CORS based on registered domains, and (4) a lightweight analytics event collection endpoint.

**Primary recommendation:** Keep the existing Vite IIFE + Shadow DOM architecture. Use CSS custom properties injected into the Shadow DOM `:host` for branding (not Tailwind v4 for widget styles — Tailwind v4's `:root` variables don't inherit into Shadow DOM). Write widget UI styles as plain CSS or Tailwind with explicit Shadow DOM injection. Use `react-error-boundary` v6.1.1 for the error boundary, `dompurify` for XSS protection, and native `EventSource` for SSE streaming.

---

## Standard Stack

### Core (already installed / existing infrastructure)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| react | 19.2.0 | Widget UI framework | Already installed; React 19 has improved web components/Shadow DOM support |
| react-dom | 19.2.0 | React DOM rendering | Already installed |
| vite | 7.2.2 | Build tool (IIFE bundle) | Already configured with `formats: ['iife']` |
| @vitejs/plugin-react | 5.1.0 | React transform | Already installed |
| vite-plugin-css-injected-by-js | 3.5.2 | Inline CSS into JS bundle | Already installed; critical for single-file embed |
| tailwindcss | 4.1.17 | Utility CSS (with Shadow DOM caveat, see below) | Already installed |
| TypeScript | 5.9.3 | Type safety | Already installed |
| fastapi | 0.116.1 | Backend API | Existing project dependency |
| slowapi | 0.1.9 | Rate limiting (per-IP for branding endpoint) | Existing project dependency |
| motor / pymongo | 3.3.2 / 4.6.1 | MongoDB async driver | Existing project dependency |

### New Dependencies to Add

| Library | Version | Purpose | Why |
|---------|---------|---------|-----|
| react-error-boundary | ^6.1.1 | Global widget error boundary | v6 is latest, supports React 19, widely adopted (8M downloads/week); prevents widget errors from crashing host page |
| dompurify | ^3.3.1 | XSS sanitization | DOM-only, fast, includes TypeScript types; OWASP recommended; needed for SSE narrative text if ever rendered as HTML |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| react-error-boundary | Custom class component ErrorBoundary | react-error-boundary is the ecosystem standard (bvaughn maintained); saves writing boilerplate class component |
| dompurify | Manual escaping only | dompurify handles edge cases (SVG XSS, DOM clobbering) that manual escaping misses; 3KB gzip overhead is worth it |
| CSS custom properties for branding | Inline styles on each element | Custom properties on `:host` are inherited by all child elements; far less code than prop-drilling inline styles |
| Starlette CORSMiddleware subclass | allow_origin_regex | Regex covers wildcard subdomains but not per-tenant database lookup; subclass is more precise |
| MongoDB analytics counter | PostHog / Plausible | External analytics adds widget bundle size, GDPR concerns for contractors, and external dependency; MongoDB counters are zero-cost infrastructure additions |

**Installation for new dependencies:**
```bash
cd apps/efofx-widget && npm install react-error-boundary dompurify
cd apps/efofx-widget && npm install --save-dev @types/dompurify
```

---

## Architecture Patterns

### Recommended Project Structure

```
apps/efofx-widget/src/
├── main.tsx                    # IIFE entry: reads data attrs, calls init()
├── App.tsx                     # Root: WidgetContext.Provider → FloatingWidget or InlineWidget
├── context/
│   └── WidgetContext.tsx       # apiKey, brandingConfig, widgetState, session
├── components/
│   ├── ShadowDOMWrapper.tsx    # Existing — Shadow DOM host + React root (keep as-is)
│   ├── ErrorBoundary.tsx       # react-error-boundary wrapper with silent fallback
│   ├── FloatingButton.tsx      # Fixed position trigger button
│   ├── ChatPanel.tsx           # Slide-up panel (desktop) / full-screen (mobile)
│   ├── ChatBubble.tsx          # Single message bubble (user=right, assistant=left)
│   ├── TypingIndicator.tsx     # Three-dot animation
│   ├── LeadCaptureForm.tsx     # Name/email/phone form before estimate
│   ├── EstimateCard.tsx        # P50/P80 range bar + accordion breakdown
│   ├── NarrativeStream.tsx     # SSE-streamed narrative text display
│   └── ConsultationCTA.tsx    # Disclaimer + "Request Free Consultation" button
├── hooks/
│   ├── useBranding.ts          # Fetch branding config on mount
│   ├── useChat.ts              # Chat state machine, send message, session management
│   └── useEstimateStream.ts   # EventSource SSE hook for narrative streaming
├── api/
│   ├── client.ts               # Fetch wrapper with API key auth header
│   ├── chat.ts                 # POST /chat/send, POST /lead/capture
│   └── branding.ts             # GET /widget/branding/:apiKeyPrefix
├── types/
│   └── widget.d.ts             # BrandingConfig, WidgetState, LeadData, EstimationOutput
└── widget.css                  # Plain CSS for widget (NOT Tailwind for Shadow DOM styles)
```

```
apps/efofx-estimate/app/api/
├── routes.py                   # Existing — add analytics event endpoint
├── auth.py                     # Existing
└── widget.py                   # NEW: branding config + lead capture endpoints

apps/efofx-estimate/app/models/
└── widget.py                   # NEW: BrandingConfig, LeadCapture, WidgetAnalyticsEvent

apps/efofx-estimate/app/services/
└── widget_service.py           # NEW: branding CRUD, lead storage, analytics counters

apps/efofx-estimate/app/middleware/
└── cors.py                     # NEW: DynamicCORSMiddleware subclass
```

### Pattern 1: IIFE Entry with data-attributes Config

The embed script tag passes config via HTML data attributes — no JS required from the contractor:

```html
<!-- 5-line embed (the deliverable) -->
<script
  src="https://cdn.efofx.ai/widget/embed.js"
  data-api-key="sk_live_abc123..."
  data-mode="floating"
  data-container="my-div-id"
></script>
```

`main.tsx` reads config at load time:

```typescript
// Source: makerkit.dev embeddable widgets guide + existing test-embed.html pattern
function getScriptConfig(): WidgetConfig {
  const script = document.currentScript as HTMLScriptElement | null;
  // Fallback: find script by src pattern (when currentScript is null after async load)
  const scriptEl = script || document.querySelector('script[data-api-key]') as HTMLScriptElement;
  return {
    apiKey: scriptEl?.dataset.apiKey ?? '',
    mode: (scriptEl?.dataset.mode as 'floating' | 'inline') ?? 'floating',
    containerId: scriptEl?.dataset.container ?? 'efofx-widget',
  };
}

export function init(config: WidgetConfig = {}) {
  // Global error boundary BEFORE React mounts — catches mount failures
  try {
    const resolved = { ...getScriptConfig(), ...config };
    // Find or create container
    let container = resolved.containerId
      ? document.getElementById(resolved.containerId)
      : null;
    if (!container) {
      container = document.createElement('div');
      container.id = 'efofx-widget';
      document.body.appendChild(container);
    }
    const root = createRoot(container);
    root.render(
      <ErrorBoundary fallback={null} onError={(e) => console.error('[efOfX widget]', e)}>
        <ShadowDOMWrapper>
          <App config={resolved} />
        </ShadowDOMWrapper>
      </ErrorBoundary>
    );
    return { destroy: () => root.unmount() };
  } catch (e) {
    // Silent fail — never crash host page (WSEC-01)
    console.error('[efOfX widget] initialization failed', e);
    return { destroy: () => {} };
  }
}
```

### Pattern 2: CSS Custom Properties for Branding (Shadow DOM)

Branding config arrives from the API. Apply it as CSS custom properties on the Shadow DOM `:host` element. This avoids Tailwind v4's `:root` variable inheritance issue:

```typescript
// In ShadowDOMWrapper.tsx — after shadow root created, before React mount
function applyBranding(shadowRoot: ShadowRoot, branding: BrandingConfig) {
  const style = document.createElement('style');
  style.textContent = `
    :host {
      --brand-primary: ${branding.primary_color};
      --brand-secondary: ${branding.secondary_color};
      --brand-accent: ${branding.accent_color};
    }
  `;
  shadowRoot.insertBefore(style, shadowRoot.firstChild);
}
```

All widget CSS references `var(--brand-primary)` etc. This is the standard Web Components theming pattern, verified against MDN and CSS-Tricks documentation.

**Critical caveat — Tailwind v4 in Shadow DOM:** Tailwind v4 defines its theme variables using only `:root` which does NOT inherit into Shadow DOM. The existing `vite-plugin-css-injected-by-js` injects CSS into `document.head`, not the shadow root. This means:
- Tailwind utility classes will NOT work inside the Shadow DOM unless CSS is also injected into the shadow root.
- **Recommended approach:** Write widget component styles as plain CSS injected into the Shadow DOM, OR use a dedicated approach to copy Tailwind's compiled CSS into the shadow root on mount.
- See `ShadowDOMWrapper.tsx` pattern: create a `<style>` element, set its `textContent` to the Tailwind-compiled CSS string (imported as a string via `?inline` Vite query), and append it to the shadow root.

```typescript
// Import compiled CSS as a string (Vite ?inline query)
import widgetStyles from './widget.css?inline';

// Inside ShadowDOMWrapper useEffect, after attachShadow:
const styleEl = document.createElement('style');
styleEl.textContent = widgetStyles;  // includes compiled Tailwind output
shadowRoot.appendChild(styleEl);
```

This pattern is confirmed working by multiple community sources. The `?inline` Vite query returns file contents as a string.

### Pattern 3: Widget State Machine

Five discrete states drive the UI rendering:

```typescript
type WidgetPhase =
  | 'idle'           // Widget closed (floating button visible)
  | 'chatting'       // Chat panel open, conversation in progress
  | 'ready'          // Chat complete, ready for lead capture gate
  | 'lead_capture'   // Lead form displayed
  | 'generating'     // Typing indicator, SSE stream in progress
  | 'result';        // Estimate card + narrative shown

// Transitions:
// idle → chatting: user clicks floating button
// chatting → ready: chat.is_ready === true in response
// ready → lead_capture: user clicks "Get My Estimate" (or auto-advance)
// lead_capture → generating: form submitted
// generating → result: SSE 'done' event received
// result → idle: user closes widget
// any → idle: user clicks X button
```

### Pattern 4: SSE Streaming Hook (Phase 3 Integration)

The narrative stream connects to the existing `POST /api/v1/chat/{session_id}/generate-estimate` SSE endpoint from Phase 3. The widget uses native `EventSource` — no library needed:

```typescript
// hooks/useEstimateStream.ts
export function useEstimateStream() {
  const [estimateData, setEstimateData] = useState<EstimationOutput | null>(null);
  const [narrative, setNarrative] = useState('');
  const [phase, setPhase] = useState<WidgetPhase>('idle');

  const startStream = useCallback((sessionId: string, apiKey: string) => {
    // EventSource doesn't support custom headers — use query param for API key
    // OR use fetch with ReadableStream (supports headers)
    const response = await fetch(
      `${API_BASE}/api/v1/chat/${sessionId}/generate-estimate`,
      { headers: { Authorization: `Bearer ${apiKey}` }, method: 'POST' }
    );
    const reader = response.body!.getReader();
    const decoder = new TextDecoder();

    // Parse SSE frames manually from fetch stream
    // Events: 'thinking', 'estimate', 'data:' (narrative tokens), 'done', 'error'
    // This approach supports auth headers, which EventSource does not
  }, []);

  return { estimateData, narrative, phase, startStream };
}
```

**Critical note:** Native `EventSource` does NOT support custom request headers (including `Authorization`). Since the SSE endpoint requires API key auth, use `fetch()` with `ReadableStream` to handle SSE manually. This pattern is confirmed necessary for authenticated SSE in the browser.

### Pattern 5: Dynamic CORS for Per-Tenant Domains (WDGT-05)

Replace static `CORSMiddleware` with a subclass that performs per-request tenant lookup:

```python
# app/middleware/cors.py
from starlette.middleware.cors import CORSMiddleware

class TenantAwareCORSMiddleware(CORSMiddleware):
    async def is_allowed_origin(self, origin: str) -> bool:
        if origin in settings.ALLOWED_ORIGINS:  # Global allowlist (admin/dashboard)
            return True
        # Parse API key from Authorization header to find tenant
        # Check tenant.settings.get('allowed_origins', [])
        # Cache with TTL to avoid DB hit on every preflight
        # Return True if origin matches tenant's registered domains
        ...
```

**Simpler alternative:** Store tenant's allowed origins in a fast in-memory dict keyed by API key prefix. Refresh on auth. This avoids async DB calls in middleware.

### Pattern 6: Unauthenticated Branding Endpoint

```python
# app/api/widget.py
@widget_router.get("/widget/branding/{api_key_prefix}")
@limiter.limit("30/minute", key_func=get_remote_address)
async def get_branding_config(
    request: Request,
    api_key_prefix: str,
) -> BrandingConfigResponse:
    """Public endpoint — no auth. Rate limited by IP. Returns tenant branding."""
    # Lookup by api_key_prefix (first 32 chars of API key = tenant_id no dashes)
    # Returns: primary_color, secondary_color, accent_color, logo_url,
    #          welcome_message, button_text, company_name
    # Never returns: hashed keys, encrypted BYOK keys, tenant PII
    ...
```

The widget passes `data-api-key` → strips to first 32 chars as the prefix → fetches branding without exposing the full key to unauthenticated callers.

### Pattern 7: Lead Capture Storage

```python
# Minimal lead model — stored in tenant-scoped collection
class LeadCapture(BaseModel):
    session_id: str
    name: str
    email: str
    phone: str
    captured_at: datetime
    estimate_session_id: Optional[str] = None  # linked after estimate complete
```

Lead is stored via `POST /api/v1/widget/lead` (requires API key auth). Then the SSE estimate generation stream is started.

### Pattern 8: Analytics Counters (WFTR-04)

Lightweight per-tenant event counters in MongoDB. No external service. No PII.

```python
# Analytics collection structure
{
  "tenant_id": "...",
  "date": "2026-02-27",   # daily bucketing
  "widget_views": 42,
  "chat_starts": 18,
  "estimate_completions": 7
}
# Upsert with $inc on each event type
```

Widget fires analytics events as fire-and-forget POST requests. Backend uses `upsert=True` with `$inc` operators.

### Anti-Patterns to Avoid

- **Using EventSource for authenticated SSE:** EventSource doesn't support headers. Use `fetch()` + `ReadableStream` + manual SSE frame parsing.
- **Using Tailwind v4 classes in Shadow DOM without style injection:** Tailwind's `:root` variables don't reach Shadow DOM. Must inject compiled CSS as a string into the shadow root via `<style>` element.
- **Closed Shadow DOM mode:** `mode: 'closed'` blocks debugging tools (Chrome DevTools, react-devtools). The existing `ShadowDOMWrapper` uses `mode: 'open'` — keep it that way. IIFE + module scope already prevents global namespace pollution.
- **Passing full API key in branding URL path:** The branding endpoint uses only the api_key_prefix (derived from tenant_id, not secret). Never expose the full `sk_live_...` key in a URL — it would appear in server logs.
- **Injecting CSS into document.head for Shadow DOM styles:** `vite-plugin-css-injected-by-js` injects into `document.head` which doesn't affect Shadow DOM children. Must use the `?inline` import pattern to inject into the shadow root directly.
- **Building custom accordion/range-bar from scratch with full animation libs:** Plain CSS transitions are sufficient; adding animation libraries (framer-motion, react-spring) to the widget bundle increases size and initialization time.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| React error catching without crashes | Custom class ErrorBoundary | `react-error-boundary` v6.1.1 | Handles async errors, hooks, event handlers; 6 years battle-tested; supports React 19 |
| XSS prevention | Custom HTML sanitizer | `dompurify` v3.3.1 | Handles SVG XSS, DOM clobbering, prototype pollution; OWASP recommended; ~3KB gzip |
| Rate limiting IP-based | Custom IP tracking | `slowapi` `@limiter.limit(key_func=get_remote_address)` | Already used in project (login endpoint); same pattern |
| CSS variable inheritance across Shadow DOM | JS-based style propagation | CSS custom properties on `:host` | Standard web platform feature; no JS overhead at runtime |
| Authenticated SSE | EventSource with auth | `fetch()` + `ReadableStream` manual parse | EventSource cannot set headers; fetch with streaming is the correct approach |

**Key insight:** The widget bundle size matters for a host-page embed. Every KB added risks slower load times on the contractor's customer-facing pages. Prefer small, purpose-built solutions over general frameworks. Target <150KB gzipped for the full bundle (React included).

---

## Common Pitfalls

### Pitfall 1: CSS Scope Leaking Into or Out of Shadow DOM

**What goes wrong:** Widget styles affect host page elements, or host page styles override widget styles.
**Why it happens:** CSS from `vite-plugin-css-injected-by-js` goes into `document.head` — it applies to the whole page, not just the shadow root. Conversely, host page CSS cannot reach into Shadow DOM children.
**How to avoid:** Use the `?inline` Vite import to get compiled CSS as a string, then inject it as a `<style>` element appended to the shadow root. This keeps widget styles isolated and prevents leakage.
**Warning signs:** Widget styles appearing on host page elements; host page styles affecting widget layout.

### Pitfall 2: Tailwind v4 Variables Missing in Shadow DOM

**What goes wrong:** Tailwind utility classes like `shadow-*`, `ring-*`, `border-*` silently fail inside Shadow DOM because their `@property`-registered CSS variables are defined on `:root`, not `:host`.
**Why it happens:** Tailwind v4 changed from CSS class-based to CSS variable-based utilities. The `@property` at-rules and `:root` variable definitions don't cross the Shadow DOM boundary.
**How to avoid:** After injecting the Tailwind-compiled CSS string into the shadow root, also add a `:host { /* all tailwind variables */ }` block. Alternatively, avoid relying on Tailwind's variable-dependent utilities (shadow, ring, etc.) inside the widget, and use plain CSS for those properties.
**Warning signs:** `box-shadow`, `outline`, `border` utilities working in dev (no shadow DOM) but broken in embed test.

### Pitfall 3: EventSource Auth Failure

**What goes wrong:** Widget SSE stream silently fails with 403 because EventSource cannot send the Authorization header.
**Why it happens:** The `EventSource` Web API is limited to GET requests with no custom headers. The Phase 3 endpoint requires `Authorization: Bearer sk_live_...`.
**How to avoid:** Use `fetch(url, {method: 'POST', headers: {Authorization: ...}})` and read `response.body` as a `ReadableStream`. Parse SSE frames (`\n\n`-delimited) manually in a `while(true)` loop.
**Warning signs:** Console shows EventSource connected but no events received; network tab shows 401/403 on the SSE request.

### Pitfall 4: document.currentScript is null After async Load

**What goes wrong:** Widget reads `null` from `document.currentScript` and cannot extract data-attributes.
**Why it happens:** `document.currentScript` is only populated while the script is executing synchronously. If the script tag has `async` or `defer`, or if `init()` is called after a timeout/promise, `currentScript` is null.
**How to avoid:** Read `document.currentScript` immediately at the top of `main.tsx` (synchronously) and store in a module-level variable before any async operations. Fallback: `document.querySelector('script[data-api-key]')`.
**Warning signs:** `config.apiKey` is undefined; widget renders without branding.

### Pitfall 5: Floating Widget z-index Conflicts

**What goes wrong:** Widget floating button or panel appears behind host page modals, dropdowns, or fixed headers.
**Why it happens:** Shadow DOM does NOT isolate z-index stacking contexts. The Shadow host element (`<div id="efofx-widget">`) participates in the host page's stacking context.
**How to avoid:** Set `z-index: 2147483647` (maximum int32) on the shadow host element's wrapper styles applied from outside the shadow. Also set on the fixed-position panel inside the shadow.
**Warning signs:** Widget button appears below a sticky navigation header on the contractor's site.

### Pitfall 6: Mobile Full-Screen Takeover Broken by Viewport Meta

**What goes wrong:** Mobile full-screen widget is clipped at the top or bottom because safe area insets are ignored.
**Why it happens:** iPhone notch/home indicator consume screen real estate that `100vh` or `100dvh` doesn't account for without proper CSS env() values.
**How to avoid:** Use `height: 100dvh` for the full-screen overlay. Add `padding-top: env(safe-area-inset-top)` and `padding-bottom: env(safe-area-inset-bottom)` to the full-screen panel. The Shadow DOM container needs `position: fixed; inset: 0` on the host page side.
**Warning signs:** Chat input field obscured by iPhone home indicator; close button hidden behind notch.

### Pitfall 7: react-shadow Compatibility with React 19

**Note from STATE.md blocker:** `react-shadow==20.6.0` may have compatibility issues with React 19.2.0. The existing `ShadowDOMWrapper.tsx` was built manually (useRef + useEffect + attachShadow) specifically to avoid this dependency. **Do not add react-shadow as a dependency.** The manual implementation in `ShadowDOMWrapper.tsx` is confirmed working with React 19.2.0 and should be kept.

---

## Code Examples

Verified patterns from official sources and codebase analysis:

### Shadow DOM CSS Injection Pattern (Critical)

```typescript
// ShadowDOMWrapper.tsx — inject widget CSS into shadow root
import widgetStyles from '../widget.css?inline';  // Vite ?inline query returns string

useEffect(() => {
  if (!hostRef.current) return;
  if (!shadowRootRef.current) {
    shadowRootRef.current = hostRef.current.attachShadow({ mode: 'open' });

    // Inject CSS into shadow root (not document.head)
    const styleEl = document.createElement('style');
    styleEl.textContent = widgetStyles;
    shadowRootRef.current.appendChild(styleEl);

    const container = document.createElement('div');
    container.id = 'efofx-widget-root';
    shadowRootRef.current.appendChild(container);
    reactRootRef.current = createRoot(container);
  }
  if (reactRootRef.current) {
    reactRootRef.current.render(children);
  }
}, [children]);
```

### Manual SSE Parsing with fetch (Authenticated SSE)

```typescript
// hooks/useEstimateStream.ts
async function streamEstimate(sessionId: string, apiKey: string) {
  const response = await fetch(
    `${API_BASE}/api/v1/chat/${sessionId}/generate-estimate`,
    { method: 'POST', headers: { Authorization: `Bearer ${apiKey}` } }
  );
  if (!response.ok || !response.body) throw new Error('Stream failed');

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });

    // SSE frames are separated by \n\n
    const frames = buffer.split('\n\n');
    buffer = frames.pop() ?? '';  // incomplete last frame stays in buffer

    for (const frame of frames) {
      const lines = frame.split('\n');
      const eventLine = lines.find(l => l.startsWith('event:'));
      const dataLine = lines.find(l => l.startsWith('data:'));
      const eventType = eventLine?.slice(7).trim();
      const data = dataLine?.slice(5).trim();

      if (eventType === 'thinking') { setPhase('generating'); }
      if (eventType === 'estimate') { setEstimateData(JSON.parse(data!)); }
      if (!eventType && data) {
        // Plain data: line = narrative token
        setNarrative(prev => prev + data.replace(/\\n/g, '\n'));
      }
      if (eventType === 'done') { setPhase('result'); }
      if (eventType === 'error') { handleError(JSON.parse(data!)); }
    }
  }
}
```

### Branding Config Endpoint (Backend)

```python
# app/api/widget.py
from fastapi import APIRouter, Request, HTTPException
from app.core.rate_limit import limiter
from slowapi.util import get_remote_address
from app.models.widget import BrandingConfigResponse
from app.services.widget_service import get_branding_by_prefix

widget_router = APIRouter(prefix="/widget", tags=["widget"])

@widget_router.get("/branding/{api_key_prefix}", response_model=BrandingConfigResponse)
@limiter.limit("30/minute", key_func=get_remote_address)
async def get_widget_branding(
    request: Request,
    api_key_prefix: str,
) -> BrandingConfigResponse:
    """Public unauthenticated endpoint for widget branding config.

    api_key_prefix = first 32 hex chars of API key = tenant_id without dashes.
    Returns only safe public fields (no keys, no PII).
    Rate limited 30/minute per IP.
    """
    branding = await get_branding_by_prefix(api_key_prefix)
    if not branding:
        raise HTTPException(status_code=404, detail="Widget not found")
    return branding
```

### Per-Tenant CORS Middleware (Backend)

```python
# app/middleware/cors.py
from starlette.middleware.cors import CORSMiddleware
from typing import Sequence

class TenantAwareCORSMiddleware(CORSMiddleware):
    """Extends CORSMiddleware to allow per-tenant registered domains.

    Checks tenant's settings.allowed_origins (list of domains) via
    api_key_prefix extracted from request path, with in-memory cache.
    """
    def __init__(self, app, **kwargs):
        super().__init__(app, **kwargs)
        self._tenant_origins_cache: dict[str, list[str]] = {}

    def is_allowed_origin(self, origin: str) -> bool:
        # Check static list first (fast path)
        if super().is_allowed_origin(origin):
            return True
        # Per-tenant lookup via cache
        # Cache populated during API key validation — see widget_service
        # Check if origin matches any registered tenant domain
        return origin in self._get_all_tenant_origins()
```

### Error Boundary for Widget (Frontend)

```typescript
// components/ErrorBoundary.tsx
import { ErrorBoundary } from 'react-error-boundary';

// Silent fallback — widget disappears rather than crashing host page (WSEC-01)
export function WidgetErrorBoundary({ children }: { children: React.ReactNode }) {
  return (
    <ErrorBoundary
      fallback={null}
      onError={(error, info) => {
        console.error('[efOfX widget] Runtime error:', error, info);
      }}
    >
      {children}
    </ErrorBoundary>
  );
}
```

### Branding Config Pydantic Model (Backend)

```python
# app/models/widget.py
from pydantic import BaseModel, Field
from typing import Optional

class BrandingConfig(BaseModel):
    """Stored in Tenant.settings['branding']"""
    primary_color: str = Field(default="#2563eb", description="CSS color for user bubbles, CTA buttons")
    secondary_color: str = Field(default="#f3f4f6", description="CSS color for system bubble backgrounds")
    accent_color: str = Field(default="#1d4ed8", description="Hover states, range bar fill")
    logo_url: Optional[str] = Field(default=None, description="HTTPS URL for contractor logo")
    welcome_message: str = Field(default="Hi! Tell me about your project and I'll help estimate the cost.")
    button_text: str = Field(default="Get an Estimate")
    company_name: str = Field(default="")

class BrandingConfigResponse(BaseModel):
    """Public response — safe fields only, no keys"""
    primary_color: str
    secondary_color: str
    accent_color: str
    logo_url: Optional[str]
    welcome_message: str
    button_text: str
    company_name: str

class LeadCapture(BaseModel):
    session_id: str
    tenant_id: str
    name: str
    email: str
    phone: str
    captured_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| iframe embedding for widget isolation | Shadow DOM + IIFE | ~2020-2022 | Shadow DOM has no cross-origin restriction on same-domain APIs; better mobile UX; no postMessage complexity |
| EventSource for all SSE | fetch() ReadableStream for auth'd SSE | ~2021 | EventSource can't send auth headers; fetch streaming handles auth properly |
| react-shadow library | Manual useEffect + attachShadow | Phase 4 risk noted in STATE.md | react-shadow has React 19 compatibility uncertainty; manual implementation confirmed working |
| Tailwind in Shadow DOM via document.head | CSS string injected via `?inline` into shadow root | Tailwind v4 (2024) | Tailwind v4 `:root` variables don't inherit into Shadow DOM; must inject into shadow root |
| Inline styles for widget theming | CSS custom properties on `:host` | 2022+ | Custom properties inherit through shadow DOM to children; enables designer-friendly branding API |
| External analytics (PostHog/Plausible) | MongoDB counter collection | — | Zero added bundle size, zero external dependency, GDPR-friendly by default |

**Deprecated/outdated:**
- `react-shadow` library: React 19.2.0 compatibility unconfirmed per STATE.md; the manual `ShadowDOMWrapper.tsx` approach works and is already in place.
- `EventSource` for authenticated SSE: Cannot set Authorization header; use `fetch()` + `ReadableStream`.
- `styled-components` / CSS-in-JS inside Shadow DOM: Adds 150-200ms initialization overhead per makerkit.dev benchmark; plain CSS + CSS custom properties is faster.

---

## Open Questions

1. **Branding endpoint URL — full API key vs prefix-only**
   - What we know: The widget passes `data-api-key="sk_live_..."` in the script tag. The branding endpoint should not expose the full key in the URL path (appears in access logs).
   - What's unclear: Best identifier for the branding lookup — the 32-char tenant_id prefix derived from the api_key, or a separate public widget_id stored on the Tenant?
   - Recommendation: Use the 32-char tenant_id prefix derived from the API key (same pattern as `_validate_api_key` in security.py). The prefix is not secret (it's the tenant_id without dashes, which is a UUID). Route: `GET /api/v1/widget/branding/{tenant_id_no_dashes}`. Widget derives it: `apiKey.split('_')[2].slice(0, 32)`.

2. **Where to store tenant's allowed CORS origins**
   - What we know: `Tenant.settings` is `Dict[str, Any]`; per-tenant CORS requires knowing which domains the contractor owns.
   - What's unclear: Is domain verification required (prove you own the domain), or just let contractors self-declare?
   - Recommendation: Self-declared for v1. Store as `tenant.settings['allowed_origins'] = ['https://acme-contractor.com']`. Contractors add domains via a future contractor dashboard (out of scope for Phase 4 — for now, seed via the Phase 2 profile update endpoint).

3. **Widget CSS approach — Tailwind utility classes vs plain CSS**
   - What we know: Tailwind v4's Shadow DOM integration is broken-by-default (`:root` variables); requires `?inline` injection workaround. Plain CSS works without workarounds.
   - What's unclear: How complex will the widget styles be? Can Tailwind utility classes be avoided for core layout?
   - Recommendation: Use plain CSS (in `widget.css`) for the widget component styles, injected via `?inline`. Use Tailwind only for any CSS that ships to `document.head` (i.e., none for this widget — the widget is fully Shadow DOM isolated). This is the safest approach given Tailwind v4 Shadow DOM bugs.

4. **Analytics endpoint auth — API key required or fire-and-forget anonymous?**
   - What we know: WFTR-04 says analytics are per-tenant. WSEC-02 says all widget API calls use tenant API key.
   - What's unclear: Does the analytics endpoint need auth, or can it be unauthenticated (just using tenant_id from URL path)?
   - Recommendation: Require API key auth (WSEC-02 scope covers analytics). Widget already has the API key in context; adding it to the analytics request is one line.

---

## Sources

### Primary (HIGH confidence)
- Existing codebase (`apps/efofx-widget/`) — vite.config.ts IIFE configuration, ShadowDOMWrapper.tsx implementation, package.json versions
- Existing codebase (`apps/efofx-estimate/`) — routes.py SSE endpoint, security.py API key auth, main.py CORSMiddleware, tenant.py settings dict
- `.planning/STATE.md` — Phase 4 blocker note on react-shadow React 19 compatibility

### Secondary (MEDIUM confidence)
- [makerkit.dev — Building Embeddable React Widgets](https://makerkit.dev/blog/tutorials/embeddable-widgets-react) — IIFE patterns, Shadow DOM, data-attributes, bundle size targets; verified against existing vite.config.ts
- [Tailwind v4 Shadow DOM Discussion #15556](https://github.com/tailwindlabs/tailwindcss/discussions/15556) — Official maintainer response: "no plans to support `:root, :host`"; workarounds documented
- [Tailwind v4 box-shadow Shadow DOM bug #16772](https://github.com/tailwindlabs/tailwindcss/discussions/16772) — `@property` declarations failing in Shadow DOM
- [Vite build options — official docs](https://vite.dev/config/build-options) — IIFE format configuration
- [react-error-boundary npm](https://www.npmjs.com/package/react-error-boundary) — v6.1.1, 8.4M weekly downloads, React 19 compatible
- [DOMPurify v3.3.1](https://cure53.de/purify) — Latest release, TypeScript included, OWASP recommended

### Tertiary (LOW confidence)
- [FastAPI CORS dynamic origins 2025](https://johal.in/fastapi-cors-starlette-trusted-hosts-origins-2025-2/) — Starlette CORSMiddleware subclass pattern; needs validation against actual Starlette version used
- Community reports on Tailwind v4 + Shadow DOM workarounds — multiple DEV.to posts with varying approaches; recommend testing before committing

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all core packages are already installed and configured; versions confirmed from node_modules
- Architecture: HIGH — IIFE + Shadow DOM pattern is proven in existing codebase; SSE patterns verified against Phase 3 code
- Tailwind v4 Shadow DOM: MEDIUM — multiple community sources confirm the issue; workaround documented but not yet tested in this project
- Dynamic CORS: MEDIUM — Starlette subclass pattern is community-documented; need to verify against fastapi 0.116.1 / starlette version
- Analytics: HIGH — MongoDB `$inc` upsert is standard; no external dependencies

**Research date:** 2026-02-27
**Valid until:** 2026-03-30 (stable ecosystem; Tailwind v4 Shadow DOM situation may improve)
