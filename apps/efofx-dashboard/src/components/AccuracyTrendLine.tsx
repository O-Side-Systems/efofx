import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from 'recharts'
import { format, parse } from 'date-fns'
import { useCalibrationTrend } from '../hooks/useCalibrationTrend'

interface TooltipPayloadItem {
  value: number
  payload: {
    period: string
    mean_variance_pct: number
    outcome_count: number
  }
}

interface CustomTooltipProps {
  active?: boolean
  payload?: TooltipPayloadItem[]
  label?: string
}

function CustomTooltip({ active, payload }: CustomTooltipProps) {
  if (!active || !payload || payload.length === 0) return null
  const d = payload[0].payload
  return (
    <div className="recharts-custom-tooltip">
      <p style={{ fontSize: '0.8125rem', fontWeight: 600, marginBottom: '0.25rem' }}>
        {formatPeriod(d.period)}
      </p>
      <p style={{ fontSize: '0.8125rem', color: 'var(--color-text-muted)' }}>
        Variance: &plusmn;{d.mean_variance_pct.toFixed(1)}%
      </p>
      <p style={{ fontSize: '0.8125rem', color: 'var(--color-text-muted)' }}>
        Outcomes: {d.outcome_count}
      </p>
    </div>
  )
}

function formatPeriod(period: string): string {
  try {
    return format(parse(period, 'yyyy-MM', new Date()), "MMM ''yy")
  } catch {
    return period
  }
}

export default function AccuracyTrendLine() {
  const { data, isPending, isError } = useCalibrationTrend(12)

  if (isPending) {
    return (
      <div>
        <h2 className="section-heading">Accuracy Trend</h2>
        <div className="skeleton" style={{ height: '300px', marginTop: '1rem' }} />
      </div>
    )
  }

  if (isError || !data || data.below_threshold) {
    return null
  }

  if (!data.trend || data.trend.length < 2) {
    return (
      <div>
        <h2 className="section-heading">Accuracy Trend</h2>
        <p style={{ color: 'var(--color-text-muted)', fontSize: '0.9375rem', marginTop: '1rem' }}>
          More months of data needed to show trend
        </p>
      </div>
    )
  }

  return (
    <div>
      <h2 className="section-heading">Accuracy Trend</h2>
      <div style={{ marginTop: '1rem' }}>
        <ResponsiveContainer width="100%" height={300}>
          <LineChart data={data.trend}>
            <CartesianGrid strokeDasharray="3 3" stroke="var(--color-border)" />
            <XAxis
              dataKey="period"
              tickFormatter={formatPeriod}
              tick={{ fontSize: 12, fill: 'var(--color-text-muted)' }}
            />
            <YAxis
              unit="%"
              tick={{ fontSize: 12, fill: 'var(--color-text-muted)' }}
              tickFormatter={(v: number) => v.toFixed(1)}
            />
            <Tooltip content={<CustomTooltip />} />
            <Line
              type="monotone"
              dataKey="mean_variance_pct"
              stroke="var(--color-accent)"
              strokeWidth={2}
              dot={{ r: 4, fill: 'var(--color-accent)', strokeWidth: 0 }}
              activeDot={{ r: 5 }}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  )
}
