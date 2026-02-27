/**
 * NarrativeStream — Displays incrementally-streamed LLM narrative text.
 *
 * Rendered as a left-aligned assistant-style bubble.
 * React re-renders automatically as the `narrative` string grows via SSE token appending.
 * Text rendered as plain text (no HTML parsing — XSS safe).
 */
interface NarrativeStreamProps {
  narrative: string;
}

export function NarrativeStream({ narrative }: NarrativeStreamProps) {
  if (!narrative) return null;

  return (
    <div className="efofx-bubble-wrapper efofx-bubble-wrapper--assistant">
      <div className="efofx-bubble efofx-bubble-assistant efofx-narrative">
        {narrative}
      </div>
    </div>
  );
}

export default NarrativeStream;
