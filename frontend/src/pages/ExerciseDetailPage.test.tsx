import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { ExerciseDetailPage } from './ExerciseDetailPage'

function jsonResponse(body: unknown, status = 200) {
  return new Response(status === 204 ? null : JSON.stringify(body), {
    status,
    headers: { 'Content-Type': 'application/json' },
  })
}

const exercise = {
  id: 'R1',
  start_time: '2026-09-08T14:02:11Z',
  start_time_local: '2026-09-08T17:02:11',
  utc_offset_min: 180,
  upload_time: null,
  duration_s: 3133,
  sport: 'RUNNING',
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

function renderPage(role: 'owner' | 'viewer' | null) {
  const calls: { url: string; method: string }[] = []
  const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input)
    const method = init?.method ?? 'GET'
    calls.push({ url, method })
    if (url === '/api/auth/me') {
      return role
        ? jsonResponse({ id: 'u1', email: 'x@example.com', role })
        : jsonResponse({ error: { code: 'unauthorized', message: 'x' } }, 401)
    }
    if (url === '/api/exercises/R1' && method === 'DELETE') return jsonResponse(null, 204)
    if (url === '/api/exercises/R1') return jsonResponse(exercise)
    return jsonResponse({})
  })
  vi.stubGlobal('fetch', fetchMock)

  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/exercises/R1']}>
        <Routes>
          <Route path="/exercises/:id" element={<ExerciseDetailPage />} />
          <Route path="/exercises" element={<h1>Harjoitukset-lista</h1>} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
  return calls
}

afterEach(() => vi.unstubAllGlobals())

describe('ExerciseDetailPage', () => {
  it('hides the delete button from anonymous viewers', async () => {
    renderPage(null)
    expect(await screen.findByRole('heading', { name: 'Juoksu' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Poista harjoitus' })).not.toBeInTheDocument()
  })

  it('hides the delete button from viewers', async () => {
    renderPage('viewer')
    expect(await screen.findByRole('heading', { name: 'Juoksu' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Poista harjoitus' })).not.toBeInTheDocument()
  })

  it('lets the owner delete after confirmation and returns to the list', async () => {
    const calls = renderPage('owner')
    const button = await screen.findByRole('button', { name: 'Poista harjoitus' })
    const user = userEvent.setup()

    vi.stubGlobal('confirm', vi.fn(() => false))
    await user.click(button)
    expect(calls.some((c) => c.method === 'DELETE')).toBe(false)

    vi.stubGlobal('confirm', vi.fn(() => true))
    await user.click(button)
    expect(await screen.findByText('Harjoitukset-lista')).toBeInTheDocument()
    expect(calls.filter((c) => c.method === 'DELETE').map((c) => c.url)).toEqual(['/api/exercises/R1'])
  })
})
