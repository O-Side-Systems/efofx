export interface AccuracyBuckets {
  within_10_pct: number
  within_20_pct: number
  within_30_pct: number
  beyond_30_pct: number
}

export interface ReferenceClassBreakdown {
  reference_class: string
  outcome_count: number
  mean_variance_pct: number
  accuracy_buckets: AccuracyBuckets
  limited_data: boolean
}

export interface CalibrationMetrics {
  below_threshold: boolean
  outcome_count: number
  threshold: number
  mean_variance_pct?: number
  accuracy_buckets?: AccuracyBuckets
  by_reference_class?: ReferenceClassBreakdown[]
  date_range?: string
}

export interface CalibrationTrendPoint {
  period: string // "YYYY-MM" format
  mean_variance_pct: number
  outcome_count: number
}

export interface CalibrationTrendResponse {
  below_threshold: boolean
  outcome_count: number
  threshold: number
  trend: CalibrationTrendPoint[]
  months: number
}
