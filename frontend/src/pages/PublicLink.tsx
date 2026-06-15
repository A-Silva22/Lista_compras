import { useEffect, useRef, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { api } from '../api'
import type { Item, PublicView } from '../api'

export default function PublicLink() {
  const { token = '' } = useParams()
  const nav = useNavigate()
  const [view, setView] = useState<PublicView | null>(null)
  const [error, setError] = useState('')
  const [nome, setNome] = useState('')
  const [infoOpen, setInfoOpen] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    api.publicView(token).then(setView).catch(e => setError((e as Error).message))
  }, [token])

  if (error) return <div className="centered">{error}</div>
  if (!view) return <div className="centered">A carregar…</div>

  const perms = [
    view.pode_toggle && 'marcar comprado', view.pode_adicionar && 'adicionar',
    view.pode_editar && 'editar', view.pode_apagar && 'apagar',
  ].filter(Boolean).join(' · ') || 'só leitura'

  async function entrar() {
    await api.publicStash(token); nav('/entrar')
  }
  async function add(e: React.FormEvent) {
    e.preventDefault()
    const v = nome.trim(); if (!v) return
    setView(await api.publicAdd(token, v)); setNome(''); inputRef.current?.focus()
  }
  const toggle = async (it: Item) => setView(await api.publicToggle(token, it.id))
  const del = async (it: Item) => { if (confirm(`Apagar «${it.nome}»?`)) setView(await api.publicDelete(token, it.id)) }

  const section = (title: string, items: Item[], emptyMsg: string) => (
    <details open className="section-collapsible">
      <summary className="section-title" style={{ cursor: 'pointer', listStyle: 'none' }}>
        <span className="caret">▾</span> {title} <span className="count">{items.length}</span>
      </summary>
      {items.length === 0 ? <p className="empty-msg">{emptyMsg}</p> : (
        <ul className="item-list">
          {items.map(it => (
            <li key={it.id} className="item">
              {view.pode_toggle
                ? <button className="item-name" onClick={() => toggle(it)}>{it.nome}</button>
                : <span className="item-name" style={{ cursor: 'default' }}>{it.nome}</span>}
              {view.pode_apagar && <button className="icon-btn danger" title="Apagar" onClick={() => del(it)}>✕</button>}
            </li>
          ))}
        </ul>
      )}
    </details>
  )

  return (
    <div className="container">
      <div className="list-bar">
        <span className="active-list-name" style={{ flex: '1 1 auto', justifyContent: 'flex-start', minWidth: 0 }}>
          <img src="/logo3.png" alt="ListaIsto" style={{ height: '1.8em' }} />
          {view.lista_nome}
          <span className="badge">PARTILHADA</span>
        </span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexShrink: 0 }}>
          <span className={'info-wrap' + (infoOpen ? ' open' : '')}>
            <button className="info-icon" aria-label="Informação" onClick={() => setInfoOpen(!infoOpen)}>i</button>
            <div className="info-popover">Expira em <strong style={{ color: '#ccc' }}>{view.expira_em_str}</strong> · permissões: <span style={{ color: '#ccc' }}>{perms}</span></div>
          </span>
          {view.already_logged_in
            ? <button className="login-btn" onClick={() => nav('/')}>Abrir app</button>
            : <button className="login-btn" onClick={entrar}>Entrar</button>}
        </div>
      </div>

      <div id="listsRegion">
        {section('Artigos a comprar', view.a_comprar, 'Nenhum artigo para comprar.')}
        <hr className="divider" />
        {section('Despensa', view.despensa, 'Despensa vazia.')}
      </div>

      {view.pode_adicionar && (
        <div className="add-bar">
          <form className="add-form" onSubmit={add}>
            <input ref={inputRef} type="text" value={nome} onChange={e => setNome(e.target.value)}
              placeholder="Novo artigo..." autoComplete="off" />
            <button type="submit" className="btn-submit" title="Adicionar">+</button>
          </form>
        </div>
      )}
    </div>
  )
}
