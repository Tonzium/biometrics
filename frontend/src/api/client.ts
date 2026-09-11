/**
 * Ohut fetch-kääre backendin JSON-rajapintaan.
 *
 * - Cookie kulkee automaattisesti (same-origin: dev-proxy ja tuotannon nginx
 *   palvelevat frontendin ja `/api`:n samasta originista).
 * - Virhevastaukset `{ error: { code, message } }` nostetaan `ApiError`-poikkeuksena.
 */

import type { ErrorBody } from './types'

export class ApiError extends Error {
  readonly status: number
  readonly code: string

  constructor(status: number, code: string, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
    this.code = code
  }
}

/** Kyselyparametrit: mikä tahansa olio, jonka arvot ovat skalaareja tai tyhjiä. */
export type QueryParams = object

function withQuery(path: string, params?: QueryParams): string {
  if (!params) return path
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params as Record<string, unknown>)) {
    if (value === undefined || value === null || value === '') continue
    search.set(key, String(value))
  }
  const qs = search.toString()
  return qs ? `${path}?${qs}` : path
}

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const response = await fetch(path, {
    method,
    credentials: 'same-origin',
    headers: body === undefined ? { Accept: 'application/json' } : {
      Accept: 'application/json',
      'Content-Type': 'application/json',
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  })

  if (response.status === 204) return undefined as T

  const text = await response.text()
  let data: unknown = null
  if (text) {
    try {
      data = JSON.parse(text)
    } catch {
      data = null
    }
  }

  if (!response.ok) {
    const err = (data as Partial<ErrorBody> | null)?.error
    throw new ApiError(
      response.status,
      err?.code ?? 'http_error',
      err?.message ?? `${response.status} ${response.statusText}`,
    )
  }
  return data as T
}

export const api = {
  get: <T>(path: string, params?: QueryParams) => request<T>('GET', withQuery(path, params)),
  post: <T>(path: string, body?: unknown) => request<T>('POST', path, body),
  delete: <T>(path: string) => request<T>('DELETE', path),
}
