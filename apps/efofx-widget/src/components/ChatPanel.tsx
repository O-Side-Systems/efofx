import { useState } from 'react';
import DOMPurify from 'dompurify';
import { useWidget } from '../context/WidgetContext';
import { useChat } from '../hooks/useChat';
import { ChatBubble } from './ChatBubble';
import { TypingIndicator } from './TypingIndicator';

/**
 * ChatPanel — Main panel container
 *
 * Desktop: slide-up panel (efofx-panel class)
 * Mobile (<= 480px): full-screen takeover via CSS @media
 * Inline mode: fills contractor's container div
 *
 * Header shows contractor branding (logo + company name) — sanitized via DOMPurify (WSEC-03).
 * Error states displayed inline for 401/403 (auth) and network errors.
 * WFTR-04: tracks 'widget_view' on first open, 'chat_start' on first message (delegated to useChat).
 */
export function ChatPanel() {
  const { config, branding, phase, setPhase } = useWidget();
  const { messages, isLoading, error: chatError, send } = useChat(config.apiKey);
  const [inputValue, setInputValue] = useState('');
  const [hasOpened, setHasOpened] = useState(false);

  // Floating mode: hide when idle (button is shown instead)
  if (config.mode === 'floating' && phase === 'idle') {
    return null;
  }

  // Track widget_view on first open (fire-and-forget via useChat's trackEvent)
  if (!hasOpened) {
    setHasOpened(true);
    // WFTR-04: widget_view tracked on panel open
    import('../api/chat').then(({ trackEvent }) => {
      trackEvent(config.apiKey, 'widget_view');
    });
  }

  function handleClose() {
    setPhase('idle');
  }

  async function handleSend() {
    const trimmed = inputValue.trim();
    if (!trimmed) return;
    setInputValue('');
    await send(trimmed);
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      void handleSend();
    }
  }

  // Sanitize branding values from API before rendering (WSEC-03, defense-in-depth)
  const safeCompanyName = branding?.company_name
    ? DOMPurify.sanitize(branding.company_name, { ALLOWED_TAGS: [] })
    : 'Get an Estimate';
  const safeLogoUrl = branding?.logo_url
    ? DOMPurify.sanitize(branding.logo_url, { ALLOWED_TAGS: [] })
    : null;
  const safeWelcomeMessage = branding?.welcome_message
    ? DOMPurify.sanitize(branding.welcome_message, { ALLOWED_TAGS: [] })
    : null;

  // Determine CSS class based on mode
  const panelClass = config.mode === 'inline' ? 'efofx-inline-container' : 'efofx-panel';

  // Determine error type for appropriate message
  const isAuthError = chatError
    ? chatError.includes('Invalid API key') || chatError.includes('not verified')
    : false;

  return (
    <div className={panelClass} role="dialog" aria-modal="true" aria-label="Estimation chat">
      {/* Header — contractor branding only, no efOfX branding */}
      <header className="efofx-header">
        {safeLogoUrl ? (
          <img
            src={safeLogoUrl}
            alt={safeCompanyName}
            className="efofx-header-logo"
          />
        ) : (
          <div className="efofx-header-logo-placeholder" aria-hidden="true">
            {safeCompanyName ? safeCompanyName.charAt(0).toUpperCase() : '?'}
          </div>
        )}

        <p className="efofx-header-company">
          {safeCompanyName}
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

      {/* Message area */}
      <div className="efofx-messages">
        {/* Welcome message from branding config */}
        {safeWelcomeMessage && messages.length === 0 && (
          <div className="efofx-bubble-wrapper efofx-bubble-wrapper--assistant">
            <div className="efofx-bubble efofx-bubble-assistant">
              {safeWelcomeMessage}
            </div>
          </div>
        )}

        {/* Chat messages */}
        {messages.map((msg, i) => (
          <ChatBubble key={i} message={msg} />
        ))}

        {/* Typing indicator while waiting for response */}
        {isLoading && <TypingIndicator />}

        {/* Error state — auth errors have no retry */}
        {chatError && !isLoading && (
          <div className="efofx-error" role="alert">
            {chatError}
            {!isAuthError && (
              <div>
                <button
                  className="efofx-retry-btn"
                  type="button"
                  onClick={() => void send(inputValue || 'retry')}
                >
                  Try again
                </button>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Input area */}
      <div className="efofx-input-area">
        <div className="efofx-input-row">
          <input
            className="efofx-input"
            type="text"
            placeholder="Describe your project..."
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={isLoading || isAuthError}
            aria-label="Type your message"
          />
          <button
            className="efofx-send-btn"
            type="button"
            onClick={() => void handleSend()}
            disabled={isLoading || !inputValue.trim() || isAuthError}
            aria-label="Send message"
          >
            <svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
              <line x1="22" y1="2" x2="11" y2="13" />
              <polygon points="22 2 15 22 11 13 2 9 22 2" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  );
}

export default ChatPanel;
