import { useEffect, useState } from 'react'
import type { FormEvent } from 'react'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { api } from '../api'

type Status = 'checking' | 'valid' | 'invalid' | 'done'

export default function ResetPassword() {
  const { token = '' } = useParams()
  const nav = useNavigate()
  const [status, setStatus] = useState<Status>('checking')
  const [password, setPassword] = useState('')
  const [erro, setErro] = useState('')
  const [busy, setBusy] = useState(false)

  // Validate the token on load so an expired/used link shows a message, not the form.
  useEffect(() => {
    api.resetCheck(token)
      .then(r => setStatus(r.valid ? 'valid' : 'invalid'))
      .catch(() => setStatus('invalid'))
  }, [token])

  async function onSubmit(e: FormEvent) {
    e.preventDefault()
    setErro(''); setBusy(true)
    try {
      await api.resetPassword(token, password)
      setStatus('done')
      setTimeout(() => nav('/entrar', { replace: true }), 2000)
    } catch (err) {
      // Token may have expired between load and submit.
      const msg = (err as Error).message
      if (/inválid|expir/i.test(msg)) setStatus('invalid')
      else setErro(msg)
    } finally { setBusy(false) }
  }

  return (
    <div className="auth-card">
      <img src="/logo3.png" alt="ListaIsto" className="auth-logo-large" />
      <h1>Nova palavra-passe</h1>

      {status === 'checking' && <p className="subtitle">A verificar o link…</p>}

      {status === 'invalid' && (
        <>
          <div className="erro">Este link de recuperação é inválido ou expirou.</div>
          <p className="link"><Link to="/recuperar">Pedir um novo link</Link></p>
          <p className="link"><Link to="/entrar">Voltar ao login</Link></p>
        </>
      )}

      {status === 'done' && (
        <>
          <div className="ok">Palavra-passe alterada. A redireccionar para o login…</div>
          <p className="link"><Link to="/entrar">Entrar agora</Link></p>
        </>
      )}

      {status === 'valid' && (
        <>
          {erro && <div className="erro">{erro}</div>}
          <form onSubmit={onSubmit}>
            <input
              type="password" placeholder="Nova palavra-passe" autoComplete="new-password"
              value={password} onChange={e => setPassword(e.target.value)} required minLength={4}
            />
            <button type="submit" disabled={busy}>{busy ? 'A guardar…' : 'Guardar'}</button>
          </form>
          <p className="link"><Link to="/entrar">Voltar ao login</Link></p>
        </>
      )}
    </div>
  )
}
