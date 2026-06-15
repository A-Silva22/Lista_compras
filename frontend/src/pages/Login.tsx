import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useLocation, useNavigate } from 'react-router-dom'
import { api } from '../api'
import { useAuth } from '../auth'

export default function Login() {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [erro, setErro] = useState('')
  const [busy, setBusy] = useState(false)
  const { setMe } = useAuth()
  const nav = useNavigate()
  const location = useLocation()
  const next = (location.state as { from?: { pathname?: string } } | null)?.from?.pathname

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setErro('')
    setBusy(true)
    try {
      const me = await api.login(username, password)
      setMe(me)
      if (me.needs_email) {
        nav('/adicionar-email', { replace: true })
      } else {
        nav(next ?? '/', { replace: true })
      }
    } catch (err) {
      setErro((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="auth-card">
      <img src="/logo3.png" alt="ListaIsto" className="auth-logo-large" />
      <h1>Entrar no ListaIsto</h1>
      {erro && <div className="erro">{erro}</div>}
      <form onSubmit={onSubmit}>
        <input
          type="text" placeholder="Nome de utilizador" autoComplete="username"
          value={username} onChange={e => setUsername(e.target.value)} required
        />
        <input
          type="password" placeholder="Palavra-passe" autoComplete="current-password"
          value={password} onChange={e => setPassword(e.target.value)} required
        />
        <button type="submit" disabled={busy}>{busy ? 'A entrar…' : 'Entrar'}</button>
      </form>
      <p className="link"><Link to="/recuperar">Esqueceu a palavra-passe?</Link></p>
      <p className="link">Não tem conta? <Link to="/registar">Registar</Link></p>
    </div>
  )
}
