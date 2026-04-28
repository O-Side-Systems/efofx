import { useState, useCallback } from 'react';
import DOMPurify from 'dompurify';
import { appendMessage, createChatSession, trackEvent } from '../api/chat';
import type { ChatMessage } from '../types/widget';

/**
 * useChat — Chat state machine hook
 *
 * Sanitizes user input via DOMPurify before sending (WSEC-03).
 * Tracks 'chat_start' analytics event on first message (WFTR-04).
 * Fire-and-forget analytics calls never block user experience.
 *
 * Two-call flow against the Rust core: first message creates the session
 * with `initial_message`; subsequent messages append to it.
 */
export function useChat(apiKey: string) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isReady, setIsReady] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const send = useCallback(async (text: string) => {
    if (!text.trim()) return;

    // WSEC-03: Sanitize user input — strip ALL HTML tags, keep only plain text
    const sanitized = DOMPurify.sanitize(text, { ALLOWED_TAGS: [] });
    if (!sanitized.trim()) return;

    setIsLoading(true);
    setError(null);

    // Add user message immediately (optimistic), using sanitized text
    const userMsg: ChatMessage = {
      role: 'user',
      content: sanitized,
      timestamp: new Date().toISOString(),
    };
    setMessages((prev) => [...prev, userMsg]);

    try {
      let assistantContent: string;
      let assistantTimestamp: string;
      let nextIsReady: boolean;

      if (!sessionId) {
        const session = await createChatSession(apiKey, sanitized);
        const last = session.messages[session.messages.length - 1];
        if (!last || last.role !== 'assistant') {
          throw new Error('Chat session returned without an assistant reply');
        }
        assistantContent = last.content;
        assistantTimestamp = last.timestamp;
        nextIsReady = session.is_ready;

        setSessionId(session.session_id);
        // WFTR-04: Track chat_start on first message — fire-and-forget
        trackEvent(apiKey, 'chat_start');
      } else {
        const response = await appendMessage(apiKey, sessionId, sanitized);
        assistantContent = response.assistant_message.content;
        assistantTimestamp = response.assistant_message.timestamp;
        nextIsReady = response.is_ready;
      }

      // Defense-in-depth: sanitize assistant content from the API
      const sanitizedContent = DOMPurify.sanitize(assistantContent, { ALLOWED_TAGS: [] });
      const assistantMsg: ChatMessage = {
        role: 'assistant',
        content: sanitizedContent,
        timestamp: assistantTimestamp,
      };
      setMessages((prev) => [...prev, assistantMsg]);

      if (nextIsReady) {
        setIsReady(true);
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : 'Failed to send message';
      setError(msg);
      // Remove optimistic user message on error
      setMessages((prev) => prev.slice(0, -1));
    } finally {
      setIsLoading(false);
    }
  }, [apiKey, sessionId]);

  return { sessionId, messages, isReady, isLoading, error, send, setMessages };
}
