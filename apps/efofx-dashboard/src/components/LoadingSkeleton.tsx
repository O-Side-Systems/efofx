export default function LoadingSkeleton() {
  return (
    <div className="loading-skeleton">
      {/* Stat cards */}
      <div className="stats-grid" style={{ marginBottom: '2rem' }}>
        <div className="skeleton" style={{ height: '100px' }} />
        <div className="skeleton" style={{ height: '100px' }} />
      </div>

      {/* Chart skeleton */}
      <div className="skeleton" style={{ height: '120px', marginBottom: '2rem' }} />

      {/* Trend chart skeleton */}
      <div className="skeleton" style={{ height: '320px', marginBottom: '2rem' }} />

      {/* Table skeleton */}
      <div>
        <div
          className="skeleton"
          style={{ height: '40px', marginBottom: '0.5rem', borderRadius: '4px' }}
        />
        {[1, 2, 3, 4].map((i) => (
          <div
            key={i}
            className="skeleton"
            style={{ height: '48px', marginBottom: '0.25rem', borderRadius: '4px' }}
          />
        ))}
      </div>
    </div>
  )
}
