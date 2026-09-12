import { describe, expect, it } from 'vitest'

import {
  cardioStatusLabel,
  formatDistance,
  formatDuration,
  formatHours,
  isoWeek,
  lastDays,
  parseIsoDuration,
  rechargeLabel,
  sportLabel,
} from './format'

describe('formatDuration', () => {
  it('formats hours, minutes and seconds', () => {
    expect(formatDuration(3723)).toBe('1 h 2 min')
    expect(formatDuration(3600)).toBe('1 h')
    expect(formatDuration(2700)).toBe('45 min')
    expect(formatDuration(30)).toBe('30 s')
    expect(formatDuration(null)).toBe('–')
  })
})

describe('formatHours and formatDistance', () => {
  it('uses Finnish decimal separator', () => {
    expect(formatHours(26_640)).toBe('7,4 h')
    expect(formatDistance(10_123.4)).toBe('10,1 km')
    expect(formatDistance(850)).toBe('850 m')
    expect(formatDistance(undefined)).toBe('–')
  })
})

describe('parseIsoDuration', () => {
  it('parses Polar durations', () => {
    expect(parseIsoDuration('PT2H44M45S')).toBe(2 * 3600 + 44 * 60 + 45)
    expect(parseIsoDuration('PT30M')).toBe(1800)
    expect(parseIsoDuration('PT5.5S')).toBe(5)
    expect(parseIsoDuration('P1DT1H')).toBe(90_000)
    expect(parseIsoDuration('nonsense')).toBeNull()
    expect(parseIsoDuration(null)).toBeNull()
  })
})

describe('labels', () => {
  it('translates known sports and humanises unknown ones', () => {
    expect(sportLabel('RUNNING')).toBe('Juoksu')
    expect(sportLabel('STRENGTH_TRAINING')).toBe('Voimaharjoittelu')
    expect(sportLabel('WATERSPORTS_WATERSKI')).toBe('Watersports waterski')
    expect(sportLabel(null)).toBe('Tuntematon')
  })

  it('maps recharge and cardio statuses', () => {
    expect(rechargeLabel(4)).toBe('Tavallinen')
    expect(rechargeLabel(null)).toBe('–')
    expect(cardioStatusLabel('PRODUCTIVE')).toBe('Tuottava')
    expect(cardioStatusLabel('SOMETHING_NEW')).toBe('SOMETHING_NEW')
  })
})

describe('isoWeek', () => {
  it('numbers weeks from their Monday', () => {
    expect(isoWeek('2026-09-07')).toBe(37)
    expect(isoWeek('2026-01-01')).toBe(1)
    // 2026 alkaa torstaina, joten edellinen maanantai kuuluu vielä viikkoon 1.
    expect(isoWeek('2025-12-29')).toBe(1)
  })
})

describe('lastDays', () => {
  it('returns an inclusive range ending today', () => {
    const { from, to } = lastDays(7)
    expect(to).toMatch(/^\d{4}-\d{2}-\d{2}$/)
    const diff = (new Date(to).getTime() - new Date(from).getTime()) / 86_400_000
    expect(Math.round(diff)).toBe(6)
  })
})
