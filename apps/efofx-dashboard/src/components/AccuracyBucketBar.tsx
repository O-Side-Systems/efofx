import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from 'recharts'
import type { AccuracyBuckets } from '../types/calibration'

interface AccuracyBucketBarProps {
  buckets: AccuracyBuckets
  height?: number
}

const BUCKET_CONFIG = [
  { key: 'within_10', label: 'Within 10%', color: '#22c55e' },
  { key: 'within_20', label: '10-20%', color: '#eab308' },
  { key: 'within_30', label: '20-30%', color: '#f97316' },
  { key: 'beyond_30', label: '>30%', color: '#ef4444' },
]

interface ChartData {
  name: string
  within_10: number
  within_20: number
  within_30: number
  beyond_30: number
  [key: string]: string | number
}

interface TooltipPayloadItem {
  name: string
  value: number
  color: string
}

interface CustomTooltipProps {
  active?: boolean
  payload?: TooltipPayloadItem[]
}

function CustomTooltip({ active, payload }: CustomTooltipProps) {
  if (!active || !payload || payload.length === 0) return null

  return (
    <div className="recharts-custom-tooltip">
      {payload.map((entry) => {
        const config = BUCKET_CONFIG.find((c) => c.key === entry.name)
        const label = config ? config.label : entry.name
        return (
          <p key={entry.name} style={{ color: entry.color, fontSize: '0.8125rem' }}>
            {label}: {(entry.value * 100).toFixed(1)}%
          </p>
        )
      })}
    </div>
  )
}

export default function AccuracyBucketBar({ buckets, height = 80 }: AccuracyBucketBarProps) {
  const data: ChartData[] = [
    {
      name: 'Accuracy',
      within_10: buckets.within_10_pct,
      within_20: buckets.within_20_pct,
      within_30: buckets.within_30_pct,
      beyond_30: buckets.beyond_30_pct,
    },
  ]

  return (
    <div>
      <ResponsiveContainer width="100%" height={height}>
        <BarChart data={data} layout="vertical">
          <XAxis type="number" domain={[0, 1]} hide />
          <YAxis type="category" dataKey="name" hide />
          <Tooltip content={<CustomTooltip />} />
          <Bar dataKey="within_10" stackId="a" fill="#22c55e">
            <Cell fill="#22c55e" />
          </Bar>
          <Bar dataKey="within_20" stackId="a" fill="#eab308">
            <Cell fill="#eab308" />
          </Bar>
          <Bar dataKey="within_30" stackId="a" fill="#f97316">
            <Cell fill="#f97316" />
          </Bar>
          <Bar dataKey="beyond_30" stackId="a" fill="#ef4444">
            <Cell fill="#ef4444" />
          </Bar>
        </BarChart>
      </ResponsiveContainer>
      <div className="bucket-legend">
        {BUCKET_CONFIG.map(({ key, label, color }) => (
          <span key={key} className="bucket-legend-item">
            <span className="bucket-legend-dot" style={{ background: color }} />
            {label}
          </span>
        ))}
      </div>
    </div>
  )
}
