/**
 * TypingIndicator — iMessage-style three-dot bouncing animation.
 *
 * Rendered as a left-aligned assistant bubble (var(--brand-secondary) background).
 * Three dots bounce sequentially using CSS @keyframes efofx-bounce (defined in widget.css).
 */
export function TypingIndicator() {
  return (
    <div className="efofx-bubble-wrapper efofx-bubble-wrapper--assistant">
      <div className="efofx-typing-indicator" aria-label="Typing..." role="status">
        <span className="efofx-typing-dot" />
        <span className="efofx-typing-dot" />
        <span className="efofx-typing-dot" />
      </div>
    </div>
  );
}

export default TypingIndicator;
