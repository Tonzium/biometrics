import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, within } from '@testing-library/react'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { Layout } from './Layout'

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })
}

function renderLayout(path: string) {
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input)
    if (url === '/api/auth/me') return jsonResponse(401, { error: { code: 'unauthorized', message: 'x' } })
    if (url === '/api/meta') {
      return jsonResponse(200, { version: '0.1.0', public_read: true, public_body_metrics: false, polar_configured: true })
    }
    return jsonResponse(404, { error: { code: 'not_found', message: url } })
  })
  vi.stubGlobal('fetch', fetchMock)
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={[path]}>
        <Routes>
          <Route element={<Layout />}>
            <Route index element={<h1>Yleiskuva-sivu</h1>} />
            <Route path="/exercises" element={<h1>Harjoitukset-sivu</h1>} />
            <Route path="/sleep" element={<h1>Uni-sivu</h1>} />
            <Route path="/activity" element={<h1>Aktiivisuus-sivu</h1>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('Layout-navigaatio', () => {
  it('näyttää kaikki neljä osiota omina linkkeinään', () => {
    renderLayout('/')
    const nav = screen.getByRole('navigation', { name: 'Päävalikko' })
    const links = within(nav).getAllByRole('link')

    expect(links.map((a) => a.getAttribute('href'))).toEqual(['/', '/exercises', '/sleep', '/activity'])
    // Sama linkki sisältää koko nimen työpöydälle ja lyhennyksen alapalkkiin;
    // CSS näyttää niistä vain toisen kerrallaan.
    expect(within(links[2]!).getByText('Uni ja palautuminen')).toBeInTheDocument()
    expect(within(links[2]!).getByText('Uni')).toBeInTheDocument()
  })

  it('merkitsee vain nykyisen osion aktiiviseksi', () => {
    renderLayout('/sleep')
    const nav = screen.getByRole('navigation', { name: 'Päävalikko' })
    const active = within(nav)
      .getAllByRole('link')
      .filter((a) => a.className.includes('active'))

    expect(active.map((a) => a.getAttribute('href'))).toEqual(['/sleep'])
  })

  it('pitää Yleiskuvan aktiivisena vain juuripolulla', () => {
    renderLayout('/exercises')
    const nav = screen.getByRole('navigation', { name: 'Päävalikko' })
    const overview = within(nav).getAllByRole('link')[0]!

    expect(overview.className).not.toContain('active')
  })
})
