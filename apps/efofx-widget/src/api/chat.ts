import { apiClient } from './client';
import type { ChatResponse, ConsultationFormData, LeadData } from '../types/widget';

export async function sendMessage(apiKey: string, message: string, sessionId: string | null): Promise<ChatResponse> {
  const body: Record<string, string> = { message };
  if (sessionId) body.session_id = sessionId;
  const res = await apiClient('/chat/send', apiKey, { method: 'POST', body: JSON.stringify(body) });
  if (!res.ok) throw new Error(`Chat send failed: ${res.status}`);
  return res.json();
}

export async function submitLead(apiKey: string, sessionId: string, lead: LeadData): Promise<void> {
  const res = await apiClient('/widget/lead', apiKey, {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, ...lead }),
  });
  if (!res.ok) throw new Error(`Lead capture failed: ${res.status}`);
}

export async function trackEvent(apiKey: string, eventType: string): Promise<void> {
  // Fire-and-forget — don't await or throw on failure
  apiClient('/widget/analytics', apiKey, {
    method: 'POST',
    body: JSON.stringify({ event_type: eventType }),
  }).catch(() => {}); // Silently ignore analytics errors
}

export async function submitConsultation(
  apiKey: string,
  sessionId: string,
  data: ConsultationFormData,
): Promise<void> {
  const res = await apiClient('/widget/consultation', apiKey, {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, ...data }),
  });
  if (!res.ok) throw new Error(`Consultation submit failed: ${res.status}`);
}
