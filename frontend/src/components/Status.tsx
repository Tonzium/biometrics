import { ApiError } from '../api/client'

export function Loading({ label = 'Ladataan…' }: { label?: string }) {
  return (
    <div className="status status-loading" role="status" aria-live="polite">
      <span className="spinner" aria-hidden="true" />
      {label}
    </div>
  )
}

export function ErrorBox({ error, title = 'Tietojen haku epäonnistui' }: { error: unknown; title?: string }) {
  const message =
    error instanceof ApiError
      ? `${error.status} ${error.code}: ${error.message}`
      : error instanceof Error
        ? error.message
        : String(error)
  return (
    <div className="status status-error" role="alert">
      <strong>{title}</strong>
      <div className="muted">{message}</div>
    </div>
  )
}

export function Empty({ children }: { children: React.ReactNode }) {
  return <div className="status status-empty">{children}</div>
}

export function Notice({ kind = 'info', children }: { kind?: 'info' | 'success' | 'warning'; children: React.ReactNode }) {
  return (
    <div className={`notice notice-${kind}`} role="status">
      {children}
    </div>
  )
}
