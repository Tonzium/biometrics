import { Link } from 'react-router'

import { useDaily, useExercises, useOverview, useWeekly } from '../api/hooks'
import { SleepStagesChart, WeeklyTrainingChart } from '../components/charts'
import { StatCard } from '../components/StatCard'
import { Empty, ErrorBox, Loading } from '../components/Status'
import {
  cardioStatusLabel,
  formatDate,
  formatDateTime,
  formatDistance,
  formatDuration,
  formatHours,
  formatLocalDateTime,
  formatNumber,
  lastDays,
  rechargeLabel,
  sportLabel,
} from '../lib/format'

export function DashboardPage() {
  const overview = useOverview()
  const weekly = useWeekly(12)
  const daily = useDaily(lastDays(14))
  const recent = useExercises({ per_page: 5 })

  if (overview.isPending) return <Loading />
  if (overview.isError) return <ErrorBox error={overview.error} />

  const o = overview.data
  const l = o.latest

  if (!o.polar_connected || (o.exercises === 0 && o.sleep_nights === 0)) {
    return (
      <section className="page">
        <h1>Yleiskuva</h1>
        <Empty>
          {o.polar_connected
            ? 'Polar-tili on yhdistetty, mutta dataa ei ole vielä synkronoitu.'
            : 'Polar-tiliä ei ole vielä yhdistetty. Omistaja voi tehdä sen asetuksista.'}
        </Empty>
      </section>
    )
  }

  return (
    <section className="page">
      <div className="page-head">
        <h1>Yleiskuva</h1>
        <p className="muted">
          Dataa {formatDate(o.first_date)} – {formatDate(o.last_date)} · {formatNumber(o.exercises)} harjoitusta,{' '}
          {formatNumber(o.sleep_nights)} yötä · Päivitetty {formatDateTime(o.last_sync_at)}
        </p>
      </div>

      <div className="stat-grid">
        <StatCard
          label="Unipisteet"
          value={l.sleep_score === null || l.sleep_score === undefined ? '–' : String(l.sleep_score)}
          hint={l.sleep_date ? `${formatDate(l.sleep_date)} · ${formatHours(l.sleep_total_s)}` : undefined}
          accent="blue"
        />
        <StatCard
          label="Palautuminen"
          value={rechargeLabel(l.nightly_recharge_status)}
          hint={l.recharge_date ? formatDate(l.recharge_date) : undefined}
          accent="green"
        />
        <StatCard
          label="Askeleet"
          value={formatNumber(l.steps)}
          hint={l.activity_date ? formatDate(l.activity_date) : undefined}
          accent="orange"
        />
        <StatCard label="Harjoituskuorma" value={cardioStatusLabel(l.cardio_load_status)} accent="red" />
        <StatCard label="Paino" value={l.weight_kg ? `${l.weight_kg.toLocaleString('fi-FI')} kg` : '–'} accent="purple" />
        <StatCard
          label="VO₂max / leposyke"
          value={`${l.vo2_max ?? '–'} / ${l.resting_heart_rate ?? '–'}`}
          hint="ml/kg/min · bpm"
          accent="gray"
        />
      </div>

      <div className="grid-2">
        <div className="card">
          <h2>Harjoittelu ja uni viikoittain</h2>
          {weekly.isPending ? <Loading /> : weekly.isError ? <ErrorBox error={weekly.error} /> : weekly.data.length === 0 ? <Empty>Ei viikkodataa.</Empty> : <WeeklyTrainingChart weeks={weekly.data} />}
        </div>
        <div className="card">
          <h2>Uni viimeiset 14 päivää</h2>
          {daily.isPending ? <Loading /> : daily.isError ? <ErrorBox error={daily.error} /> : daily.data.filter((d) => d.sleep_total_s).length === 0 ? <Empty>Ei unidataa.</Empty> : <SleepStagesChart nights={daily.data.filter((d) => d.sleep_total_s)} />}
        </div>
      </div>

      <div className="card">
        <div className="card-head">
          <h2>Viimeisimmät harjoitukset</h2>
          <Link to="/exercises" className="btn btn-ghost">
            Kaikki harjoitukset →
          </Link>
        </div>
        {recent.isPending ? (
          <Loading />
        ) : recent.isError ? (
          <ErrorBox error={recent.error} />
        ) : recent.data.items.length === 0 ? (
          <Empty>Ei harjoituksia.</Empty>
        ) : (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Aika</th>
                  <th>Laji</th>
                  <th>Kesto</th>
                  <th>Matka</th>
                  <th>Syke ka.</th>
                  <th>Kuorma</th>
                </tr>
              </thead>
              <tbody>
                {recent.data.items.map((e) => (
                  <tr key={e.id}>
                    <td>
                      <Link to={`/exercises/${encodeURIComponent(e.id)}`}>{formatLocalDateTime(e.start_time_local)}</Link>
                    </td>
                    <td>{sportLabel(e.detailed_sport_info ?? e.sport)}</td>
                    <td>{formatDuration(e.duration_s)}</td>
                    <td>{formatDistance(e.distance_m)}</td>
                    <td>{e.hr_avg ?? '–'}</td>
                    <td>{e.training_load === null || e.training_load === undefined ? '–' : Math.round(e.training_load)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </section>
  )
}
