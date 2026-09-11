import { Link, useParams } from 'react-router'

import { useExercise } from '../api/hooks'
import { HrZonesChart } from '../components/charts'
import { StatCard } from '../components/StatCard'
import { ErrorBox, Loading } from '../components/Status'
import { formatDateTime, formatDistance, formatDuration, formatLocalDateTime, formatNumber, sportLabel } from '../lib/format'

export function ExerciseDetailPage() {
  const { id } = useParams()
  const exercise = useExercise(id)

  if (exercise.isPending) return <Loading />
  if (exercise.isError) return <ErrorBox error={exercise.error} title="Harjoitusta ei löytynyt" />

  const e = exercise.data
  const pace =
    e.distance_m && e.distance_m > 0 && e.sport.includes('RUNNING')
      ? `${formatDuration(Math.round(e.duration_s / (e.distance_m / 1000)))} / km`
      : null
  const speed = e.distance_m && e.distance_m > 0 ? `${((e.distance_m / 1000) / (e.duration_s / 3600)).toLocaleString('fi-FI', { maximumFractionDigits: 1 })} km/h` : null

  return (
    <section className="page">
      <p>
        <Link to="/exercises" className="muted">
          ← Harjoitukset
        </Link>
      </p>
      <div className="page-head">
        <h1>{sportLabel(e.detailed_sport_info ?? e.sport)}</h1>
        <p className="muted">
          {formatLocalDateTime(e.start_time_local)}
          {e.device ? ` · ${e.device}` : ''}
          {e.upload_time ? ` · siirretty ${formatDateTime(e.upload_time)}` : ''}
        </p>
      </div>

      <div className="stat-grid">
        <StatCard label="Kesto" value={formatDuration(e.duration_s)} accent="red" />
        <StatCard label="Matka" value={formatDistance(e.distance_m)} hint={pace ?? speed ?? undefined} accent="blue" />
        <StatCard label="Syke ka. / max" value={`${e.hr_avg ?? '–'} / ${e.hr_max ?? '–'}`} hint="bpm" accent="orange" />
        <StatCard label="Energia" value={formatNumber(e.calories, 'kcal')} accent="green" />
        <StatCard label="Harjoituskuorma" value={e.training_load === null || e.training_load === undefined ? '–' : String(Math.round(e.training_load))} accent="purple" />
        {e.running_index ? <StatCard label="Running Index" value={String(e.running_index)} accent="gray" /> : null}
      </div>

      {Array.isArray(e.heart_rate_zones) && e.heart_rate_zones.length > 0 ? (
        <div className="card">
          <h2>Sykevyöhykkeet</h2>
          <HrZonesChart zones={e.heart_rate_zones} />
        </div>
      ) : null}

      {e.fat_percentage !== null && e.fat_percentage !== undefined ? (
        <div className="card">
          <h2>Energianlähteet</h2>
          <div className="energy-bar" aria-label="Energianlähteiden jakauma">
            <span style={{ width: `${e.fat_percentage}%` }} className="energy-fat" title={`Rasva ${e.fat_percentage} %`} />
            <span style={{ width: `${e.carbohydrate_percentage ?? 0}%` }} className="energy-carb" title={`Hiilihydraatit ${e.carbohydrate_percentage ?? 0} %`} />
            <span style={{ width: `${e.protein_percentage ?? 0}%` }} className="energy-protein" title={`Proteiini ${e.protein_percentage ?? 0} %`} />
          </div>
          <p className="muted">
            Rasva {e.fat_percentage} % · Hiilihydraatit {e.carbohydrate_percentage ?? 0} % · Proteiini {e.protein_percentage ?? 0} %
          </p>
        </div>
      ) : null}
    </section>
  )
}
