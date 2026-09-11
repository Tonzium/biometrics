import { Link, useSearchParams } from 'react-router'

import { useExercises, useOverview } from '../api/hooks'
import { Empty, ErrorBox, Loading } from '../components/Status'
import { formatDistance, formatDuration, formatLocalDateTime, formatNumber, sportLabel } from '../lib/format'

const PER_PAGE = 25

export function ExercisesPage() {
  const [params, setParams] = useSearchParams()
  const sport = params.get('sport') ?? ''
  const from = params.get('from') ?? ''
  const to = params.get('to') ?? ''
  const page = Math.max(1, Number(params.get('page') ?? '1') || 1)

  const overview = useOverview()
  const exercises = useExercises({ sport: sport || undefined, from: from || undefined, to: to || undefined, page, per_page: PER_PAGE })

  function update(next: Record<string, string>) {
    const merged = new URLSearchParams(params)
    for (const [k, v] of Object.entries(next)) {
      if (v) merged.set(k, v)
      else merged.delete(k)
    }
    if (!('page' in next)) merged.delete('page')
    setParams(merged)
  }

  const totalPages = exercises.data ? Math.max(1, Math.ceil(exercises.data.total / PER_PAGE)) : 1

  return (
    <section className="page">
      <div className="page-head">
        <h1>Harjoitukset</h1>
        {exercises.data ? <p className="muted">{formatNumber(exercises.data.total)} harjoitusta</p> : null}
      </div>

      <form className="filters" onSubmit={(e) => e.preventDefault()}>
        <label>
          Laji
          <select value={sport} onChange={(e) => update({ sport: e.target.value })}>
            <option value="">Kaikki lajit</option>
            {(overview.data?.sports ?? []).map((s) => (
              <option key={s} value={s}>
                {sportLabel(s)}
              </option>
            ))}
          </select>
        </label>
        <label>
          Alkaen
          <input type="date" value={from} max={to || undefined} onChange={(e) => update({ from: e.target.value })} />
        </label>
        <label>
          Päättyen
          <input type="date" value={to} min={from || undefined} onChange={(e) => update({ to: e.target.value })} />
        </label>
        {sport || from || to ? (
          <button type="button" className="btn btn-ghost" onClick={() => setParams(new URLSearchParams())}>
            Tyhjennä
          </button>
        ) : null}
      </form>

      <div className="card">
        {exercises.isPending ? (
          <Loading />
        ) : exercises.isError ? (
          <ErrorBox error={exercises.error} />
        ) : exercises.data.items.length === 0 ? (
          <Empty>Ei harjoituksia valituilla ehdoilla.</Empty>
        ) : (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Aika</th>
                  <th>Laji</th>
                  <th>Kesto</th>
                  <th>Matka</th>
                  <th>Syke ka. / max</th>
                  <th>Kcal</th>
                  <th>Kuorma</th>
                </tr>
              </thead>
              <tbody>
                {exercises.data.items.map((e) => (
                  <tr key={e.id}>
                    <td>
                      <Link to={`/exercises/${encodeURIComponent(e.id)}`}>{formatLocalDateTime(e.start_time_local)}</Link>
                    </td>
                    <td>{sportLabel(e.detailed_sport_info ?? e.sport)}</td>
                    <td>{formatDuration(e.duration_s)}</td>
                    <td>{formatDistance(e.distance_m)}</td>
                    <td>
                      {e.hr_avg ?? '–'} / {e.hr_max ?? '–'}
                    </td>
                    <td>{formatNumber(e.calories)}</td>
                    <td>{e.training_load === null || e.training_load === undefined ? '–' : Math.round(e.training_load)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {exercises.data && totalPages > 1 ? (
          <nav className="pagination" aria-label="Sivutus">
            <button type="button" className="btn btn-ghost" disabled={page <= 1} onClick={() => update({ page: String(page - 1) })}>
              ← Edellinen
            </button>
            <span className="muted">
              Sivu {page} / {totalPages}
            </span>
            <button type="button" className="btn btn-ghost" disabled={page >= totalPages} onClick={() => update({ page: String(page + 1) })}>
              Seuraava →
            </button>
          </nav>
        ) : null}
      </div>
    </section>
  )
}
