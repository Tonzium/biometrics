import { useSearchParams } from 'react-router'

import { ApiError } from '../api/client'
import { useDisconnectPolar, useMe, useMeta, usePolarStatus, useSyncNow, useSyncRuns } from '../api/hooks'
import { ErrorBox, Loading, Notice } from '../components/Status'
import type { SyncCounts } from '../api/types'
import { formatDateTime } from '../lib/format'

const COUNT_LABELS: Array<[keyof SyncCounts, string]> = [
  ['exercises', 'harjoitusta'],
  ['sleep_nights', 'yötä'],
  ['nightly_recharge', 'palautumista'],
  ['daily_activity', 'aktiivisuuspäivää'],
  ['physical_info', 'fyysistä tietoa'],
  ['cardio_load', 'cardio load -päivää'],
]

function countsText(counts: Partial<SyncCounts> | null | undefined): string {
  if (!counts) return ''
  const parts = COUNT_LABELS.filter(([k]) => (counts[k] ?? 0) > 0).map(([k, label]) => `${counts[k]} ${label}`)
  if (counts.skipped) parts.push(`${counts.skipped} ohitettu`)
  return parts.join(', ') || 'ei uutta dataa'
}

export function SettingsPage() {
  const me = useMe()
  const meta = useMeta()
  const isOwner = me.data?.role === 'owner'
  const status = usePolarStatus(Boolean(me.data))
  const runs = useSyncRuns(Boolean(me.data))
  const syncNow = useSyncNow()
  const disconnect = useDisconnectPolar()
  const [params, setParams] = useSearchParams()
  const polarResult = params.get('polar')

  const flash =
    polarResult === 'connected'
      ? { kind: 'success' as const, text: 'Polar-tili yhdistettiin. Voit nyt synkronoida datan.' }
      : polarResult === 'denied'
        ? { kind: 'warning' as const, text: 'Yhdistäminen peruttiin Polar Flow:ssa.' }
        : polarResult === 'error'
          ? { kind: 'warning' as const, text: 'Yhdistäminen epäonnistui. Tarkista palvelimen loki.' }
          : null

  function dismissFlash() {
    const next = new URLSearchParams(params)
    next.delete('polar')
    setParams(next, { replace: true })
  }

  return (
    <section className="page page-narrow">
      <h1>Asetukset</h1>
      <p className="muted">
        Kirjautuneena: {me.data?.email} ({me.data?.role === 'owner' ? 'omistaja' : 'katselija'}) ·{' '}
        {meta.data?.public_read ? 'data on julkisesti luettavissa' : 'data vaatii kirjautumisen'}
      </p>

      {flash ? (
        <Notice kind={flash.kind}>
          {flash.text}{' '}
          <button type="button" className="btn btn-ghost btn-small" onClick={dismissFlash}>
            Sulje
          </button>
        </Notice>
      ) : null}

      <div className="card">
        <h2>Polar-tili</h2>
        {status.isPending ? (
          <Loading />
        ) : status.isError ? (
          <ErrorBox error={status.error} />
        ) : !status.data.configured ? (
          <Notice kind="warning">
            Palvelimelle ei ole asetettu Polar AccessLink -tunnuksia (POLAR_CLIENT_ID / POLAR_CLIENT_SECRET). Yhdistäminen ei ole mahdollista.
          </Notice>
        ) : status.data.connected ? (
          <>
            <dl className="kv">
              <dt>Tila</dt>
              <dd>Yhdistetty</dd>
              <dt>Polar-käyttäjä</dt>
              <dd>{status.data.polar_user_id}</dd>
              <dt>Yhdistetty</dt>
              <dd>{formatDateTime(status.data.registered_at)}</dd>
              <dt>Viimeisin synkronointi</dt>
              <dd>{formatDateTime(status.data.last_sync_at)}</dd>
            </dl>
            {isOwner ? (
              <div className="actions">
                <button type="button" className="btn btn-primary" onClick={() => syncNow.mutate()} disabled={syncNow.isPending}>
                  {syncNow.isPending ? 'Synkronoidaan…' : 'Synkronoi nyt'}
                </button>
                <button
                  type="button"
                  className="btn btn-danger"
                  disabled={disconnect.isPending}
                  onClick={() => {
                    if (window.confirm('Irrotetaanko Polar-tili? Jo synkronoitu data säilyy kannassa.')) disconnect.mutate()
                  }}
                >
                  Irrota Polar-tili
                </button>
              </div>
            ) : null}
            {syncNow.isSuccess ? (
              <Notice kind={syncNow.data.status === 'ok' ? 'success' : 'warning'}>
                Synkronointi {syncNow.data.status === 'ok' ? 'onnistui' : syncNow.data.status === 'partial' ? 'onnistui osittain' : 'epäonnistui'}:{' '}
                {countsText(syncNow.data.counts)}
                {syncNow.data.errors.length ? ` · ${syncNow.data.errors.join('; ')}` : ''}
              </Notice>
            ) : null}
            {syncNow.isError ? (
              <Notice kind="warning">
                {syncNow.error instanceof ApiError && syncNow.error.status === 409 ? 'Synkronointi on jo käynnissä.' : `Synkronointi epäonnistui: ${syncNow.error.message}`}
              </Notice>
            ) : null}
            {disconnect.isError ? <ErrorBox error={disconnect.error} title="Irrotus epäonnistui" /> : null}
          </>
        ) : (
          <>
            <p>Polar-tiliä ei ole yhdistetty. Yhdistäminen ohjaa sinut Polar Flow:n valtuutussivulle ja takaisin.</p>
            {isOwner ? (
              // Täysi navigointi (ei fetch): backend vastaa 303-ohjauksella Polariin.
              <a className="btn btn-primary" href="/api/polar/connect">
                Yhdistä Polar-tili
              </a>
            ) : (
              <p className="muted">Vain omistaja voi yhdistää tilin.</p>
            )}
          </>
        )}
      </div>

      <div className="card">
        <h2>Synkronointihistoria</h2>
        {runs.isPending ? (
          <Loading />
        ) : runs.isError ? (
          <ErrorBox error={runs.error} />
        ) : runs.data.length === 0 ? (
          <p className="muted">Ei vielä ajoja.</p>
        ) : (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Alkoi</th>
                  <th>Käynnistys</th>
                  <th>Tila</th>
                  <th>Tulos</th>
                </tr>
              </thead>
              <tbody>
                {runs.data.map((r) => (
                  <tr key={r.id}>
                    <td>{formatDateTime(r.started_at)}</td>
                    <td>{r.trigger === 'manual' ? 'manuaalinen' : r.trigger === 'scheduled' ? 'ajastettu' : r.trigger}</td>
                    <td>
                      <span className={`badge badge-${r.status}`}>{r.status}</span>
                    </td>
                    <td>
                      {countsText(r.counts as Partial<SyncCounts>)}
                      {r.error ? <div className="muted small">{r.error}</div> : null}
                    </td>
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
