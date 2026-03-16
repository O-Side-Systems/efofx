interface ThresholdProgressProps {
  outcomeCount: number
  threshold: number
}

export default function ThresholdProgress({ outcomeCount, threshold }: ThresholdProgressProps) {
  const remaining = threshold - outcomeCount
  const progressPct = Math.min((outcomeCount / threshold) * 100, 100)

  return (
    <div className="threshold-progress-wrapper">
      <div className="card threshold-progress-card">
        <p className="threshold-count">
          {outcomeCount} of {threshold} outcomes recorded
        </p>
        <div className="progress-bar-track" style={{ marginTop: '1rem', marginBottom: '1rem' }}>
          <div
            className="progress-bar-fill"
            style={{ width: `${progressPct}%` }}
            role="progressbar"
            aria-valuenow={outcomeCount}
            aria-valuemin={0}
            aria-valuemax={threshold}
          />
        </div>
        <p className="threshold-explanation">
          We need at least {threshold} completed projects to calculate meaningful accuracy metrics.{' '}
          {remaining > 0 ? `${remaining} more to go.` : 'Threshold reached!'}
        </p>
      </div>
    </div>
  )
}
