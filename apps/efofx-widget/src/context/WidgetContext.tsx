import React, { createContext, useContext, useState } from 'react';
import type { WidgetConfig, WidgetPhase, BrandingConfig, ChatMessage } from '../types/widget';

interface WidgetContextValue {
  config: WidgetConfig;
  branding: BrandingConfig | null;
  phase: WidgetPhase;
  setPhase: (phase: WidgetPhase) => void;
  sessionId: string | null;
  setSessionId: (id: string | null) => void;
  messages: ChatMessage[];
  addMessage: (msg: ChatMessage) => void;
}

const WidgetContext = createContext<WidgetContextValue | null>(null);

interface WidgetProviderProps {
  config: WidgetConfig;
  branding?: BrandingConfig | null;
  children: React.ReactNode;
}

export function WidgetProvider({ config, branding = null, children }: WidgetProviderProps) {
  const [phase, setPhase] = useState<WidgetPhase>('idle');
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);

  function addMessage(msg: ChatMessage) {
    setMessages((prev) => [...prev, msg]);
  }

  const value: WidgetContextValue = {
    config,
    branding: branding ?? null,
    phase,
    setPhase,
    sessionId,
    setSessionId,
    messages,
    addMessage,
  };

  return <WidgetContext.Provider value={value}>{children}</WidgetContext.Provider>;
}

export function useWidget(): WidgetContextValue {
  const ctx = useContext(WidgetContext);
  if (!ctx) {
    throw new Error('useWidget must be used inside a WidgetProvider');
  }
  return ctx;
}

export default WidgetContext;
