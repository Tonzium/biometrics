import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { MemoryRouter, Route, Routes } from 'react-router'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { LoginPage } from './LoginPage'

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })
}

function renderLogin(fetchMock: typeof fetch) {
  vi.stubGlobal('fetch', fetchMock)
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter initialEntries={['/login']}>
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/settings" element={<h1>Asetukset-sivu</h1>} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  )
}

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('LoginPage', () => {
  it('posts credentials and navigates to settings on success', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input)
      if (url === '/api/auth/me') return jsonResponse(401, { error: { code: 'unauthorized', message: 'x' } })
      if (url === '/api/auth/login' && init?.method === 'POST') {
        expect(JSON.parse(String(init.body))).toEqual({ email: 'toni@example.com', password: 'secret-password' })
        return jsonResponse(200, { id: '1', email: 'toni@example.com', role: 'owner', created_at: '2026-09-10T00:00:00Z' })
      }
      return jsonResponse(404, { error: { code: 'not_found', message: url } })
    })
    renderLogin(fetchMock as unknown as typeof fetch)

    const user = userEvent.setup()
    await user.type(await screen.findByLabelText('Sähköposti'), 'toni@example.com')
    await user.type(screen.getByLabelText('Salasana'), 'secret-password')
    await user.click(screen.getByRole('button', { name: 'Kirjaudu' }))

    expect(await screen.findByText('Asetukset-sivu')).toBeInTheDocument()
    expect(fetchMock).toHaveBeenCalledWith('/api/auth/login', expect.objectContaining({ method: 'POST' }))
  })

  it('shows a Finnish error on 401', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/me') return jsonResponse(401, { error: { code: 'unauthorized', message: 'x' } })
      if (url === '/api/auth/login') return jsonResponse(401, { error: { code: 'unauthorized', message: 'authentication required' } })
      return jsonResponse(404, {})
    })
    renderLogin(fetchMock as unknown as typeof fetch)

    const user = userEvent.setup()
    await user.type(await screen.findByLabelText('Sähköposti'), 'a@b.fi')
    await user.type(screen.getByLabelText('Salasana'), 'wrong')
    await user.click(screen.getByRole('button', { name: 'Kirjaudu' }))

    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('Väärä sähköposti tai salasana.'))
  })

  it('shows a Finnish error on 429', async () => {
    const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input)
      if (url === '/api/auth/me') return jsonResponse(401, { error: { code: 'unauthorized', message: 'x' } })
      if (url === '/api/auth/login')
        return jsonResponse(429, { error: { code: 'too_many_requests', message: 'too many login attempts, try again shortly' } })
      return jsonResponse(404, {})
    })
    renderLogin(fetchMock as unknown as typeof fetch)

    const user = userEvent.setup()
    await user.type(await screen.findByLabelText('Sähköposti'), 'a@b.fi')
    await user.type(screen.getByLabelText('Salasana'), 'secret')
    await user.click(screen.getByRole('button', { name: 'Kirjaudu' }))

    await waitFor(() =>
      expect(screen.getByRole('alert')).toHaveTextContent(
        'Liikaa kirjautumisyrityksiä juuri nyt. Yritä hetken kuluttua uudelleen.',
      ),
    )
  })

  it('keeps the submit button disabled until both fields are filled', async () => {
    const fetchMock = vi.fn(async () => jsonResponse(401, { error: { code: 'unauthorized', message: 'x' } }))
    renderLogin(fetchMock as unknown as typeof fetch)
    const button = await screen.findByRole('button', { name: 'Kirjaudu' })
    expect(button).toBeDisabled()
  })
})
