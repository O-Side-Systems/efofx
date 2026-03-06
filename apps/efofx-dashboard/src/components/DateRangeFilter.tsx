interface DateRangeFilterProps {
  value: string
  onChange: (range: string) => void
}

const OPTIONS = [
  { value: '6months', label: '6 Months' },
  { value: '1year', label: '1 Year' },
  { value: 'all', label: 'All Time' },
]

export default function DateRangeFilter({ value, onChange }: DateRangeFilterProps) {
  return (
    <div className="date-range-filter">
      {OPTIONS.map((opt) => (
        <button
          key={opt.value}
          className={value === opt.value ? 'active' : ''}
          onClick={() => onChange(opt.value)}
          type="button"
        >
          {opt.label}
        </button>
      ))}
    </div>
  )
}
