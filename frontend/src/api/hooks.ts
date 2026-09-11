/**
 * TanStack Query -hookit. Jokainen vastaa yhtä backendin reittiä.
 * Avaimet sisältävät parametrit, joten eri aikavälit välimuistittuvat erikseen.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'

import { ApiError, api } from './client'
import type {
  CardioLoad,
  DailyActivity,
  DailyWellness,
  DateRange,
  Exercise,
  LoginRequest,
  Meta,
  NightlyRecharge,
  Overview,
  PagedExercises,
  PhysicalInfo,
  PolarStatus,
  SleepNight,
  SyncReport,
  SyncRunRecord,
  User,
  WeeklySummary,
} from './types'

// --- Järjestelmä ja istunto -------------------------------------------------

export function useMeta() {
  return useQuery({
    queryKey: ['meta'],
    queryFn: () => api.get<Meta>('/api/meta'),
    staleTime: Infinity,
  })
}

/** Kirjautunut käyttäjä tai `null`. 401 ei ole virhe vaan "ei kirjautunut". */
export function useMe() {
  return useQuery({
    queryKey: ['me'],
    queryFn: async (): Promise<User | null> => {
      try {
        return await api.get<User>('/api/auth/me')
      } catch (e) {
        if (e instanceof ApiError && e.status === 401) return null
        throw e
      }
    },
    staleTime: 5 * 60_000,
    retry: false,
  })
}

export function useLogin() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: LoginRequest) => api.post<User>('/api/auth/login', body),
    onSuccess: (user) => {
      qc.setQueryData(['me'], user)
      void qc.invalidateQueries({ queryKey: ['polar'] })
    },
  })
}

export function useLogout() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.post<{ ok: boolean }>('/api/auth/logout'),
    onSuccess: () => {
      qc.setQueryData(['me'], null)
      qc.removeQueries({ queryKey: ['polar'] })
      qc.removeQueries({ queryKey: ['sync'] })
    },
  })
}

// --- Yhteenvedot ------------------------------------------------------------

export function useOverview() {
  return useQuery({
    queryKey: ['summary', 'overview'],
    queryFn: () => api.get<Overview>('/api/summary/overview'),
  })
}

export function useWeekly(weeks: number) {
  return useQuery({
    queryKey: ['summary', 'weekly', weeks],
    queryFn: () => api.get<WeeklySummary[]>('/api/summary/weekly', { weeks }),
  })
}

export function useDaily(range: DateRange) {
  return useQuery({
    queryKey: ['summary', 'daily', range],
    queryFn: () => api.get<DailyWellness[]>('/api/summary/daily', range),
  })
}

// --- Data -------------------------------------------------------------------

export interface ExerciseFilter extends DateRange {
  sport?: string
  page?: number
  per_page?: number
}

export function useExercises(filter: ExerciseFilter) {
  return useQuery({
    queryKey: ['exercises', filter],
    queryFn: () => api.get<PagedExercises>('/api/exercises', filter),
    placeholderData: (previous) => previous,
  })
}

export function useExercise(id: string | undefined) {
  return useQuery({
    queryKey: ['exercises', 'detail', id],
    queryFn: () => api.get<Exercise>(`/api/exercises/${encodeURIComponent(id!)}`),
    enabled: Boolean(id),
  })
}

export function useSleep(range: DateRange) {
  return useQuery({
    queryKey: ['sleep', range],
    queryFn: () => api.get<SleepNight[]>('/api/sleep', range),
  })
}

export function useRecharge(range: DateRange) {
  return useQuery({
    queryKey: ['recharge', range],
    queryFn: () => api.get<NightlyRecharge[]>('/api/recharge', range),
  })
}

export function useActivity(range: DateRange) {
  return useQuery({
    queryKey: ['activity', range],
    queryFn: () => api.get<DailyActivity[]>('/api/activity', range),
  })
}

export function useCardioLoad(range: DateRange) {
  return useQuery({
    queryKey: ['cardio-load', range],
    queryFn: () => api.get<CardioLoad[]>('/api/cardio-load', range),
  })
}

export function usePhysical() {
  return useQuery({
    queryKey: ['physical'],
    queryFn: () => api.get<PhysicalInfo[]>('/api/physical'),
  })
}

// --- Omistajan toiminnot ----------------------------------------------------

export function usePolarStatus(enabled: boolean) {
  return useQuery({
    queryKey: ['polar', 'status'],
    queryFn: () => api.get<PolarStatus>('/api/polar/status'),
    enabled,
  })
}

export function useDisconnectPolar() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.delete<void>('/api/polar/disconnect'),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['polar'] })
      void qc.invalidateQueries({ queryKey: ['summary'] })
    },
  })
}

export function useSyncRuns(enabled: boolean) {
  return useQuery({
    queryKey: ['sync', 'runs'],
    queryFn: () => api.get<SyncRunRecord[]>('/api/sync/runs', { limit: 20 }),
    enabled,
  })
}

export function useSyncNow() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.post<SyncReport>('/api/sync'),
    onSettled: () => {
      // Synkronointi muuttaa käytännössä kaiken datan.
      void qc.invalidateQueries()
    },
  })
}
