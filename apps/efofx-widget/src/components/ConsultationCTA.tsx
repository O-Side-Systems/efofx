/**
 * ConsultationCTA — Disclaimer text and "Request Free Consultation" CTA button.
 *
 * DEBT-04: Button now opens an inline contact form (ConsultationForm) within
 * the chat panel. On form submission, the form is replaced by a success message.
 * The form submits to POST /widget/consultation on the backend.
 */
import { useState } from 'react';
import { useWidget } from '../context/WidgetContext';
import ConsultationForm from './ConsultationForm';
import { getLabels } from '../i18n/consultationForm';

export function ConsultationCTA() {
  const { config, sessionId, branding } = useWidget();
  const [showForm, setShowForm] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const locale = branding?.locale ?? 'en';
  const labels = getLabels(locale, branding?.consultation_form_labels);

  if (submitted) {
    return (
      <div className="efofx-cta-container">
        <p className="efofx-cta-success">{labels.success}</p>
      </div>
    );
  }

  if (showForm) {
    return (
      <ConsultationForm
        apiKey={config.apiKey}
        sessionId={sessionId ?? ''}
        branding={branding}
        onSubmitted={() => setSubmitted(true)}
      />
    );
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
        onClick={() => setShowForm(true)}
      >
        Request Free Consultation
      </button>
    </div>
  );
}

export default ConsultationCTA;
