import { createRoot } from 'react-dom/client';
import { ErrorBoundary } from '@efofx/ui';
import ShadowDOMWrapper from './components/ShadowDOMWrapper';
import App from './App';
import type { WidgetConfig, WidgetMode } from './types/widget';

/**
 * efOfX Widget — Embeddable estimation chat widget
 *
 * Embed via:
 *   <script src="embed.js"
 *     data-api-key="sk_live_..."
 *     data-mode="floating"
 *     data-container="my-container-id">
 *   </script>
 *
 * document.currentScript is only available during synchronous module execution,
 * so we capture it immediately at the top level before any async operations.
 */
const _scriptEl = document.currentScript as HTMLScriptElement | null;

/**
 * Read WidgetConfig from data attributes on the script tag.
 * Falls back to querying for any script with data-api-key if currentScript is null.
 */
function getScriptConfig(): Partial<WidgetConfig> {
  const el = _scriptEl ?? document.querySelector<HTMLScriptElement>('script[data-api-key]');
  if (!el) return {};

  const apiKey = el.getAttribute('data-api-key') ?? '';
  const mode = (el.getAttribute('data-mode') ?? 'floating') as WidgetMode;
  const containerId = el.getAttribute('data-container') ?? 'efofx-widget';

  return { apiKey, mode, containerId };
}

/**
 * Initialize the widget.
 *
 * Wraps all initialization in a try/catch — on any error the widget silently
 * fails and does NOT crash the host page (WSEC-01).
 */
export function init(config: Partial<WidgetConfig> = {}): { destroy: () => void } {
  try {
    const scriptConfig = getScriptConfig();

    const mergedConfig: WidgetConfig = {
      apiKey: config.apiKey ?? scriptConfig.apiKey ?? '',
      mode: config.mode ?? scriptConfig.mode ?? 'floating',
      containerId: config.containerId ?? scriptConfig.containerId ?? 'efofx-widget',
    };

    let container: HTMLElement | null = null;

    if (mergedConfig.mode === 'floating') {
      // For floating mode: create a new host div appended to body
      const hostDiv = document.createElement('div');
      hostDiv.id = `efofx-host-${Date.now()}`;
      hostDiv.style.position = 'fixed';
      hostDiv.style.zIndex = '2147483647';
      hostDiv.style.top = '0';
      hostDiv.style.left = '0';
      hostDiv.style.width = '0';
      hostDiv.style.height = '0';
      hostDiv.style.overflow = 'visible';
      hostDiv.style.pointerEvents = 'none';
      document.body.appendChild(hostDiv);
      container = hostDiv;
    } else {
      // For inline mode: find contractor's container div
      container = document.getElementById(mergedConfig.containerId);
      if (!container) {
        // Fallback: create a div and append to body
        const fallbackDiv = document.createElement('div');
        fallbackDiv.id = mergedConfig.containerId;
        document.body.appendChild(fallbackDiv);
        container = fallbackDiv;
      }
    }

    const root = createRoot(container);

    root.render(
      <ErrorBoundary>
        <ShadowDOMWrapper>
          <App config={mergedConfig} />
        </ShadowDOMWrapper>
      </ErrorBoundary>,
    );

    return {
      destroy: () => {
        root.unmount();
        if (container && container.parentNode) {
          container.parentNode.removeChild(container);
        }
      },
    };
  } catch (e) {
    console.error('[efOfX widget] initialization failed', e);
    return { destroy: () => {} };
  }
}

// Auto-initialize in dev mode with demo tenant
if (import.meta.env.DEV) {
  init({
    apiKey: import.meta.env.VITE_DEMO_API_KEY || '',
    mode: 'floating',
  });
}

// IIFE export
export default { init };
