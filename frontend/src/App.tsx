import { Navigate, Route, Routes, useLocation } from 'react-router'

import { useMe, useMeta } from './api/hooks'
import { Layout } from './components/Layout'
import { Loading } from './components/Status'
import { ActivityPage } from './pages/ActivityPage'
import { DashboardPage } from './pages/DashboardPage'
import { ExerciseDetailPage } from './pages/ExerciseDetailPage'
import { ExercisesPage } from './pages/ExercisesPage'
import { LoginPage } from './pages/LoginPage'
import { NotFoundPage } from './pages/NotFoundPage'
import { SettingsPage } from './pages/SettingsPage'
import { SleepPage } from './pages/SleepPage'

/**
 * Suojaa datasivut, kun palvelin on asetettu yksityiseksi (PUBLIC_READ=false).
 * Oletuksena (näyteikkuna) kaikki pääsevät läpi.
 */
function ReadGuard({ children }: { children: React.ReactNode }) {
  const meta = useMeta()
  const me = useMe()
  const location = useLocation()

  if (meta.isPending || me.isPending) return <Loading />
  if (meta.data && !meta.data.public_read && !me.data) {
    return <Navigate to="/login" replace state={{ from: location.pathname }} />
  }
  return <>{children}</>
}

/** Asetussivu vaatii aina kirjautumisen. */
function OwnerGuard({ children }: { children: React.ReactNode }) {
  const me = useMe()
  const location = useLocation()
  if (me.isPending) return <Loading />
  if (!me.data) return <Navigate to="/login" replace state={{ from: location.pathname }} />
  return <>{children}</>
}

export default function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route
          index
          element={
            <ReadGuard>
              <DashboardPage />
            </ReadGuard>
          }
        />
        <Route
          path="/exercises"
          element={
            <ReadGuard>
              <ExercisesPage />
            </ReadGuard>
          }
        />
        <Route
          path="/exercises/:id"
          element={
            <ReadGuard>
              <ExerciseDetailPage />
            </ReadGuard>
          }
        />
        <Route
          path="/sleep"
          element={
            <ReadGuard>
              <SleepPage />
            </ReadGuard>
          }
        />
        <Route
          path="/activity"
          element={
            <ReadGuard>
              <ActivityPage />
            </ReadGuard>
          }
        />
        <Route
          path="/settings"
          element={
            <OwnerGuard>
              <SettingsPage />
            </OwnerGuard>
          }
        />
        <Route path="/login" element={<LoginPage />} />
        <Route path="*" element={<NotFoundPage />} />
      </Route>
    </Routes>
  )
}
