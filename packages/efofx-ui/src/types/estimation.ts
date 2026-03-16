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
