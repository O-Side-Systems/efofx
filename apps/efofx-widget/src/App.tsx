import { useEffect, useRef } from 'react';
import { useBranding } from './hooks/useBranding';
import { WidgetProvider } from './context/WidgetContext';
import { FloatingButton } from './components/FloatingButton';
import { ChatPanel } from './components/ChatPanel';
import type { WidgetConfig } from './types/widget';

interface AppProps {
  config: WidgetConfig;
}

/**
 * App — Root widget component
 *
 * Fetches branding via useBranding and provides it to WidgetProvider (shared context).
 * Also applies CSS custom properties to the shadow root :host when branding loads,
 * so all brand colors are available throughout the shadow DOM immediately.
 *
 * App renders INSIDE the shadow root (via ShadowDOMWrapper's createRoot),
 * so it can traverse up to the shadow root's host element to inject branding overrides.
 */
function App({ config }: AppProps) {
  const { branding } = useBranding(config.apiKey);
  const brandingStyleRef = useRef<HTMLStyleElement | null>(null);
  const rootRef = useRef<HTMLDivElement>(null);

  // Apply branding CSS custom properties to the shadow root when branding loads
  useEffect(() => {
    if (!rootRef.current) return;

    // Find the shadow root that contains this element
    const shadowRoot = rootRef.current.getRootNode();
    if (!(shadowRoot instanceof ShadowRoot)) return;

    // Remove previous branding style if any
    if (brandingStyleRef.current?.parentNode) {
      brandingStyleRef.current.parentNode.removeChild(brandingStyleRef.current);
      brandingStyleRef.current = null;
    }

    if (branding) {
      const brandEl = document.createElement('style');
      brandEl.textContent = `
        :host {
          --brand-primary: ${branding.primary_color};
          --brand-secondary: ${branding.secondary_color};
          --brand-accent: ${branding.accent_color};
        }
      `;
      shadowRoot.appendChild(brandEl);
      brandingStyleRef.current = brandEl;
    }

    return () => {
      if (brandingStyleRef.current?.parentNode) {
        brandingStyleRef.current.parentNode.removeChild(brandingStyleRef.current);
        brandingStyleRef.current = null;
      }
    };
  }, [branding]);

  return (
    <div ref={rootRef} style={{ display: 'contents' }}>
      <WidgetProvider config={config} branding={branding}>
        {config.mode === 'floating' ? (
          <>
            <FloatingButton />
            <ChatPanel />
          </>
        ) : (
          <ChatPanel />
        )}
      </WidgetProvider>
    </div>
  );
}

export default App;
