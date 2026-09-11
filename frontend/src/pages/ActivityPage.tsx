import { useState } from 'react'

import { useActivity, useCardioLoad, useMe, useMeta, usePhysical } from '../api/hooks'
import { RangePicker } from '../components/RangePicker'
import { CardioLoadChart, StepsChart } from '../components/charts'
import { StatCard } from '../components/StatCard'
import { Empty, ErrorBox, Loading } from '../components/Status'
import { cardioStatusLabel, formatDate, formatDistance, formatDuration, formatNumber, lastDays } from '../lib/format'

export function ActivityPage() {
  const [days, setDays] = useState(30)
  const range = lastDays(days)
  const activity = useActivity(range)
  const cardio = useCardioLoad(range)
  const physical = usePhysical()
  const me = useMe()
  const meta = useMeta()
  // Backend palauttaa painon ja pituuden null-arvoina kirjautumattomille; kerrotaan miksi.
  const bodyHidden = !me.data && meta.data?.public_body_metrics === false

  const acts = activity.data ?? []
  const totalSteps = acts.reduce((s, a) => s + (a.steps ?? 0), 0)
  const avgSteps = acts.length ? Math.round(totalSteps / acts.length) : null
  const totalActive = acts.reduce((s, a) => s + (a.active_duration_s ?? 0), 0)
  const totalDistance = acts.reduce((s, a) => s + (a.distance_from_steps_m ?? 0), 0)
  const latestCardio = (cardio.data ?? []).at(-1)
  const latestPhysical = (physical.data ?? []).at(-1)

  return (
    <section className="page">
      <div className="page-head">
        <h1>Aktiivisuus ja kuormitus</h1>
        <RangePicker days={days} onChange={setDays} />
      </div>

      <div className="stat-grid">
        <StatCard label="Askeleet / pv" value={formatNumber(avgSteps)} hint={`yhteensä ${formatNumber(totalSteps)}`} accent="blue" />
        <StatCard label="Aktiivista aikaa" value={formatDuration(totalActive)} hint={`${acts.length} päivää`} accent="orange" />
        <StatCard label="Matka askelista" value={formatDistance(totalDistance)} accent="green" />
        <StatCard
          label="Harjoituskuorma"
          value={cardioStatusLabel(latestCardio?.status)}
          hint={latestCardio ? `${formatDate(latestCardio.date)} · rasitus ${Math.round(latestCardio.strain ?? 0)} / sietokyky ${Math.round(latestCardio.tolerance ?? 0)}` : undefined}
          accent="red"
        />
      </div>

      <div className="card">
        <h2>Askeleet ja aktiiviset kalorit</h2>
        {activity.isPending ? <Loading /> : activity.isError ? <ErrorBox error={activity.error} /> : acts.length === 0 ? <Empty>Ei aktiivisuusdataa valitulta ajalta.</Empty> : <StepsChart days={acts} />}
      </div>

      <div className="card">
        <h2>Cardio load: rasitus ja sietokyky</h2>
        <p className="muted">
          Rasitus on viimeisen viikon keskimääräinen päiväkuorma, sietokyky viimeisen kuukauden. Kun rasitus ylittää sietokyvyn selvästi, tila on ylikuormitus.
        </p>
        {cardio.isPending ? <Loading /> : cardio.isError ? <ErrorBox error={cardio.error} /> : (cardio.data ?? []).length === 0 ? <Empty>Ei cardio load -dataa valitulta ajalta.</Empty> : <CardioLoadChart days={cardio.data ?? []} />}
      </div>

      <div className="card">
        <h2>Fyysiset tiedot</h2>
        {physical.isPending ? (
          <Loading />
        ) : physical.isError ? (
          <ErrorBox error={physical.error} />
        ) : !latestPhysical ? (
          <Empty>Ei fyysisiä tietoja.</Empty>
        ) : (
          <div className="stat-grid">
            <StatCard
              label="Paino"
              value={bodyHidden ? 'Piilotettu' : latestPhysical.weight_kg ? `${latestPhysical.weight_kg.toLocaleString('fi-FI')} kg` : '–'}
              hint={bodyHidden ? 'näkyy vain kirjautuneille' : formatDate(latestPhysical.date)}
              accent="purple"
            />
            <StatCard label="VO₂max" value={latestPhysical.vo2_max ? `${latestPhysical.vo2_max} ml/kg/min` : '–'} accent="blue" />
            <StatCard label="Leposyke" value={latestPhysical.resting_heart_rate ? `${latestPhysical.resting_heart_rate} bpm` : '–'} accent="red" />
            <StatCard label="Maksimisyke" value={latestPhysical.maximum_heart_rate ? `${latestPhysical.maximum_heart_rate} bpm` : '–'} accent="orange" />
            <StatCard
              label="Kynnykset"
              value={`${latestPhysical.aerobic_threshold ?? '–'} / ${latestPhysical.anaerobic_threshold ?? '–'}`}
              hint="aerobinen / anaerobinen, bpm"
              accent="green"
            />
            <StatCard label="Unitavoite" value={formatDuration(latestPhysical.sleep_goal_s)} accent="gray" />
          </div>
        )}
      </div>
    </section>
  )
}
