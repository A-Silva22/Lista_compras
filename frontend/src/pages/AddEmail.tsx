import { useState } from 'react'
import type { FormEvent } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import { api } from '../api'
import { useAuth } from '../auth'

export default function AddEmail() {
  const { me, setMe } = useAuth()
  const [email, setEmail] = useState('')
  const [erro, setErro] = useState('')
  const [busy, setBusy] = useState(false)
  const nav = useNavigate()
  const location = useLocation()
  const next = (location.state as { next?: string } | null)?.next

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setErro('')
    setBusy(true)
    try {
      const updated = await api.setEmail(email)
      setMe(updated)
      nav(next ?? '/', { replace: true })
    } catch (err) {
      setErro((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  async function logout() {
    await api.logout()
    setMe(null)
    nav('/entrar', { replace: true })
  }

  // Customer already has an email associated: don't ask for anything — just
  // reassure them that password recovery by email is available, then continue.
  if (me && !me.needs_email) {
    return (
      <div className="auth-card">
        <img src="/logo3.png" alt="ListaIsto" className="auth-logo-large" />
        <h1>Olá, {me.username}</h1>
        <p className="subtitle">
          Já tem um email associado{me.email ? ` (${me.email})` : ''}. Pode
          recuperar a palavra-passe por email sempre que precisar.
        </p>
        <button type="button" onClick={() => nav(next ?? '/', { replace: true })}>
          Continuar
        </button>
        <p className="link">
          <a href="#" onClick={e => { e.preventDefault(); logout() }}>Sair</a>
        </p>
      </div>
    )
  }

  return (
    <div className="auth-card">
      <img src="/logo3.png" alt="ListaIsto" className="auth-logo-large" />
      <h1>Olá, {me?.username}</h1>
      <p className="subtitle">
        Para podermos enviar-te o link de recuperação de palavra-passe, precisamos do teu email.
      </p>
      {erro && <div className="erro">{erro}</div>}
      <form onSubmit={onSubmit}>
        <input
          type="email" placeholder="o-teu@email.com" autoComplete="email"
          value={email} onChange={e => setEmail(e.target.value)} required
        />
        <button type="submit" disabled={busy}>{busy ? 'A guardar…' : 'Guardar'}</button>
      </form>
      <p className="link">
        <a href="#" onClick={e => { e.preventDefault(); logout() }}>Sair</a>
      </p>
    </div>
  )
}
