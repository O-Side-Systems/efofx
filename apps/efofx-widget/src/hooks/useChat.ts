import { useState, useCallback } from 'react';
import DOMPurify from 'dompurify';
import { sendMessage, trackEvent } from '../api/chat';
import type { ChatMessage, ChatResponse } from '../types/widget';

/**
 * useChat — Chat state machine hook
 *
 * Sanitizes user input via DOMPurify before sending (WSEC-03).
 * Tracks 'chat_start' analytics event on first message (WFTR-04).
 * Fire-and-forget analytics calls never block user experience.
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
      const response: ChatResponse = await sendMessage(apiKey, sanitized, sessionId);

      if (!sessionId) {
        setSessionId(response.session_id);
        // WFTR-04: Track chat_start on first message — fire-and-forget
        trackEvent(apiKey, 'chat_start');
      }

      // Add assistant response (from trusted API, sanitize as defense-in-depth)
      const sanitizedContent = DOMPurify.sanitize(response.content, { ALLOWED_TAGS: [] });
      const assistantMsg: ChatMessage = {
        role: 'assistant',
        content: sanitizedContent,
        timestamp: response.timestamp,
      };
      setMessages((prev) => [...prev, assistantMsg]);

      if (response.is_ready) {
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
