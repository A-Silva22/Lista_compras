import { createContext, useCallback, useContext, useEffect, useState } from 'react'
import type { ReactNode } from 'react'
import { Navigate, useLocation } from 'react-router-dom'
import { api } from './api'
import type { Me } from './api'

interface AuthCtx {
  me: Me | null
  loading: boolean
  setMe: (m: Me | null) => void
  refresh: () => Promise<void>
}

const Ctx = createContext<AuthCtx | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [me, setMe] = useState<Me | null>(null)
  const [loading, setLoading] = useState(true)

  const refresh = useCallback(async () => {
    try {
      const m = await api.me()
      setMe(m)
    } catch {
      setMe(null)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  return <Ctx.Provider value={{ me, loading, setMe, refresh }}>{children}</Ctx.Provider>
}

export function useAuth() {
  const ctx = useContext(Ctx)
  if (!ctx) throw new Error('useAuth outside AuthProvider')
  return ctx
}

export function ProtectedRoute({ children }: { children: ReactNode }) {
  const { me, loading } = useAuth()
  const location = useLocation()
  if (loading) return <div className="centered">A carregar…</div>
  if (!me) return <Navigate to="/entrar" state={{ from: location }} replace />
  if (me.needs_email && location.pathname !== '/adicionar-email')
    return <Navigate to="/adicionar-email" replace />
  return <>{children}</>
}

/// Logged-in users hitting React's "/" should bounce to Rust /home.
export function HomeBounce() {
  const { me, loading } = useAuth()
  if (loading) return <div className="centered">A carregar…</div>
  if (!me) return <Navigate to="/entrar" replace />
  if (me.needs_email) return <Navigate to="/adicionar-email" replace />
  if (typeof window !== 'undefined') window.location.replace('/home')
  return <div className="centered">A redireccionar…</div>
}
