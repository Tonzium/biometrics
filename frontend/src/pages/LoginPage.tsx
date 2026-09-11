import { useState } from 'react'
import { Navigate, useLocation, useNavigate } from 'react-router'

import { ApiError } from '../api/client'
import { useLogin, useMe } from '../api/hooks'

export function LoginPage() {
  const me = useMe()
  const login = useLogin()
  const navigate = useNavigate()
  const location = useLocation()
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')

  const from = (location.state as { from?: string } | null)?.from ?? '/settings'

  if (me.data) return <Navigate to={from} replace />

  function submit(event: React.FormEvent) {
    event.preventDefault()
    login.mutate({ email: email.trim(), password }, { onSuccess: () => void navigate(from, { replace: true }) })
  }

  const errorText =
    login.error instanceof ApiError && login.error.status === 401
      ? 'Väärä sähköposti tai salasana.'
      : login.error instanceof Error
        ? login.error.message
        : null

  return (
    <section className="page page-narrow">
      <h1>Kirjaudu sisään</h1>
      <p className="muted">Kirjautuminen tarvitaan vain Polar-tilin hallintaan ja synkronointiin.</p>
      <form className="card form" onSubmit={submit} noValidate>
        <label>
          Sähköposti
          <input type="email" name="email" autoComplete="username" value={email} onChange={(e) => setEmail(e.target.value)} required />
        </label>
        <label>
          Salasana
          <input type="password" name="password" autoComplete="current-password" value={password} onChange={(e) => setPassword(e.target.value)} required />
        </label>
        {errorText ? (
          <div className="status status-error" role="alert">
            {errorText}
          </div>
        ) : null}
        <button type="submit" className="btn btn-primary" disabled={login.isPending || !email || !password}>
          {login.isPending ? 'Kirjaudutaan…' : 'Kirjaudu'}
        </button>
      </form>
    </section>
  )
}
