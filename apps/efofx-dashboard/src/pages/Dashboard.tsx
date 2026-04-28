import { useState } from 'react'
import { useNavigate } from 'react-router'
import { useCalibration } from '../hooks/useCalibration'
import { LoadingSkeleton } from '@efofx/ui'
import ThresholdProgress from '../components/ThresholdProgress'
import CalibrationMetrics from '../components/CalibrationMetrics'
import AccuracyBucketBar from '../components/AccuracyBucketBar'
import AccuracyTrendLine from '../components/AccuracyTrendLine'
import ReferenceClassTable from '../components/ReferenceClassTable'
import DateRangeFilter from '../components/DateRangeFilter'
import { supabase } from '../lib/supabase'

export default function Dashboard() {
  const navigate = useNavigate()
  const [dateRange, setDateRange] = useState('all')
  const { data, isPending, isError, error, refetch } = useCalibration(dateRange)

  async function handleSignOut() {
    await supabase.auth.signOut()
    await navigate('/login')
  }

  return (
    <div className="dashboard">
      <header className="dashboard-header">
        <h1 className="dashboard-title">Calibration Dashboard</h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
          <DateRangeFilter value={dateRange} onChange={setDateRange} />
          <button type="button" onClick={handleSignOut} className="signout-button">
            Sign out
          </button>
        </div>
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
