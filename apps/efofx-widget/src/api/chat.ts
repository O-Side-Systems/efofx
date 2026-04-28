import { apiClient } from './client';
import type {
  AppendMessageResponse,
  ChatSession,
  ConsultationFormData,
  LeadData,
} from '../types/widget';

export async function createChatSession(
  apiKey: string,
  initialMessage?: string,
): Promise<ChatSession> {
  const body: Record<string, string> = {};
  if (initialMessage) body.initial_message = initialMessage;
  const res = await apiClient('/chat/sessions', apiKey, {
    method: 'POST',
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`Chat session create failed: ${res.status}`);
  return res.json();
}

export async function appendMessage(
  apiKey: string,
  sessionId: string,
  message: string,
): Promise<AppendMessageResponse> {
  const res = await apiClient(`/chat/sessions/${sessionId}/messages`, apiKey, {
    method: 'POST',
    body: JSON.stringify({ message }),
  });
  if (!res.ok) throw new Error(`Chat append failed: ${res.status}`);
  return res.json();
}

export async function submitLead(apiKey: string, sessionId: string, lead: LeadData): Promise<void> {
  const res = await apiClient('/widget/leads', apiKey, {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, ...lead }),
  });
  if (!res.ok) throw new Error(`Lead capture failed: ${res.status}`);
}

export async function trackEvent(apiKey: string, eventType: string): Promise<void> {
  // Fire-and-forget — don't await or throw on failure
  apiClient('/widget/events', apiKey, {
    method: 'POST',
    body: JSON.stringify({ event_type: eventType }),
  }).catch(() => {}); // Silently ignore analytics errors
}

export async function submitConsultation(
  apiKey: string,
  sessionId: string,
  data: ConsultationFormData,
): Promise<void> {
  const res = await apiClient('/widget/consultations', apiKey, {
    method: 'POST',
    body: JSON.stringify({ session_id: sessionId, ...data }),
  });
  if (!res.ok) throw new Error(`Consultation submit failed: ${res.status}`);
}
