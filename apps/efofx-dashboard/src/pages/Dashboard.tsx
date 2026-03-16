import { useState } from 'react'
import { useCalibration } from '../hooks/useCalibration'
import { LoadingSkeleton } from '@efofx/ui'
import ThresholdProgress from '../components/ThresholdProgress'
import CalibrationMetrics from '../components/CalibrationMetrics'
import AccuracyBucketBar from '../components/AccuracyBucketBar'
import AccuracyTrendLine from '../components/AccuracyTrendLine'
import ReferenceClassTable from '../components/ReferenceClassTable'
import DateRangeFilter from '../components/DateRangeFilter'

export default function Dashboard() {
  const [dateRange, setDateRange] = useState('all')
  const { data, isPending, isError, error, refetch } = useCalibration(dateRange)

  return (
    <div className="dashboard">
      <header className="dashboard-header">
        <h1 className="dashboard-title">Calibration Dashboard</h1>
        <DateRangeFilter value={dateRange} onChange={setDateRange} />
      </header>

      {isPending && <LoadingSkeleton />}

      {isError && (
        <div className="card error-card">
          <p className="error-message">
            {error instanceof Error ? error.message : 'Failed to load calibration data'}
          </p>
          <button className="retry-button" onClick={() => refetch()} type="button">
            Retry
          </button>
        </div>
      )}

      {data && data.below_threshold && (
        <ThresholdProgress
          outcomeCount={data.outcome_count}
          threshold={data.threshold}
        />
      )}

      {data && !data.below_threshold && (
        <>
          <CalibrationMetrics metrics={data} />

          <section className="section card">
            <h2 className="section-heading">Accuracy Distribution</h2>
            <div style={{ marginTop: '1rem' }}>
              <AccuracyBucketBar buckets={data.accuracy_buckets!} />
            </div>
          </section>

          <section className="section card">
            <AccuracyTrendLine />
          </section>

          <section className="section card">
            <h2 className="section-heading">Reference Class Breakdown</h2>
            <div style={{ marginTop: '1rem' }}>
              <ReferenceClassTable referenceClasses={data.by_reference_class ?? []} />
            </div>
          </section>
        </>
      )}
    </div>
  )
}
