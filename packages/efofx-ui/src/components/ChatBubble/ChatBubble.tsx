import styles from './ChatBubble.module.css';

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
 */
export function ChatBubble({ message }: ChatBubbleProps) {
  const isUser = message.role === 'user';

  return (
    <div className={isUser ? `${styles.bubbleWrapper} ${styles.bubbleWrapperUser}` : `${styles.bubbleWrapper} ${styles.bubbleWrapperAssistant}`}>
      <div className={isUser ? `${styles.bubble} ${styles.bubbleUser}` : `${styles.bubble} ${styles.bubbleAssistant}`}>
        {message.content}
      </div>
    </div>
  );
}

export default ChatBubble;
