import { useState } from 'react'

import { useRecharge, useSleep } from '../api/hooks'
import { RangePicker } from '../components/RangePicker'
import { RechargeChart, SleepStagesChart } from '../components/charts'
import { StatCard } from '../components/StatCard'
import { Empty, ErrorBox, Loading } from '../components/Status'
import { formatDuration, formatHours, formatWeekday, lastDays, rechargeLabel, sleepChargeLabel } from '../lib/format'

export function SleepPage() {
  const [days, setDays] = useState(30)
  const range = lastDays(days)
  const sleep = useSleep(range)
  const recharge = useRecharge(range)

  const nights = sleep.data ?? []
  const withScore = nights.filter((n) => n.sleep_score !== null && n.sleep_score !== undefined)
  const avgScore = withScore.length ? Math.round(withScore.reduce((s, n) => s + (n.sleep_score ?? 0), 0) / withScore.length) : null
  const avgTotal = nights.length
    ? nights.reduce((s, n) => s + (n.light_sleep_s ?? 0) + (n.deep_sleep_s ?? 0) + (n.rem_sleep_s ?? 0), 0) / nights.length
    : null
  const recharges = recharge.data ?? []
  const withHrv = recharges.filter((r) => r.hrv_avg_ms)
  const avgHrv = withHrv.length ? Math.round(withHrv.reduce((s, r) => s + (r.hrv_avg_ms ?? 0), 0) / withHrv.length) : null

  return (
    <section className="page">
      <div className="page-head">
        <h1>Uni ja palautuminen</h1>
        <RangePicker days={days} onChange={setDays} />
      </div>

      <div className="stat-grid">
        <StatCard label="Unipisteet ka." value={avgScore === null ? '–' : String(avgScore)} hint={`${withScore.length} yötä`} accent="blue" />
        <StatCard label="Unen kesto ka." value={formatHours(avgTotal)} accent="purple" />
        <StatCard label="HRV ka." value={avgHrv === null ? '–' : `${avgHrv} ms`} hint="yön keskiarvo" accent="green" />
        <StatCard
          label="Viimeisin palautuminen"
          value={rechargeLabel(recharges.at(-1)?.nightly_recharge_status)}
          hint={recharges.at(-1) ? formatWeekday(recharges.at(-1)!.date) : undefined}
          accent="red"
        />
      </div>

      <div className="card">
        <h2>Univaiheet ja unipisteet</h2>
        {sleep.isPending ? <Loading /> : sleep.isError ? <ErrorBox error={sleep.error} /> : nights.length === 0 ? <Empty>Ei unidataa valitulta ajalta.</Empty> : <SleepStagesChart nights={nights} />}
      </div>

      <div className="card">
        <h2>Yön syke, sykevälivaihtelu ja hengitys</h2>
        {recharge.isPending ? <Loading /> : recharge.isError ? <ErrorBox error={recharge.error} /> : recharges.length === 0 ? <Empty>Ei Nightly Recharge -dataa valitulta ajalta.</Empty> : <RechargeChart nights={recharges} />}
      </div>

      {nights.length > 0 ? (
        <div className="card">
          <h2>Yöt</h2>
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>Yö</th>
                  <th>Pisteet</th>
                  <th>Kesto</th>
                  <th>Syvä</th>
                  <th>REM</th>
                  <th>Kevyt</th>
                  <th>Hereillä</th>
                  <th>Vs. tavallinen</th>
                  <th>Palautuminen</th>
                </tr>
              </thead>
              <tbody>
                {[...nights].reverse().map((n) => {
                  const r = recharges.find((x) => x.date === n.date)
                  const total = (n.light_sleep_s ?? 0) + (n.deep_sleep_s ?? 0) + (n.rem_sleep_s ?? 0)
                  return (
                    <tr key={n.date}>
                      <td>{formatWeekday(n.date)}</td>
                      <td>
                        <strong>{n.sleep_score ?? '–'}</strong>
                      </td>
                      <td>{formatDuration(total)}</td>
                      <td>{formatDuration(n.deep_sleep_s)}</td>
                      <td>{formatDuration(n.rem_sleep_s)}</td>
                      <td>{formatDuration(n.light_sleep_s)}</td>
                      <td>{formatDuration(n.total_interruption_s)}</td>
                      <td>{sleepChargeLabel(n.sleep_charge)}</td>
                      <td>{rechargeLabel(r?.nightly_recharge_status)}</td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        </div>
      ) : null}
    </section>
  )
}
