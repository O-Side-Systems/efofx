import styles from './LoadingSkeleton.module.css';

/**
 * LoadingSkeleton — Pulsing placeholder skeleton for the calibration dashboard.
 *
 * Renders skeleton shapes for: stat cards, accuracy chart, trend chart, and reference table.
 * Uses CSS custom property var(--color-border) for background so it adapts to host theming.
 */
export function LoadingSkeleton() {
  return (
    <div className={styles.loadingSkeleton}>
      {/* Stat cards */}
      <div className={styles.statsGrid} style={{ marginBottom: '2rem' }}>
        <div className={styles.skeleton} style={{ height: '100px' }} />
        <div className={styles.skeleton} style={{ height: '100px' }} />
      </div>

      {/* Chart skeleton */}
      <div className={styles.skeleton} style={{ height: '120px', marginBottom: '2rem' }} />

      {/* Trend chart skeleton */}
      <div className={styles.skeleton} style={{ height: '320px', marginBottom: '2rem' }} />

      {/* Table skeleton */}
      <div>
        <div
          className={styles.skeleton}
          style={{ height: '40px', marginBottom: '0.5rem', borderRadius: '4px' }}
        />
        {[1, 2, 3, 4].map((i) => (
          <div
            key={i}
            className={styles.skeleton}
            style={{ height: '48px', marginBottom: '0.25rem', borderRadius: '4px' }}
          />
        ))}
      </div>
    </div>
  );
}

export default LoadingSkeleton;
