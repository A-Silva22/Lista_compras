import type { ReactNode } from 'react'
import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'
import { AuthProvider, ProtectedRoute, useAuth } from './auth'
import Login from './pages/Login'
import Register from './pages/Register'
import AddEmail from './pages/AddEmail'
import Recover from './pages/Recover'
import ResetPassword from './pages/ResetPassword'
import Home from './pages/Home'
import PublicLink from './pages/PublicLink'

function PublicOnly({ children }: { children: ReactNode }) {
  const { me, loading } = useAuth()
  if (loading) return <div className="centered">A carregar…</div>
  // Authenticated users always pass through the email step: it prompts for an
  // email when missing, otherwise shows the "recover by email" message. This
  // also matches Login's post-submit navigation, avoiding a redirect race.
  if (me) return <Navigate to="/adicionar-email" replace />
  return <>{children}</>
}

export default function App() {
  return (
    <BrowserRouter>
      <AuthProvider>
        <Routes>
          <Route path="/entrar" element={<PublicOnly><Login /></PublicOnly>} />
          <Route path="/registar" element={<PublicOnly><Register /></PublicOnly>} />
          <Route path="/recuperar" element={<PublicOnly><Recover /></PublicOnly>} />
          <Route path="/reset/:token" element={<ResetPassword />} />
          <Route path="/adicionar-email" element={<ProtectedRoute><AddEmail /></ProtectedRoute>} />
          <Route path="/link/:token" element={<PublicLink />} />
          <Route path="/" element={<ProtectedRoute><Home /></ProtectedRoute>} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </AuthProvider>
    </BrowserRouter>
  )
}
