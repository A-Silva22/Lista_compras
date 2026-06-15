import { useState } from 'react'
import type { FormEvent } from 'react'
import { Link } from 'react-router-dom'
import { api } from '../api'

export default function Recover() {
  const [email, setEmail] = useState('')
  const [erro, setErro] = useState('')
  const [enviado, setEnviado] = useState(false)
  const [busy, setBusy] = useState(false)

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setErro('')
    setBusy(true)
    try {
      await api.recover(email)
      setEnviado(true)
    } catch (err) {
      setErro((err as Error).message)
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="auth-card">
      <img src="/logo3.png" alt="ListaIsto" className="auth-logo-large" />
      <h1>Recuperar palavra-passe</h1>
      {enviado ? (
        <>
          <div className="ok">
            Se o email existir na nossa base, será enviada uma mensagem com instruções.
          </div>
          <p className="link"><Link to="/entrar">Voltar ao login</Link></p>
        </>
      ) : (
        <>
          {erro && <div className="erro">{erro}</div>}
          <p className="subtitle">Indica o email associado à tua conta.</p>
          <form onSubmit={onSubmit}>
            <input
              type="email" placeholder="o-teu@email.com"
              value={email} onChange={e => setEmail(e.target.value)} required
            />
            <button type="submit" disabled={busy}>{busy ? 'A enviar…' : 'Enviar'}</button>
          </form>
          <p className="link"><Link to="/entrar">Voltar ao login</Link></p>
        </>
      )}
    </div>
  )
}
