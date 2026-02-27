import { useState, useCallback } from 'react';
import type { EstimationOutput } from '../types/widget';

const API_BASE = import.meta.env.VITE_API_URL || 'https://api.efofx.ai';

export function useEstimateStream() {
  const [estimateData, setEstimateData] = useState<EstimationOutput | null>(null);
  const [narrative, setNarrative] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const startStream = useCallback(async (sessionId: string, apiKey: string) => {
    setIsStreaming(true);
    setError(null);
    setNarrative('');
    setEstimateData(null);

    try {
      const response = await fetch(
        `${API_BASE}/api/v1/chat/${sessionId}/generate-estimate`,
        { method: 'POST', headers: { Authorization: `Bearer ${apiKey}` } }
      );
      if (!response.ok || !response.body) {
        throw new Error(`Stream failed: ${response.status}`);
      }

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        buffer += decoder.decode(value, { stream: true });

        // SSE frames separated by \n\n
        const frames = buffer.split('\n\n');
        buffer = frames.pop() ?? '';

        for (const frame of frames) {
          if (!frame.trim()) continue;
          const lines = frame.split('\n');
          const eventLine = lines.find(l => l.startsWith('event:'));
          const dataLine = lines.find(l => l.startsWith('data:'));
          const eventType = eventLine?.slice(7).trim();
          const data = dataLine?.slice(5).trim();

          if (eventType === 'thinking') { /* Phase is already 'generating' */ }
          if (eventType === 'estimate' && data) {
            setEstimateData(JSON.parse(data) as EstimationOutput);
          }
          if (!eventType && data) {
            // Plain data line = narrative token
            setNarrative(prev => prev + data.replace(/\\n/g, '\n'));
          }
          if (eventType === 'done') {
            setIsStreaming(false);
          }
          if (eventType === 'error' && data) {
            const errData = JSON.parse(data) as { message?: string };
            setError(errData.message || 'Estimate generation failed');
            setIsStreaming(false);
          }
        }
      }
    } catch (e: unknown) {
      const message = e instanceof Error ? e.message : 'Stream connection failed';
      setError(message);
      setIsStreaming(false);
    }
  }, []);

  return { estimateData, narrative, isStreaming, error, startStream };
}
