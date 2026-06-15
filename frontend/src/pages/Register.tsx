import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { api } from '../api'
import { useAuth } from '../auth'

export default function Register() {
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [erro, setErro] = useState('')
  const [busy, setBusy] = useState(false)
  const { setMe } = useAuth()
  const nav = useNavigate()

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setErro('')
    setBusy(true)
    try {
      const me = await api.register(username, password)
      setMe(me)
      nav('/adicionar-email', { replace: true })
    } catch (err) {
      setErro((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="auth-card">
      <img src="/logo3.png" alt="ListaIsto" className="auth-logo-large" />
      <h1>Criar conta no ListaIsto</h1>
      {erro && <div className="erro">{erro}</div>}
      <form onSubmit={onSubmit}>
        <input
          type="text" placeholder="Nome de utilizador" autoComplete="username"
          value={username} onChange={e => setUsername(e.target.value)} required
        />
        <input
          type="password" placeholder="Palavra-passe" autoComplete="new-password"
          value={password} onChange={e => setPassword(e.target.value)} required
        />
        <button type="submit" disabled={busy}>{busy ? 'A criar…' : 'Registar'}</button>
      </form>
      <p className="link">Já tem conta? <Link to="/entrar">Entrar</Link></p>
    </div>
  )
}
