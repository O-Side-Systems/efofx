export type WidgetMode = 'floating' | 'inline';

export type WidgetPhase =
  | 'idle'           // Widget closed (floating button visible)
  | 'chatting'       // Chat panel open, conversation in progress
  | 'ready'          // Chat complete, enough info gathered
  | 'lead_capture'   // Lead form displayed
  | 'generating'     // Typing indicator, SSE stream in progress
  | 'result';        // Estimate card + narrative shown

export interface WidgetConfig {
  apiKey: string;
  mode: WidgetMode;
  containerId: string;
}

export interface ConsultationFormLabels {
  title: string;
  name: string;
  email: string;
  phone: string;
  message: string;
  submit: string;
  submitting: string;
  success: string;
}

export interface ConsultationFormData {
  name: string;
  email: string;
  phone: string;
  message: string;
}

export interface BrandingConfig {
  primary_color: string;
  secondary_color: string;
  accent_color: string;
  logo_url: string | null;
  welcome_message: string;
  button_text: string;
  company_name: string;
  locale?: string;
  consultation_form_labels?: Partial<ConsultationFormLabels>;
}

export interface LeadData {
  name: string;
  email: string;
  phone: string;
}

export interface CostCategoryEstimate {
  category: string;
  p50_cost: number;
  p80_cost: number;
  percentage_of_total: number;
}

export interface AdjustmentFactor {
  name: string;
  multiplier: number;
  reason: string;
}

export interface EstimationOutput {
  total_cost_p50: number;
  total_cost_p80: number;
  timeline_weeks_p50: number;
  timeline_weeks_p80: number;
  cost_breakdown: CostCategoryEstimate[];
  adjustment_factors: AdjustmentFactor[];
  confidence_score: number;
  assumptions: string[];
  summary: string;
}

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
}

export interface ChatResponse {
  session_id: string;
  content: string;
  timestamp: string;
  is_ready: boolean;
  scoping_context: Record<string, string | null> | null;
  status: string;
}
