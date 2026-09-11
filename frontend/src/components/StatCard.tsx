interface StatCardProps {
  label: string
  value: string
  hint?: string
  accent?: 'red' | 'blue' | 'green' | 'purple' | 'orange' | 'gray'
}

/** Yksi tunnusluku dashboardin yläriville. */
export function StatCard({ label, value, hint, accent = 'gray' }: StatCardProps) {
  return (
    <div className={`stat stat-${accent}`}>
      <div className="stat-label">{label}</div>
      <div className="stat-value">{value}</div>
      {hint ? <div className="stat-hint">{hint}</div> : null}
    </div>
  )
}
