/** Muotoiluapurit suomalaiseen esitystapaan. */

const dateFmt = new Intl.DateTimeFormat('fi-FI', { day: 'numeric', month: 'numeric', year: 'numeric' })
const shortDateFmt = new Intl.DateTimeFormat('fi-FI', { day: 'numeric', month: 'numeric' })
const dateTimeFmt = new Intl.DateTimeFormat('fi-FI', {
  day: 'numeric',
  month: 'numeric',
  year: 'numeric',
  hour: '2-digit',
  minute: '2-digit',
})
const weekdayFmt = new Intl.DateTimeFormat('fi-FI', { weekday: 'short', day: 'numeric', month: 'numeric' })
const numberFmt = new Intl.NumberFormat('fi-FI', { maximumFractionDigits: 0 })
const oneDecimalFmt = new Intl.NumberFormat('fi-FI', { minimumFractionDigits: 1, maximumFractionDigits: 1 })

/** `2026-09-10` tai ISO-aikaleima → `10.9.2026`. */
export function formatDate(value: string | null | undefined): string {
  if (!value) return '–'
  return dateFmt.format(parseDate(value))
}

/** `2026-09-10` → `10.9.` */
export function formatShortDate(value: string): string {
  return shortDateFmt.format(parseDate(value))
}

/** `2026-09-10` → `to 10.9.` */
export function formatWeekday(value: string): string {
  return weekdayFmt.format(parseDate(value))
}

export function formatDateTime(value: string | null | undefined): string {
  if (!value) return '–'
  return dateTimeFmt.format(new Date(value))
}

/** Paikallinen aika ilman vyöhykettä (kellon aika) → `10.9.2026 klo 17.02`. */
export function formatLocalDateTime(value: string): string {
  const [date, time] = value.split('T')
  if (!date || !time) return value
  return `${formatDate(date)} klo ${time.slice(0, 5).replace(':', '.')}`
}

/** Sekunnit → `1 h 12 min` / `45 min` / `30 s`. */
export function formatDuration(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) return '–'
  const s = Math.max(0, Math.round(seconds))
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  if (h > 0) return m > 0 ? `${h} h ${m} min` : `${h} h`
  if (m > 0) return `${m} min`
  return `${s} s`
}

/** Sekunnit → tunnit yhdellä desimaalilla (`7,4 h`). */
export function formatHours(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) return '–'
  return `${oneDecimalFmt.format(seconds / 3600)} h`
}

/** Metrit → `10,1 km` tai `850 m`. */
export function formatDistance(meters: number | null | undefined): string {
  if (meters === null || meters === undefined) return '–'
  if (meters >= 1000) return `${oneDecimalFmt.format(meters / 1000)} km`
  return `${numberFmt.format(meters)} m`
}

export function formatNumber(value: number | null | undefined, unit = ''): string {
  if (value === null || value === undefined) return '–'
  return unit ? `${numberFmt.format(value)} ${unit}` : numberFmt.format(value)
}

export function formatDecimal(value: number | null | undefined, unit = ''): string {
  if (value === null || value === undefined) return '–'
  return unit ? `${oneDecimalFmt.format(value)} ${unit}` : oneDecimalFmt.format(value)
}

/** ISO 8601 -kesto (`PT1H2M3S`) → sekunnit. Polarin sykevyöhykkeet käyttävät tätä. */
export function parseIsoDuration(value: string | null | undefined): number | null {
  if (!value) return null
  const match = /^P(?:(\d+(?:[.,]\d+)?)D)?(?:T(?:(\d+(?:[.,]\d+)?)H)?(?:(\d+(?:[.,]\d+)?)M)?(?:(\d+(?:[.,]\d+)?)S)?)?$/.exec(
    value.trim(),
  )
  if (!match) return null
  const num = (s: string | undefined) => (s ? Number(s.replace(',', '.')) : 0)
  const total = num(match[1]) * 86_400 + num(match[2]) * 3600 + num(match[3]) * 60 + num(match[4])
  return Number.isFinite(total) ? Math.floor(total) : null
}

const SPORT_LABELS: Record<string, string> = {
  RUNNING: 'Juoksu',
  TRAIL_RUNNING: 'Polkujuoksu',
  TREADMILL_RUNNING: 'Juoksumatto',
  CYCLING: 'Pyöräily',
  ROAD_CYCLING: 'Maantiepyöräily',
  MOUNTAIN_BIKING: 'Maastopyöräily',
  INDOOR_CYCLING: 'Sisäpyöräily',
  WALKING: 'Kävely',
  HIKING: 'Vaellus',
  SWIMMING: 'Uinti',
  POOL_SWIMMING: 'Allasuinti',
  OPEN_WATER_SWIMMING: 'Avovesiuinti',
  STRENGTH_TRAINING: 'Voimaharjoittelu',
  CIRCUIT_TRAINING: 'Kuntopiiri',
  FUNCTIONAL_TRAINING: 'Toiminnallinen harjoittelu',
  CROSS_TRAINER: 'Crosstrainer',
  ROWING: 'Soutu',
  INDOOR_ROWING: 'Sisäsoutu',
  CROSS_COUNTRY_SKIING: 'Hiihto',
  CLASSIC_XC_SKIING: 'Perinteinen hiihto',
  FREESTYLE_XC_SKIING: 'Vapaa hiihto',
  DOWNHILL_SKIING: 'Laskettelu',
  ICE_SKATING: 'Luistelu',
  ICE_HOCKEY: 'Jääkiekko',
  FLOORBALL: 'Salibandy',
  FOOTBALL: 'Jalkapallo',
  BADMINTON: 'Sulkapallo',
  TENNIS: 'Tennis',
  PADEL: 'Padel',
  GOLF: 'Golf',
  YOGA: 'Jooga',
  PILATES: 'Pilates',
  STRETCHING: 'Venyttely',
  MOBILITY_STATIC: 'Liikkuvuus',
  CORE: 'Keskivartalo',
  GROUP_EXERCISE: 'Ryhmäliikunta',
  DISC_GOLF: 'Frisbeegolf',
  OTHER: 'Muu',
  OTHER_INDOOR: 'Muu sisäliikunta',
  OTHER_OUTDOOR: 'Muu ulkoliikunta',
}

/** Polarin lajitunniste (`STRENGTH_TRAINING`) → suomenkielinen nimi. */
export function sportLabel(sport: string | null | undefined): string {
  if (!sport) return 'Tuntematon'
  const known = SPORT_LABELS[sport]
  if (known) return known
  const words = sport.toLowerCase().split('_')
  return words.map((w, i) => (i === 0 ? w.charAt(0).toUpperCase() + w.slice(1) : w)).join(' ')
}

const RECHARGE_LABELS: Record<number, string> = {
  1: 'Paljon alle tavallisen',
  2: 'Alle tavallisen',
  3: 'Heikentynyt',
  4: 'Tavallinen',
  5: 'Hyvä',
  6: 'Erittäin hyvä',
}

/** Nightly Recharge -status 1–6 → sanallinen arvio. */
export function rechargeLabel(status: number | null | undefined): string {
  if (status === null || status === undefined) return '–'
  return RECHARGE_LABELS[status] ?? `Tila ${status}`
}

const SLEEP_CHARGE_LABELS: Record<number, string> = {
  1: 'Paljon alle tavallisen',
  2: 'Alle tavallisen',
  3: 'Tavallinen',
  4: 'Yli tavallisen',
  5: 'Paljon yli tavallisen',
}

export function sleepChargeLabel(charge: number | null | undefined): string {
  if (charge === null || charge === undefined) return '–'
  return SLEEP_CHARGE_LABELS[charge] ?? `Taso ${charge}`
}

const CARDIO_LABELS: Record<string, string> = {
  LOAD_STATUS_NOT_AVAILABLE: 'Ei saatavilla',
  DETRAINING: 'Alikuormitus',
  MAINTAINING: 'Ylläpito',
  PRODUCTIVE: 'Tuottava',
  OVERREACHING: 'Ylikuormitus',
  RECOVERY_AFTER_OVERREACHING: 'Palautuminen ylikuormituksesta',
  PRODUCTIVE_DROPPED_FROM_OVERREACHING: 'Tuottava (laskee ylikuormituksesta)',
  PRODUCTIVE_ALMOST_OVERREACHING: 'Tuottava (lähellä ylikuormitusta)',
  UNRECOGNIZED: 'Tuntematon',
}

export function cardioStatusLabel(status: string | null | undefined): string {
  if (!status) return '–'
  return CARDIO_LABELS[status] ?? status
}

/** Sallii sekä `YYYY-MM-DD` että täydet ISO-aikaleimat ilman vyöhykeyllätyksiä. */
function parseDate(value: string): Date {
  if (/^\d{4}-\d{2}-\d{2}$/.test(value)) {
    const [y, m, d] = value.split('-').map(Number)
    return new Date(y!, m! - 1, d!)
  }
  return new Date(value)
}

/** Tämän päivän ja `days` päivää sitten olevan päivän `YYYY-MM-DD`-merkkijonot. */
export function lastDays(days: number): { from: string; to: string } {
  const to = new Date()
  const from = new Date()
  from.setDate(to.getDate() - (days - 1))
  return { from: toIsoDate(from), to: toIsoDate(to) }
}

export function toIsoDate(date: Date): string {
  const y = date.getFullYear()
  const m = String(date.getMonth() + 1).padStart(2, '0')
  const d = String(date.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}
