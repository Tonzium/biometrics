import { NavLink, Outlet, useNavigate } from 'react-router'

import { useLogout, useMe, useMeta } from '../api/hooks'

type IconName = 'overview' | 'exercises' | 'sleep' | 'activity'

interface NavItem {
  to: string
  label: string
  /** Lyhennys mobiilin alapalkkiin, kun koko nimi ei mahdu. */
  short?: string
  icon: IconName
  end?: boolean
}

const NAV: NavItem[] = [
  { to: '/', label: 'Yleiskuva', icon: 'overview', end: true },
  { to: '/exercises', label: 'Harjoitukset', icon: 'exercises' },
  { to: '/sleep', label: 'Uni ja palautuminen', short: 'Uni', icon: 'sleep' },
  { to: '/activity', label: 'Aktiivisuus', icon: 'activity' },
]

/**
 * Alapalkin kuvakkeet. Piirretään viivoina ilman täyttöä, jolloin ne perivät
 * linkin värin ja muuttuvat punaisiksi aktiivisella välilehdellä.
 * Näkyvät vain mobiilissa; työpöydällä CSS piilottaa ne.
 */
function NavIcon({ name }: { name: IconName }) {
  return (
    <svg
      className="nav-icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {name === 'overview' ? (
        <>
          <rect x="3" y="3" width="7.5" height="7.5" rx="1.5" />
          <rect x="13.5" y="3" width="7.5" height="7.5" rx="1.5" />
          <rect x="3" y="13.5" width="7.5" height="7.5" rx="1.5" />
          <rect x="13.5" y="13.5" width="7.5" height="7.5" rx="1.5" />
        </>
      ) : name === 'exercises' ? (
        // Käsipaino.
        <path d="M4 9.5v5M7.5 6.5v11M16.5 6.5v11M20 9.5v5M7.5 12h9" />
      ) : name === 'sleep' ? (
        // Kuunsirppi.
        <path d="M20.5 14.8A8.6 8.6 0 0 1 9.2 3.5a7 7 0 1 0 11.3 11.3Z" />
      ) : (
        // Sykekäyrä.
        <path d="M3 12h3.5L9 5l4 14 2.5-7H21" />
      )}
    </svg>
  )
}

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
                <NavIcon name={item.icon} />
                <span className="nav-label">{item.label}</span>
                <span className="nav-label-short">{item.short ?? item.label}</span>
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
