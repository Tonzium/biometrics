/**
 * Recharts-kaaviot. Kaikki ottavat backendin tyypit sellaisenaan ja
 * muuntavat ne itse; sivut eivät tee datamuunnoksia.
 */

import {
  Bar,
  BarChart,
  CartesianGrid,
  ComposedChart,
  Legend,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from 'recharts'

import type { CardioLoad, DailyActivity, DailyWellness, NightlyRecharge, SleepNight, WeeklySummary } from '../api/types'
import { formatDuration, formatShortDate, parseIsoDuration } from '../lib/format'

const COLORS = {
  red: '#d32f2f',
  blue: '#1e6fd9',
  lightBlue: '#7fb2f0',
  deepBlue: '#153c7a',
  purple: '#7b3fe4',
  green: '#2e9e5b',
  orange: '#e8842c',
  gray: '#8a8f98',
}

const hours = (s: number | null | undefined) => (s ? Math.round((s / 3600) * 100) / 100 : 0)

// ---------------------------------------------------------------------------

export function SleepStagesChart({ nights }: { nights: Array<SleepNight | DailyWellness> }) {
  const data = nights.map((n) => ({
    date: formatShortDate(n.date),
    Syvä: hours(n.deep_sleep_s),
    REM: hours(n.rem_sleep_s),
    Kevyt: hours(n.light_sleep_s),
    score: n.sleep_score ?? null,
  }))
  return (
    <ResponsiveContainer width="100%" height={260}>
      <ComposedChart data={data} margin={{ top: 8, right: 8, left: -16, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" vertical={false} />
        <XAxis dataKey="date" tick={{ fontSize: 12 }} />
        <YAxis yAxisId="h" unit=" h" tick={{ fontSize: 12 }} />
        <YAxis yAxisId="s" orientation="right" domain={[0, 100]} tick={{ fontSize: 12 }} />
        <Tooltip formatter={(v, name) => (name === 'Unipisteet' ? v : `${v} h`)} />
        <Legend />
        <Bar yAxisId="h" dataKey="Syvä" stackId="a" fill={COLORS.deepBlue} />
        <Bar yAxisId="h" dataKey="REM" stackId="a" fill={COLORS.purple} />
        <Bar yAxisId="h" dataKey="Kevyt" stackId="a" fill={COLORS.lightBlue} radius={[3, 3, 0, 0]} />
        <Line yAxisId="s" type="monotone" dataKey="score" name="Unipisteet" stroke={COLORS.red} strokeWidth={2} dot={false} connectNulls />
      </ComposedChart>
    </ResponsiveContainer>
  )
}

export function RechargeChart({ nights }: { nights: NightlyRecharge[] }) {
  const data = nights.map((n) => ({
    date: formatShortDate(n.date),
    'HRV (ms)': n.hrv_avg_ms ?? null,
    'Yösyke (bpm)': n.heart_rate_avg ?? null,
    'Hengitys (/min)': n.breathing_rate_avg ?? null,
  }))
  return (
    <ResponsiveContainer width="100%" height={240}>
      <LineChart data={data} margin={{ top: 8, right: 8, left: -16, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" vertical={false} />
        <XAxis dataKey="date" tick={{ fontSize: 12 }} />
        <YAxis tick={{ fontSize: 12 }} />
        <Tooltip />
        <Legend />
        <Line type="monotone" dataKey="HRV (ms)" stroke={COLORS.green} strokeWidth={2} dot={false} connectNulls />
        <Line type="monotone" dataKey="Yösyke (bpm)" stroke={COLORS.red} strokeWidth={2} dot={false} connectNulls />
        <Line type="monotone" dataKey="Hengitys (/min)" stroke={COLORS.gray} strokeWidth={1.5} dot={false} connectNulls />
      </LineChart>
    </ResponsiveContainer>
  )
}

export function StepsChart({ days }: { days: Array<DailyActivity | DailyWellness> }) {
  const data = days.map((d) => ({
    date: formatShortDate(d.date),
    Askeleet: d.steps ?? 0,
    'Aktiiviset kalorit': d.active_calories ?? null,
  }))
  return (
    <ResponsiveContainer width="100%" height={240}>
      <ComposedChart data={data} margin={{ top: 8, right: 8, left: -8, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" vertical={false} />
        <XAxis dataKey="date" tick={{ fontSize: 12 }} />
        <YAxis yAxisId="steps" tick={{ fontSize: 12 }} />
        <YAxis yAxisId="kcal" orientation="right" tick={{ fontSize: 12 }} />
        <Tooltip />
        <Legend />
        <Bar yAxisId="steps" dataKey="Askeleet" fill={COLORS.blue} radius={[3, 3, 0, 0]} />
        <Line yAxisId="kcal" type="monotone" dataKey="Aktiiviset kalorit" stroke={COLORS.orange} strokeWidth={2} dot={false} connectNulls />
      </ComposedChart>
    </ResponsiveContainer>
  )
}

export function CardioLoadChart({ days }: { days: CardioLoad[] }) {
  const data = days.map((d) => ({
    date: formatShortDate(d.date),
    Rasitus: d.strain ?? null,
    Sietokyky: d.tolerance ?? null,
    'Päivän kuorma': d.cardio_load ?? null,
  }))
  return (
    <ResponsiveContainer width="100%" height={240}>
      <ComposedChart data={data} margin={{ top: 8, right: 8, left: -16, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" vertical={false} />
        <XAxis dataKey="date" tick={{ fontSize: 12 }} />
        <YAxis tick={{ fontSize: 12 }} />
        <Tooltip />
        <Legend />
        <Bar dataKey="Päivän kuorma" fill={COLORS.lightBlue} radius={[3, 3, 0, 0]} />
        <Line type="monotone" dataKey="Rasitus" stroke={COLORS.red} strokeWidth={2} dot={false} connectNulls />
        <Line type="monotone" dataKey="Sietokyky" stroke={COLORS.green} strokeWidth={2} dot={false} connectNulls />
      </ComposedChart>
    </ResponsiveContainer>
  )
}

export function WeeklyTrainingChart({ weeks }: { weeks: WeeklySummary[] }) {
  const data = weeks.map((w) => ({
    week: `vk ${isoWeek(w.week_start)}`,
    'Harjoittelu (h)': hours(w.total_duration_s),
    'Harjoituksia': w.exercise_count,
    'Unipisteet ka.': w.avg_sleep_score === null || w.avg_sleep_score === undefined ? null : Math.round(w.avg_sleep_score),
  }))
  return (
    <ResponsiveContainer width="100%" height={260}>
      <ComposedChart data={data} margin={{ top: 8, right: 8, left: -16, bottom: 0 }}>
        <CartesianGrid strokeDasharray="3 3" vertical={false} />
        <XAxis dataKey="week" tick={{ fontSize: 12 }} />
        <YAxis yAxisId="h" unit=" h" tick={{ fontSize: 12 }} />
        <YAxis yAxisId="s" orientation="right" domain={[0, 100]} tick={{ fontSize: 12 }} />
        <Tooltip />
        <Legend />
        <Bar yAxisId="h" dataKey="Harjoittelu (h)" fill={COLORS.red} radius={[3, 3, 0, 0]} />
        <Line yAxisId="s" type="monotone" dataKey="Unipisteet ka." stroke={COLORS.blue} strokeWidth={2} dot connectNulls />
      </ComposedChart>
    </ResponsiveContainer>
  )
}

interface HrZone {
  index?: number
  'lower-limit'?: number
  'upper-limit'?: number
  'in-zone'?: string
}

/** Sykevyöhykkeet yhdestä harjoituksesta. `zones` on Polarin lista sellaisenaan. */
export function HrZonesChart({ zones }: { zones: unknown }) {
  if (!Array.isArray(zones) || zones.length === 0) return null
  const data = (zones as HrZone[]).map((z, i) => {
    const secs = parseIsoDuration(z['in-zone']) ?? 0
    return {
      zone: `Z${z.index ?? i + 1}`,
      range: z['lower-limit'] !== undefined && z['upper-limit'] !== undefined ? `${z['lower-limit']}–${z['upper-limit']} bpm` : '',
      minutes: Math.round(secs / 60),
      secs,
    }
  })
  const fills = [COLORS.gray, COLORS.lightBlue, COLORS.green, COLORS.orange, COLORS.red]
  return (
    <ResponsiveContainer width="100%" height={200}>
      <BarChart data={data} layout="vertical" margin={{ top: 4, right: 24, left: 8, bottom: 4 }}>
        <XAxis type="number" unit=" min" tick={{ fontSize: 12 }} />
        <YAxis type="category" dataKey="zone" width={36} tick={{ fontSize: 12 }} />
        <Tooltip
          formatter={(_v, _n, item) => formatDuration((item.payload as { secs: number }).secs)}
          labelFormatter={(label, payload) => {
            const p = payload?.[0]?.payload as { range?: string } | undefined
            return p?.range ? `${label} · ${p.range}` : String(label)
          }}
        />
        <Bar dataKey="minutes" name="Aika vyöhykkeellä" radius={[0, 3, 3, 0]} fill={COLORS.red}
          // Recharts sallii per-palkki-värit `Cell`-komponentilla; yksinkertaisempi tapa on
          // antaa väri datassa ja käyttää `fill`-funktiota shape-propissa. Käytetään tässä
          // yhtä väriä per vyöhykeindeksi.
          shape={(props: unknown) => {
            const { x, y, width, height, index } = props as { x: number; y: number; width: number; height: number; index: number }
            return <rect x={x} y={y} width={Math.max(width, 0)} height={height} rx={3} fill={fills[index % fills.length]} />
          }}
        />
      </BarChart>
    </ResponsiveContainer>
  )
}

/** ISO-viikon numero päivämäärästä (maanantai). */
function isoWeek(dateStr: string): number {
  const [y, m, d] = dateStr.split('-').map(Number)
  const date = new Date(Date.UTC(y!, m! - 1, d!))
  const day = date.getUTCDay() || 7
  date.setUTCDate(date.getUTCDate() + 4 - day)
  const yearStart = new Date(Date.UTC(date.getUTCFullYear(), 0, 1))
  return Math.ceil(((date.getTime() - yearStart.getTime()) / 86_400_000 + 1) / 7)
}
