/**
 * Lyhyet nimet backendin OpenAPI-kuvauksesta generoiduille tyypeille.
 * Lähde: `src/api/schema.d.ts` (`npm run gen:api`).
 */

import type { components } from './schema'

type Schemas = components['schemas']

export type Meta = Schemas['Meta']
export type Health = Schemas['Health']
export type User = Schemas['User']
export type Role = Schemas['Role']
export type LoginRequest = Schemas['LoginRequest']
export type ErrorBody = Schemas['ErrorBody']

export type Exercise = Schemas['Exercise']
export type PagedExercises = Schemas['Paged_Exercise']
export type SleepNight = Schemas['SleepNight']
export type NightlyRecharge = Schemas['NightlyRecharge']
export type DailyActivity = Schemas['DailyActivity']
export type CardioLoad = Schemas['CardioLoad']
export type PhysicalInfo = Schemas['PhysicalInfo']

export type Overview = Schemas['Overview']
export type Latest = Schemas['Latest']
export type DailyWellness = Schemas['DailyWellness']
export type WeeklySummary = Schemas['WeeklySummary']

export type PolarStatus = Schemas['PolarStatus']
export type SyncReport = Schemas['SyncReport']
export type SyncCounts = Schemas['SyncCounts']
export type SyncRunRecord = Schemas['SyncRunRecord']

/** Päivämääräväli kyselyparametreina (YYYY-MM-DD). */
export interface DateRange {
  from?: string
  to?: string
}
