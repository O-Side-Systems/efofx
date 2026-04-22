export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
}

export interface ChatBubbleProps {
  message: ChatMessage;
}

/**
 * ChatBubble — Single chat message bubble.
 *
 * User messages: right-aligned, var(--brand-primary) background, white text.
 * Assistant messages: left-aligned, var(--brand-secondary) background, dark text.
 * Content rendered as plain text (XSS prevention — no dangerouslySetInnerHTML).
 *
 * Uses plain class names (efofx-*) instead of CSS Modules so styles work
 * inside the Shadow DOM where only widget.css is injected.
 */
export function ChatBubble({ message }: ChatBubbleProps) {
  const isUser = message.role === 'user';

  return (
    <div className={`efofx-bubble-wrapper ${isUser ? 'efofx-bubble-wrapper--user' : 'efofx-bubble-wrapper--assistant'}`}>
      <div className={`efofx-bubble ${isUser ? 'efofx-bubble-user' : 'efofx-bubble-assistant'}`}>
        {message.content}
      </div>
    </div>
  );
}

export default ChatBubble;
