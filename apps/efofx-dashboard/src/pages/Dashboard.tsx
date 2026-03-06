import { useCalibration } from '../hooks/useCalibration'

export default function Dashboard() {
  const { data, isPending, isError, error } = useCalibration()

  if (isPending) {
    return (
      <div style={styles.container}>
        <p style={styles.status}>Loading calibration data…</p>
      </div>
    )
  }

  if (isError) {
    return (
      <div style={styles.container}>
        <p style={styles.errorText}>
          Failed to load calibration data:{' '}
          {error instanceof Error ? error.message : 'Unknown error'}
        </p>
      </div>
    )
  }

  return (
    <div style={styles.container}>
      <h1 style={styles.heading}>Calibration Dashboard</h1>
      <pre style={styles.pre}>{JSON.stringify(data, null, 2)}</pre>
    </div>
  )
}

const styles = {
  container: {
    maxWidth: '960px',
    margin: '0 auto',
    padding: '2rem 1.5rem',
  },
  heading: {
    fontSize: '1.5rem',
    fontWeight: 600,
    color: 'var(--color-text)',
    marginBottom: '1.5rem',
  },
  status: {
    color: 'var(--color-text-muted)',
    fontSize: '0.9375rem',
  },
  errorText: {
    color: 'var(--color-red)',
    fontSize: '0.9375rem',
  },
  pre: {
    background: 'var(--color-surface)',
    border: '1px solid var(--color-border)',
    borderRadius: '8px',
    padding: '1.25rem',
    fontSize: '0.8125rem',
    color: 'var(--color-text)',
    overflowX: 'auto' as const,
    lineHeight: 1.6,
  },
} as const
