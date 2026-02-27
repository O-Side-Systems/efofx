import { useState, useCallback } from 'react';
import { sendMessage, trackEvent } from '../api/chat';
import type { ChatMessage, ChatResponse } from '../types/widget';

export function useChat(apiKey: string) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [isReady, setIsReady] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const send = useCallback(async (text: string) => {
    if (!text.trim()) return;
    setIsLoading(true);
    setError(null);

    // Add user message immediately (optimistic)
    const userMsg: ChatMessage = { role: 'user', content: text, timestamp: new Date().toISOString() };
    setMessages(prev => [...prev, userMsg]);

    try {
      const response: ChatResponse = await sendMessage(apiKey, text, sessionId);
      if (!sessionId) {
        setSessionId(response.session_id);
        // Track chat start on first message
        trackEvent(apiKey, 'chat_start');
      }

      // Add assistant response
      const assistantMsg: ChatMessage = { role: 'assistant', content: response.content, timestamp: response.timestamp };
      setMessages(prev => [...prev, assistantMsg]);

      if (response.is_ready) {
        setIsReady(true);
      }
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Failed to send message';
      setError(message);
      // Remove optimistic user message on error
      setMessages(prev => prev.slice(0, -1));
    } finally {
      setIsLoading(false);
    }
  }, [apiKey, sessionId]);

  return { sessionId, messages, isReady, isLoading, error, send, setMessages };
}
