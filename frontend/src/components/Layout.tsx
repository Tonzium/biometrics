import { NavLink, Outlet, useNavigate } from 'react-router'

import { useLogout, useMe, useMeta } from '../api/hooks'

const NAV = [
  { to: '/', label: 'Yleiskuva', end: true },
  { to: '/exercises', label: 'Harjoitukset' },
  { to: '/sleep', label: 'Uni ja palautuminen' },
  { to: '/activity', label: 'Aktiivisuus' },
]

export function Layout() {
  const me = useMe()
  const meta = useMeta()
  const logout = useLogout()
  const navigate = useNavigate()

  const user = me.data

  return (
    <div className="app">
      <header className="topbar">
        <div className="topbar-inner">
          <NavLink to="/" className="brand" end>
            <span className="brand-mark" aria-hidden="true" />
            Polar Data Hub
          </NavLink>
          <nav className="nav" aria-label="Päävalikko">
            {NAV.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.end}
                className={({ isActive }) => (isActive ? 'nav-link active' : 'nav-link')}
              >
                {item.label}
              </NavLink>
            ))}
          </nav>
          <div className="topbar-user">
            {user ? (
              <>
                <NavLink to="/settings" className={({ isActive }) => (isActive ? 'nav-link active' : 'nav-link')}>
                  Asetukset
                </NavLink>
                <button
                  type="button"
                  className="btn btn-ghost"
                  onClick={() => logout.mutate(undefined, { onSuccess: () => void navigate('/') })}
                  disabled={logout.isPending}
                >
                  Kirjaudu ulos
                </button>
              </>
            ) : (
              <NavLink to="/login" className="btn btn-ghost">
                Kirjaudu
              </NavLink>
            )}
          </div>
        </div>
      </header>

      <main className="content">
        <Outlet />
      </main>

      <footer className="footer">
        <span>Polar Data Hub · Toni Kiuru · KAMK Web-sovelluskehitys 2026</span>
        <span>
          {meta.data && !meta.data.public_read ? 'Yksityinen näkymä' : 'Julkinen näyteikkuna'} ·{' '}
          <a href="/api/docs" target="_blank" rel="noreferrer">
            API-dokumentaatio
          </a>
        </span>
      </footer>
    </div>
  )
}
