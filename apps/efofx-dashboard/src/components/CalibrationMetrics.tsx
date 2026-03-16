import type { CalibrationMetrics as CalibrationMetricsType } from '../types/calibration'

interface CalibrationMetricsProps {
  metrics: CalibrationMetricsType
}

export default function CalibrationMetrics({ metrics }: CalibrationMetricsProps) {
  const meanVariance =
    metrics.mean_variance_pct !== undefined
      ? `\u00b1${Math.abs(metrics.mean_variance_pct).toFixed(1)}%`
      : 'N/A'

  return (
    <div className="card">
      <div className="stats-grid">
        <div className="stat-card">
          <p className="stat-value">{meanVariance}</p>
          <p className="stat-label">Average Estimate Variance</p>
        </div>
        <div className="stat-card">
          <p className="stat-value">{metrics.outcome_count}</p>
          <p className="stat-label">Completed Projects</p>
        </div>
      </div>
    </div>
  )
}
