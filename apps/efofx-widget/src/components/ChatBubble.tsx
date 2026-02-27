import type { ChatMessage } from '../types/widget';

interface ChatBubbleProps {
  message: ChatMessage;
}

/**
 * ChatBubble — Single chat message bubble.
 *
 * User messages: right-aligned, var(--brand-primary) background, white text.
 * Assistant messages: left-aligned, var(--brand-secondary) background, dark text.
 * Content rendered as plain text (XSS prevention — no dangerouslySetInnerHTML).
 */
export function ChatBubble({ message }: ChatBubbleProps) {
  const isUser = message.role === 'user';

  return (
    <div className={isUser ? 'efofx-bubble-wrapper efofx-bubble-wrapper--user' : 'efofx-bubble-wrapper efofx-bubble-wrapper--assistant'}>
      <div className={isUser ? 'efofx-bubble efofx-bubble-user' : 'efofx-bubble efofx-bubble-assistant'}>
        {message.content}
      </div>
    </div>
  );
}

export default ChatBubble;
