import { useState } from 'react'
import type { ReferenceClassBreakdown } from '../types/calibration'
import AccuracyBucketBar from './AccuracyBucketBar'

interface ReferenceClassTableProps {
  referenceClasses: ReferenceClassBreakdown[]
}

type SortKey = 'reference_class' | 'outcome_count' | 'mean_variance_pct'
type SortDirection = 'asc' | 'desc'

interface SortState {
  key: SortKey
  direction: SortDirection
}

export default function ReferenceClassTable({ referenceClasses }: ReferenceClassTableProps) {
  const [sort, setSort] = useState<SortState>({ key: 'outcome_count', direction: 'desc' })

  function handleSort(key: SortKey) {
    setSort((prev) => ({
      key,
      direction: prev.key === key && prev.direction === 'desc' ? 'asc' : 'desc',
    }))
  }

  const sorted = [...referenceClasses].sort((a, b) => {
    const { key, direction } = sort
    let aVal: string | number = a[key]
    let bVal: string | number = b[key]

    if (typeof aVal === 'string' && typeof bVal === 'string') {
      const cmp = aVal.localeCompare(bVal)
      return direction === 'asc' ? cmp : -cmp
    }

    const cmp = (aVal as number) - (bVal as number)
    return direction === 'asc' ? cmp : -cmp
  })

  function sortIndicator(key: SortKey) {
    if (sort.key !== key) return ' ↕'
    return sort.direction === 'asc' ? ' ↑' : ' ↓'
  }

  return (
    <div style={{ overflowX: 'auto' }}>
      <table className="table">
        <thead>
          <tr>
            <th data-sortable onClick={() => handleSort('reference_class')}>
              Reference Class{sortIndicator('reference_class')}
            </th>
            <th data-sortable onClick={() => handleSort('outcome_count')}>
              Count{sortIndicator('outcome_count')}
            </th>
            <th data-sortable onClick={() => handleSort('mean_variance_pct')}>
              Mean Variance{sortIndicator('mean_variance_pct')}
            </th>
            <th>Accuracy</th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((row) => (
            <tr key={row.reference_class} className={row.limited_data ? 'limited-data' : ''}>
              <td>
                {row.reference_class}
                {row.limited_data && (
                  <span
                    style={{
                      marginLeft: '0.5rem',
                      fontSize: '0.75rem',
                      color: 'var(--color-text-muted)',
                    }}
                  >
                    (Limited data)
                  </span>
                )}
              </td>
              <td>{row.outcome_count}</td>
              <td>&plusmn;{Math.abs(row.mean_variance_pct).toFixed(1)}%</td>
              <td style={{ minWidth: '120px' }}>
                <AccuracyBucketBar buckets={row.accuracy_buckets} height={28} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
