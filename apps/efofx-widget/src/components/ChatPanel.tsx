import { useEffect, useRef, useState } from 'react';
import DOMPurify from 'dompurify';
import { useWidget } from '../context/WidgetContext';
import { useChat } from '../hooks/useChat';
import { useEstimateStream } from '../hooks/useEstimateStream';
import { useBranding } from '../hooks/useBranding';
import { trackEvent } from '../api/chat';
import { ChatBubble } from './ChatBubble';
import { TypingIndicator } from './TypingIndicator';
import { LeadCaptureForm } from './LeadCaptureForm';
import { EstimateCard } from './EstimateCard';
import { NarrativeStream } from './NarrativeStream';
import { ConsultationCTA } from './ConsultationCTA';

/**
 * ChatPanel — Main orchestrator for the widget chat flow.
 *
 * Desktop: slide-up panel (efofx-panel class)
 * Mobile (<= 480px): full-screen takeover (via CSS @media)
 * Inline mode: fills contractor's container div
 *
 * Phase state machine:
 *   idle         -> chatting      (widget opened / inline auto-start)
 *   chatting     -> lead_capture  (is_ready=true from chat API)
 *   lead_capture -> generating    (lead form submitted)
 *   generating   -> result        (SSE stream complete)
 *   any          -> idle          (close button)
 *
 * No efOfX branding visible anywhere. Header shows contractor logo + company name.
 * Branding values sanitized via DOMPurify (WSEC-03, defense-in-depth).
 */
export function ChatPanel() {
  const { config, phase, setPhase } = useWidget();

  // All data hooks owned by ChatPanel for clear data flow
  const { branding } = useBranding(config.apiKey);
  const {
    sessionId,
    messages,
    isReady,
    isLoading,
    error: chatError,
    send,
  } = useChat(config.apiKey);
  const { estimateData, narrative, isStreaming, error: streamError, startStream } = useEstimateStream();

  const [inputValue, setInputValue] = useState('');
  const [streamStarted, setStreamStarted] = useState(false);

  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Inline mode: start chatting immediately on mount
  useEffect(() => {
    if (config.mode === 'inline' && phase === 'idle') {
      setPhase('chatting');
    }
  }, [config.mode, phase, setPhase]);

  // Track widget_view on panel open (floating mode)
  useEffect(() => {
    if (phase === 'chatting' && config.mode === 'floating') {
      trackEvent(config.apiKey, 'widget_view');
    }
  // Only fire on first open
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phase === 'chatting']);

  // Transition to lead_capture when chat API signals is_ready
  useEffect(() => {
    if (isReady && phase === 'chatting') {
      setPhase('lead_capture');
    }
  }, [isReady, phase, setPhase]);

  // Transition generating -> result when SSE stream finishes
  useEffect(() => {
    if (!isStreaming && estimateData && phase === 'generating') {
      setPhase('result');
    }
  }, [isStreaming, estimateData, phase, setPhase]);

  // Auto-scroll messages container to bottom on new messages or narrative tokens
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, narrative, isLoading, isStreaming]);

  // Floating mode: hide when idle (FloatingButton is shown instead)
  if (config.mode === 'floating' && phase === 'idle') {
    return null;
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

  function handleLeadSubmitted() {
    if (!sessionId) return;
    setPhase('generating');
    if (!streamStarted) {
      setStreamStarted(true);
      void startStream(sessionId, config.apiKey);
    }
  }

  // Error type detection — auth errors disable retry
  const isAuthError = chatError
    ? chatError.includes('401') || chatError.includes('403') || chatError.includes('Invalid API key')
    : false;

  const panelClass = config.mode === 'inline' ? 'efofx-inline-container' : 'efofx-panel';

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

      {/* Messages area */}
      <div className="efofx-messages">
        {/* Welcome message from branding config (shown before any messages) */}
        {safeWelcomeMessage && messages.length === 0 && (
          <div className="efofx-bubble-wrapper efofx-bubble-wrapper--assistant">
            <div className="efofx-bubble efofx-bubble-assistant">
              {safeWelcomeMessage}
            </div>
          </div>
        )}

        {/* Chat message bubbles */}
        {messages.map((msg, i) => (
          <ChatBubble key={i} message={msg} />
        ))}

        {/* Typing indicator while waiting for chat response */}
        {isLoading && <TypingIndicator />}

        {/* Typing indicator while SSE stream is initializing (before estimate data arrives) */}
        {phase === 'generating' && isStreaming && !estimateData && <TypingIndicator />}

        {/* Lead capture form — gates estimate display */}
        {phase === 'lead_capture' && sessionId && (
          <LeadCaptureForm
            apiKey={config.apiKey}
            sessionId={sessionId}
            branding={branding}
            onSubmitted={handleLeadSubmitted}
          />
        )}

        {/* Estimate card + disclaimer + narrative — shown during and after streaming */}
        {(phase === 'generating' || phase === 'result') && estimateData && (
          <>
            <EstimateCard estimate={estimateData} />
            <ConsultationCTA />
            <NarrativeStream narrative={narrative} />
          </>
        )}

        {/* Chat error display */}
        {chatError && !isLoading && (
          <div className="efofx-error" role="alert">
            {chatError}
            {!isAuthError && (
              <button
                className="efofx-retry-btn"
                type="button"
                onClick={() => void send(inputValue || 'retry')}
              >
                Try again
              </button>
            )}
          </div>
        )}

        {/* Stream error display */}
        {streamError && (
          <div className="efofx-error" role="alert">
            {streamError}
          </div>
        )}

        {/* Scroll anchor */}
        <div ref={messagesEndRef} />
      </div>

      {/* Input area — only shown during chatting phase */}
      {phase === 'chatting' && (
        <div className="efofx-input-area">
          <div className="efofx-input-row">
            <input
              className="efofx-chat-input"
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
      )}
    </div>
  );
}

export default ChatPanel;
