import { lastDays } from '../lib/format'

export interface RangeOption {
  days: number
  label: string
}

const DEFAULT_OPTIONS: RangeOption[] = [
  { days: 7, label: '7 pv' },
  { days: 30, label: '30 pv' },
  { days: 90, label: '90 pv' },
  { days: 365, label: 'Vuosi' },
]

interface RangePickerProps {
  days: number
  onChange: (days: number) => void
  options?: RangeOption[]
}

/** Pikavalinta viimeisille N päivälle. Käytä `lastDays(days)` kyselyyn. */
export function RangePicker({ days, onChange, options = DEFAULT_OPTIONS }: RangePickerProps) {
  const range = lastDays(days)
  return (
    <div className="range-picker" role="group" aria-label="Aikaväli">
      {options.map((o) => (
        <button
          key={o.days}
          type="button"
          className={o.days === days ? 'chip active' : 'chip'}
          onClick={() => onChange(o.days)}
          aria-pressed={o.days === days}
        >
          {o.label}
        </button>
      ))}
      <span className="muted range-label">
        {range.from} – {range.to}
      </span>
    </div>
  )
}
