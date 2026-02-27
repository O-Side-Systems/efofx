/**
 * ConsultationCTA — Disclaimer text and "Request Free Consultation" CTA button.
 *
 * Per locked decision from CONTEXT.md:
 * - Disclaimer is prominent, positioned below estimate card and above narrative.
 * - CTA button uses var(--brand-accent) background.
 * - No contractor email configured yet — button logs to console; actual link
 *   target will be configured via branding config in a future plan.
 */
export function ConsultationCTA() {
  function handleConsultationClick() {
    console.info('[efOfX widget] Request Free Consultation clicked');
    // TODO: Open contractor contact link when configured via branding API
  }

  return (
    <div className="efofx-cta-container">
      <p className="efofx-disclaimer">
        These estimates are unofficial ballpark figures based on similar projects.
        No figure is binding. For an official quote, request a free consultation.
      </p>
      <button
        className="efofx-cta-button"
        type="button"
        onClick={handleConsultationClick}
      >
        Request Free Consultation
      </button>
    </div>
  );
}

export default ConsultationCTA;
