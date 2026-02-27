import React, { useEffect, useRef } from 'react';
import { createRoot } from 'react-dom/client';
import type { Root } from 'react-dom/client';
import type { BrandingConfig } from '../types/widget';
import widgetStyles from '../widget.css?inline';

interface ShadowDOMWrapperProps {
  children: React.ReactNode;
  branding?: BrandingConfig | null;
}

/**
 * Shadow DOM Wrapper Component
 *
 * Provides style isolation for the widget by rendering children inside a Shadow DOM.
 * CSS is injected directly into the shadow root via ?inline import to prevent host
 * page CSS from leaking into the widget (and vice versa).
 *
 * Note: vite-plugin-css-injected-by-js injects into document.head which does NOT
 * reach Shadow DOM. The ?inline import is the only correct approach.
 */
export const ShadowDOMWrapper: React.FC<ShadowDOMWrapperProps> = ({ children, branding }) => {
  const hostRef = useRef<HTMLDivElement>(null);
  const shadowRootRef = useRef<ShadowRoot | null>(null);
  const reactRootRef = useRef<Root | null>(null);
  const brandingStyleRef = useRef<HTMLStyleElement | null>(null);

  useEffect(() => {
    if (!hostRef.current) return;

    if (!shadowRootRef.current) {
      const existingShadow = hostRef.current.shadowRoot;
      if (existingShadow) {
        shadowRootRef.current = existingShadow;
      } else {
        shadowRootRef.current = hostRef.current.attachShadow({ mode: 'open' });
      }

      // Inject widget CSS into shadow root (must happen before React renders)
      const styleEl = document.createElement('style');
      styleEl.textContent = widgetStyles;
      shadowRootRef.current.appendChild(styleEl);

      // Create container for React tree
      const container = document.createElement('div');
      container.id = 'efofx-widget-root';
      shadowRootRef.current.appendChild(container);

      reactRootRef.current = createRoot(container);
    }

    if (reactRootRef.current) {
      reactRootRef.current.render(children);
    }

    return () => {
      if (reactRootRef.current) {
        reactRootRef.current.unmount();
        reactRootRef.current = null;
        shadowRootRef.current = null;
      }
    };
  }, [children]);

  // Apply branding overrides via a separate <style> element
  useEffect(() => {
    if (!shadowRootRef.current) return;

    // Remove previous branding style if any
    if (brandingStyleRef.current && brandingStyleRef.current.parentNode) {
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
      shadowRootRef.current.appendChild(brandEl);
      brandingStyleRef.current = brandEl;
    }
  }, [branding]);

  return <div ref={hostRef} className="efofx-widget-host" />;
};

export default ShadowDOMWrapper;
