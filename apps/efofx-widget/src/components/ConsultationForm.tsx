import { useState } from 'react';
import type { BrandingConfig, ConsultationFormData } from '../types/widget';
import { submitConsultation } from '../api/chat';
import { getLabels } from '../i18n/consultationForm';

interface ConsultationFormProps {
  apiKey: string;
  sessionId: string;
  branding: BrandingConfig | null;
  onSubmitted: () => void;
}

/**
 * ConsultationForm — Inline contact form shown inside the chat panel (DEBT-04).
 *
 * Mirrors LeadCaptureForm structure: controlled inputs, validation,
 * isSubmitting state, and error display.
 *
 * Labels support en and es locales with per-tenant overrides from branding config.
 */
export function ConsultationForm({ apiKey, sessionId, branding, onSubmitted }: ConsultationFormProps) {
  const [formData, setFormData] = useState<ConsultationFormData>({
    name: '',
    email: '',
    phone: '',
    message: '',
  });
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const locale = branding?.locale ?? 'en';
  const labels = getLabels(locale, branding?.consultation_form_labels);

  function handleChange(e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) {
    setFormData(prev => ({ ...prev, [e.target.name]: e.target.value }));
  }

  function validate(): string | null {
    if (!formData.name.trim()) return 'Name is required.';
    if (!formData.email.trim() || !formData.email.includes('@')) return 'A valid email address is required.';
    if (!formData.phone.trim() || formData.phone.trim().length < 5) return 'A valid phone number is required.';
    return null;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    const validationError = validate();
    if (validationError) {
      setError(validationError);
      return;
    }

    setIsSubmitting(true);
    try {
      await submitConsultation(apiKey, sessionId, formData);
      onSubmitted();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to submit. Please try again.');
    } finally {
      setIsSubmitting(false);
    }
  }

  return (
    <div className="efofx-bubble-wrapper efofx-bubble-wrapper--assistant">
      <form className="efofx-lead-form" onSubmit={handleSubmit} noValidate>
        <p className="efofx-consultation-form-title">{labels.title}</p>
        <input
          className="efofx-lead-input"
          type="text"
          name="name"
          placeholder={labels.name}
          value={formData.name}
          onChange={handleChange}
          required
          aria-label={labels.name}
          disabled={isSubmitting}
        />
        <input
          className="efofx-lead-input"
          type="email"
          name="email"
          placeholder={labels.email}
          value={formData.email}
          onChange={handleChange}
          required
          aria-label={labels.email}
          disabled={isSubmitting}
        />
        <input
          className="efofx-lead-input"
          type="tel"
          name="phone"
          placeholder={labels.phone}
          value={formData.phone}
          onChange={handleChange}
          required
          aria-label={labels.phone}
          disabled={isSubmitting}
        />
        <textarea
          className="efofx-lead-input efofx-consultation-textarea"
          name="message"
          placeholder={labels.message}
          value={formData.message}
          onChange={handleChange}
          aria-label={labels.message}
          disabled={isSubmitting}
          rows={4}
        />
        {error && (
          <p className="efofx-lead-error" role="alert">{error}</p>
        )}
        <button
          className="efofx-lead-submit"
          type="submit"
          disabled={isSubmitting}
        >
          {isSubmitting ? labels.submitting : labels.submit}
        </button>
      </form>
    </div>
  );
}

export default ConsultationForm;
