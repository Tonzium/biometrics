import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ExercisesPage } from './ExercisesPage'

function jsonResponse(body: unknown) {
  return new Response(JSON.stringify(body), { status: 200, headers: { 'Content-Type': 'application/json' } })
}

const overview = {
  polar_connected: true,
  last_sync_at: null,
  exercises: 2,
  sleep_nights: 0,
  activity_days: 0,
  first_date: null,
  last_date: null,
  sports: ['CYCLING', 'RUNNING'],
  latest: {},
}

function exercise(id: string, sport: string, duration_s: number) {
  return {
    id,
    start_time: '2026-09-08T14:02:11Z',
    start_time_local: '2026-09-08T17:02:11',
    utc_offset_min: 180,
    upload_time: null,
    duration_s,
    sport,
    detailed_sport_info: null,
    device: 'Polar Vantage V3',
    distance_m: 10123.4,
    calories: 610,
    hr_avg: 152,
    hr_max: 181,
    training_load: 143.2,
    has_route: true,
    running_index: 51,
    fat_percentage: null,
    carbohydrate_percentage: null,
    protein_percentage: null,
    heart_rate_zones: null,
    training_load_pro: null,
  }
}

afterEach(() => vi.unstubAllGlobals())

describe('ExercisesPage', () => {
  it('lists exercises and filters by sport via the URL', async () => {
    const calls: string[] = []
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input)
      calls.push(url)
      if (url.startsWith('/api/summary/overview')) return jsonResponse(overview)
      if (url.startsWith('/api/exercises')) {
        const sport = new URL(url, 'http://x').searchParams.get('sport')
        const items = sport === 'CYCLING' ? [exercise('C1', 'CYCLING', 7200)] : [exercise('R1', 'RUNNING', 3133), exercise('C1', 'CYCLING', 7200)]
        return jsonResponse({ items, page: 1, per_page: 25, total: items.length })
      }
      return jsonResponse({})
    })
    vi.stubGlobal('fetch', fetchMock)

    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
    render(
      <QueryClientProvider client={client}>
        <MemoryRouter initialEntries={['/exercises']}>
          <ExercisesPage />
        </MemoryRouter>
      </QueryClientProvider>,
    )

    const table = await screen.findByRole('table')
    expect(within(table).getByText('Juoksu')).toBeInTheDocument()
    expect(within(table).getByText('Pyöräily')).toBeInTheDocument()
    expect(within(table).getByText('52 min')).toBeInTheDocument()
    expect(within(table).getAllByText('10,1 km')).toHaveLength(2)
    expect(screen.getByText('2 harjoitusta')).toBeInTheDocument()

    const user = userEvent.setup()
    await user.selectOptions(screen.getByLabelText('Laji'), 'CYCLING')

    expect(await screen.findByText('1 harjoitusta')).toBeInTheDocument()
    expect(within(screen.getByRole('table')).queryByText('Juoksu')).not.toBeInTheDocument()
    expect(calls.some((u) => u.includes('sport=CYCLING'))).toBe(true)
  })
})
