import styles from './TypingIndicator.module.css';

/**
 * TypingIndicator — iMessage-style three-dot bouncing animation.
 *
 * Rendered as a left-aligned assistant bubble (var(--brand-secondary) background).
 * Three dots bounce sequentially using CSS @keyframes bounce.
 */
export function TypingIndicator() {
  return (
    <div className={styles.bubbleWrapper}>
      <div className={styles.typingIndicator} aria-label="Typing..." role="status">
        <span className={styles.typingDot} />
        <span className={styles.typingDot} />
        <span className={styles.typingDot} />
      </div>
    </div>
  );
}

export default TypingIndicator;
