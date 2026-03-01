import type { ConsultationFormLabels } from '../types/widget';

export const CONSULTATION_FORM_LABELS: Record<string, ConsultationFormLabels> = {
  en: {
    title: 'Request a Free Consultation',
    name: 'Your name',
    email: 'Email address',
    phone: 'Phone number',
    message: 'Tell us about your project',
    submit: 'Send Request',
    submitting: 'Sending...',
    success: "Thank you! We'll be in touch soon.",
  },
  es: {
    title: 'Solicitar consulta gratuita',
    name: 'Tu nombre',
    email: 'Correo electrónico',
    phone: 'Número de teléfono',
    message: 'Cuéntanos sobre tu proyecto',
    submit: 'Enviar solicitud',
    submitting: 'Enviando...',
    success: '¡Gracias! Nos pondremos en contacto pronto.',
  },
} as const;

export type SupportedLocale = keyof typeof CONSULTATION_FORM_LABELS;

export function getLabels(locale: string, overrides?: Partial<ConsultationFormLabels> | null): ConsultationFormLabels {
  const base = CONSULTATION_FORM_LABELS[locale] ?? CONSULTATION_FORM_LABELS['en'];
  return { ...base, ...(overrides ?? {}) };
}
