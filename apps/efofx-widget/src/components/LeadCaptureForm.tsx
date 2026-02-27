import { useState } from 'react';
import { submitLead } from '../api/chat';
import type { BrandingConfig, LeadData } from '../types/widget';

interface LeadCaptureFormProps {
  apiKey: string;
  sessionId: string;
  branding: BrandingConfig | null;
  onSubmitted: () => void;
}

/**
 * LeadCaptureForm — Name / email / phone form.
 *
 * Gates the estimate display: appears after chat is complete (is_ready=true),
 * before estimate generation begins.
 *
 * All three fields are required per locked decision.
 * On submit: calls submitLead API, then triggers estimate generation via onSubmitted().
 */
export function LeadCaptureForm({ apiKey, sessionId, branding, onSubmitted }: LeadCaptureFormProps) {
  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [phone, setPhone] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  function validate(): string | null {
    if (!name.trim()) return 'Name is required.';
    if (!email.trim() || !email.includes('@')) return 'A valid email address is required.';
    if (!phone.trim() || phone.trim().length < 5) return 'A valid phone number is required.';
    return null;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const validationError = validate();
    if (validationError) {
      setError(validationError);
      return;
    }

    setError(null);
    setIsSubmitting(true);

    const lead: LeadData = { name: name.trim(), email: email.trim(), phone: phone.trim() };

    try {
      await submitLead(apiKey, sessionId, lead);
      onSubmitted();
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Failed to submit. Please try again.';
      setError(message);
      setIsSubmitting(false);
    }
  }

  const buttonText = branding?.button_text || 'Get My Estimate';

  return (
    <div className="efofx-bubble-wrapper efofx-bubble-wrapper--assistant">
      <form className="efofx-lead-form" onSubmit={handleSubmit} noValidate>
        <p className="efofx-lead-form-title">To receive your estimate, please share your contact info:</p>

        <input
          className="efofx-lead-input"
          type="text"
          placeholder="Your name"
          value={name}
          onChange={e => setName(e.target.value)}
          required
          aria-label="Your name"
          disabled={isSubmitting}
        />
        <input
          className="efofx-lead-input"
          type="email"
          placeholder="Email address"
          value={email}
          onChange={e => setEmail(e.target.value)}
          required
          aria-label="Email address"
          disabled={isSubmitting}
        />
        <input
          className="efofx-lead-input"
          type="tel"
          placeholder="Phone number"
          value={phone}
          onChange={e => setPhone(e.target.value)}
          required
          aria-label="Phone number"
          disabled={isSubmitting}
        />

        {error && (
          <p className="efofx-lead-error" role="alert">{error}</p>
        )}

        <button
          className="efofx-lead-submit"
          type="submit"
          disabled={isSubmitting}
        >
          {isSubmitting ? 'Submitting...' : buttonText}
        </button>
      </form>
    </div>
  );
}

export default LeadCaptureForm;
