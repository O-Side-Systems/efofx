import { useWidget } from '../context/WidgetContext';

/**
 * ChatPanel — Main panel container
 *
 * Desktop: slide-up panel (efofx-panel class)
 * Mobile (<= 480px): full-screen takeover (efofx-panel-fullscreen class via CSS media query)
 * Inline mode: fills contractor's container div
 *
 * Header shows contractor branding (logo + company name).
 * No efOfX branding visible anywhere.
 * Body and input areas are placeholder containers filled by Plan 04-03.
 */
export function ChatPanel() {
  const { config, branding, phase, setPhase } = useWidget();

  // Floating mode: hide when idle (button is shown instead)
  if (config.mode === 'floating' && phase === 'idle') {
    return null;
  }

  function handleClose() {
    setPhase('idle');
  }

  // Determine CSS class based on mode
  const panelClass = config.mode === 'inline' ? 'efofx-inline-container' : 'efofx-panel';

  return (
    <div className={panelClass} role="dialog" aria-modal="true" aria-label="Estimation chat">
      {/* Header — contractor branding only, no efOfX branding */}
      <header className="efofx-header">
        {branding?.logo_url ? (
          <img
            src={branding.logo_url}
            alt={branding.company_name}
            className="efofx-header-logo"
          />
        ) : (
          <div className="efofx-header-logo-placeholder" aria-hidden="true">
            {branding?.company_name ? branding.company_name.charAt(0).toUpperCase() : '?'}
          </div>
        )}

        <p className="efofx-header-company">
          {branding?.company_name ?? 'Get an Estimate'}
        </p>

        {config.mode === 'floating' && (
          <button
            className="efofx-close-btn"
            aria-label="Close chat"
            onClick={handleClose}
            type="button"
          >
            <svg
              viewBox="0 0 24 24"
              xmlns="http://www.w3.org/2000/svg"
              aria-hidden="true"
              focusable="false"
            >
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        )}
      </header>

      {/* Message area — filled by Plan 04-03 */}
      <div className="efofx-messages" />

      {/* Input area — filled by Plan 04-03 */}
      <div className="efofx-input-area" />
    </div>
  );
}

export default ChatPanel;
