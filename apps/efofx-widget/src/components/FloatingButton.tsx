import { useWidget } from '../context/WidgetContext';

/**
 * FloatingButton — Fixed-position circular trigger button
 *
 * Renders only in floating mode when the widget is idle.
 * Opens the chat panel by transitioning to 'chatting' phase.
 * Contains no efOfX branding — only a chat bubble icon.
 */
export function FloatingButton() {
  const { config, phase, setPhase } = useWidget();

  // Only render in floating mode when idle
  if (config.mode !== 'floating' || phase !== 'idle') {
    return null;
  }

  return (
    <button
      className="efofx-floating-btn"
      aria-label="Open estimation chat"
      onClick={() => setPhase('chatting')}
      type="button"
    >
      {/* Chat bubble SVG icon — no text branding */}
      <svg
        viewBox="0 0 24 24"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
        focusable="false"
      >
        <path
          d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"
          stroke="white"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          fill="none"
        />
      </svg>
    </button>
  );
}

export default FloatingButton;
