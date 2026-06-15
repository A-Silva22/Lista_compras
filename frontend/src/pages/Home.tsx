import { useCallback, useEffect, useRef, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api } from '../api'
import type { Item, ListDetail, ListSummary } from '../api'
import { useAuth } from '../auth'
import { getMode, setMode } from '../theme'
import type { ThemeMode } from '../theme'
import { Modal, PeopleIcon, LinkIcon, SearchIcon, SunIcon, MoonIcon, AutoIcon } from '../components/Modal'

// Copy text to clipboard with a non-secure-origin fallback (LAN IP over HTTP),
// then briefly confirm on the button.
async function copyText(text: string, btn: HTMLButtonElement) {
  let ok = false
  try {
    if (navigator.clipboard && window.isSecureContext) { await navigator.clipboard.writeText(text); ok = true }
  } catch { /* fall through */ }
  if (!ok) {
    const ta = document.createElement('textarea')
    ta.value = text; ta.style.position = 'fixed'; ta.style.opacity = '0'
    document.body.appendChild(ta); ta.focus(); ta.select()
    try { ok = document.execCommand('copy') } catch { /* ignore */ }
    document.body.removeChild(ta)
  }
  const old = btn.textContent
  btn.textContent = ok ? 'Copiado ✓' : 'Falhou'
  setTimeout(() => { btn.textContent = old }, 1500)
}

export default function Home() {
  const { me, setMe } = useAuth()
  const nav = useNavigate()
  const [lists, setLists] = useState<ListSummary[]>([])
  const [detail, setDetail] = useState<ListDetail | null>(null)
  const [loading, setLoading] = useState(true)
  const [mode, setModeState] = useState<ThemeMode>(getMode())

  // menus / modals
  const [hamOpen, setHamOpen] = useState(false)
  const [listMenu, setListMenu] = useState(false)
  const [userMenu, setUserMenu] = useState(false)
  const [shareOpen, setShareOpen] = useState(false)
  const [linkOpen, setLinkOpen] = useState(false)
  const [editing, setEditing] = useState<Item | null>(null)
  const [openItemMenu, setOpenItemMenu] = useState<number | null>(null)
  const [createOpen, setCreateOpen] = useState(false)
  const [accountOpen, setAccountOpen] = useState(false)

  const refreshLists = useCallback(async () => {
    const r = await api.lists()
    setLists(r.lists)
    return r
  }, [])

  useEffect(() => {
    (async () => {
      try {
        const r = await refreshLists()
        if (r.active_id) setDetail(await api.listDetail(r.active_id))
      } finally { setLoading(false) }
    })()
  }, [refreshLists])

  function closeMenus() { setHamOpen(false); setListMenu(false); setUserMenu(false); setOpenItemMenu(null) }

  // Close any open menu/dropdown when tapping outside it (mobile-style).
  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      const t = e.target as HTMLElement | null
      if (t && t.closest('.hamburger-wrapper, .list-menu-wrapper, .user-menu-wrapper, .menu-wrapper')) return
      setHamOpen(false); setListMenu(false); setUserMenu(false); setOpenItemMenu(null)
    }
    document.addEventListener('click', onDocClick)
    return () => document.removeEventListener('click', onDocClick)
  }, [])

  async function selectList(id: number) {
    closeMenus()
    setDetail(await api.selectList(id))
  }
  async function doCreate(name: string) {
    const d = await api.createList(name.trim())
    await refreshLists()
    setDetail(d)
    setCreateOpen(false)
  }
  async function deleteList() {
    if (!detail) return
    if (!confirm(`Apagar a lista «${detail.nome}» e todos os artigos?`)) return
    closeMenus()
    await api.deleteList(detail.id)
    const r = await refreshLists()
    setDetail(r.active_id ? await api.listDetail(r.active_id) : null)
  }
  async function logout() {
    await api.logout(); setMe(null); nav('/entrar', { replace: true })
  }
  function chooseMode(m: ThemeMode) { setMode(m); setModeState(m) }

  if (loading) return <div className="centered">A carregar…</div>

  return (
    <div>
      <div className="container">
        <Header
          me={me?.username ?? ''} email={me?.email ?? ''} lists={lists} detail={detail} mode={mode}
          hamOpen={hamOpen} listMenu={listMenu} userMenu={userMenu}
          setHamOpen={setHamOpen} setListMenu={setListMenu} setUserMenu={setUserMenu}
          onSelect={selectList} onCreate={() => setCreateOpen(true)} onDelete={deleteList}
          onShare={() => { setShareOpen(true); closeMenus() }}
          onLink={() => { setLinkOpen(true); closeMenus() }}
          onLogout={logout} onMode={chooseMode}
          onAccount={() => { setAccountOpen(true); closeMenus() }}
        />
        {!detail ? (
          <p className="empty-msg">Crie uma lista para começar.</p>
        ) : (
          <Lists detail={detail} setDetail={setDetail}
            openItemMenu={openItemMenu} setOpenItemMenu={setOpenItemMenu}
            onEdit={it => { setEditing(it); setOpenItemMenu(null) }} />
        )}
      </div>

      {detail && <AddBar lid={detail.id} setDetail={setDetail} />}

      {detail && (
        <>
          <EditModal item={editing} lid={detail.id} onClose={() => setEditing(null)} setDetail={setDetail} />
          <ShareModal open={shareOpen} detail={detail} onClose={() => setShareOpen(false)} setDetail={setDetail} />
          <LinkModal open={linkOpen} detail={detail} onClose={() => setLinkOpen(false)} setDetail={setDetail} />
        </>
      )}
      <NewListModal open={createOpen} onClose={() => setCreateOpen(false)} onCreate={doCreate} />
      <AccountModal open={accountOpen} onClose={() => setAccountOpen(false)} />
    </div>
  )
}

// ── User dropdown: dark/light checkbox, account button, logout ──
function UserMenu(p: {
  me: string; open: boolean; mode: ThemeMode
  setOpen: (b: boolean) => void; onMode: (m: ThemeMode) => void; onLogout: () => void; onAccount: () => void
}) {
  return (
    <div className="user-menu-wrapper">
      <button className="user-btn" onClick={() => p.setOpen(!p.open)}>{p.me} ▾</button>
      <div className={'user-dropdown' + (p.open ? ' open' : '')} onClick={e => e.stopPropagation()}>
        <div className="menu-theme">
          <span>Tema</span>
          <div className="theme-modes">
            <button type="button" className={p.mode === 'light' ? 'on' : ''} title="Claro" aria-label="Claro" onClick={() => p.onMode('light')}><SunIcon /></button>
            <button type="button" className={p.mode === 'dark' ? 'on' : ''} title="Escuro" aria-label="Escuro" onClick={() => p.onMode('dark')}><MoonIcon /></button>
            <button type="button" className={p.mode === 'auto' ? 'on' : ''} title="Automático" aria-label="Automático" onClick={() => p.onMode('auto')}><AutoIcon /></button>
          </div>
        </div>
        <button onClick={p.onAccount}>Mudar palavra-passe</button>
        <div className="menu-section">
          <span className="menu-label">Contacto</span>
          <a className="menu-email" href="mailto:geral@listaisto.pt">geral@listaisto.pt</a>
        </div>
        <button className="menu-logout" onClick={p.onLogout}>Sair</button>
      </div>
    </div>
  )
}

// ── Change-password popup ──
function AccountModal(p: { open: boolean; onClose: () => void }) {
  const [oldPw, setOldPw] = useState('')
  const [newPw, setNewPw] = useState('')
  const [fb, setFb] = useState<{ ok: boolean; msg: string } | null>(null)
  useEffect(() => { if (p.open) { setOldPw(''); setNewPw(''); setFb(null) } }, [p.open])
  if (!p.open) return null
  const changePw = async (e: React.FormEvent) => {
    e.preventDefault()
    try {
      await api.changePassword(oldPw, newPw)
      setOldPw(''); setNewPw(''); setFb({ ok: true, msg: 'Palavra-passe alterada.' })
    } catch (err) { setFb({ ok: false, msg: (err as Error).message }) }
  }
  return (
    <Modal open={p.open} onClose={p.onClose} title="Mudar palavra-passe">
      <form onSubmit={changePw} className="modal-form">
        <label className="field">
          <span>Palavra-passe atual</span>
          <input type="password" value={oldPw} onChange={e => setOldPw(e.target.value)} placeholder="••••••••" autoComplete="current-password" required />
        </label>
        <label className="field">
          <span>Nova palavra-passe</span>
          <input type="password" value={newPw} onChange={e => setNewPw(e.target.value)} placeholder="••••••••" autoComplete="new-password" required />
        </label>
        {fb && <div className={fb.ok ? 'ok' : 'erro'}>{fb.msg}</div>}
        <button type="submit" className="btn-primary">Guardar</button>
      </form>
      <button type="button" className="modal-close-link" onClick={p.onClose}>Cancelar</button>
    </Modal>
  )
}

// ── Header (hamburger picker + active name + menus) ──
function Header(p: {
  me: string; email: string; lists: ListSummary[]; detail: ListDetail | null; mode: ThemeMode
  hamOpen: boolean; listMenu: boolean; userMenu: boolean
  setHamOpen: (b: boolean) => void; setListMenu: (b: boolean) => void; setUserMenu: (b: boolean) => void
  onSelect: (id: number) => void; onCreate: () => void; onDelete: () => void
  onShare: () => void; onLink: () => void; onLogout: () => void; onMode: (m: ThemeMode) => void; onAccount: () => void
}) {
  return (
    <div className="list-bar">
      <div className="hamburger-wrapper">
        <button className="hamburger-btn" onClick={() => { p.setHamOpen(!p.hamOpen); p.setListMenu(false); p.setUserMenu(false) }}>☰</button>
        <div className={'hamburger-panel' + (p.hamOpen ? ' open' : '')}>
          {p.lists.length === 0 && <div style={{ padding: '10px 16px', color: '#666' }}>Sem listas.</div>}
          {p.lists.map(l => (
            <a key={l.id} href="#" className={p.detail?.id === l.id ? 'active' : ''}
               onClick={e => { e.preventDefault(); p.onSelect(l.id) }}>
              <span className="list-name-text">{l.nome}</span>
              {l.shared_users && <span className="share-ic" title="Partilhada com utilizadores"><PeopleIcon /></span>}
              {l.shared_link && <span className="share-ic" title="Partilhada por link"><LinkIcon /></span>}
            </a>
          ))}
        </div>
      </div>

      {p.detail && (
        <div className="list-menu-wrapper">
          <button className="list-menu-btn" onClick={() => { p.setListMenu(!p.listMenu); p.setHamOpen(false); p.setUserMenu(false) }}>⋮</button>
          <div className={'list-menu-dropdown' + (p.listMenu ? ' open' : '')} style={{ left: 0, right: 'auto' }}>
            <button onClick={p.onShare}>Partilhar com utilizador</button>
            <button onClick={p.onLink}>Partilhar por link</button>
            <button className="danger" onClick={p.onDelete}>Apagar lista</button>
          </div>
        </div>
      )}

      <span className="active-list-name">
        <img src="/logo3.png" alt="" />
        {p.detail ? p.detail.nome : 'ListaIsto.'}
      </span>

      <button className="list-tab-add" title="Nova lista" onClick={p.onCreate}>+</button>

      <UserMenu me={p.me} mode={p.mode} open={p.userMenu}
        setOpen={b => { p.setUserMenu(b); p.setHamOpen(false); p.setListMenu(false) }}
        onMode={p.onMode} onLogout={p.onLogout} onAccount={p.onAccount} />
    </div>
  )
}

// ── Lists region (a comprar / despensa) ──
function Lists(p: {
  detail: ListDetail; setDetail: (d: ListDetail) => void
  openItemMenu: number | null; setOpenItemMenu: (n: number | null) => void
  onEdit: (it: Item) => void
}) {
  const { detail } = p
  const toggle = async (it: Item) => p.setDetail(await api.toggleItem(detail.id, it.id))
  const del = async (it: Item) => { if (confirm(`Apagar «${it.nome}»?`)) p.setDetail(await api.deleteItem(detail.id, it.id)) }

  const section = (title: string, items: Item[], emptyMsg: string) => (
    <details open className="section-collapsible">
      <summary className="section-title" style={{ cursor: 'pointer', listStyle: 'none' }}>
        <span className="caret">▾</span> {title} <span className="count">{items.length}</span>
      </summary>
      {items.length === 0 ? <p className="empty-msg">{emptyMsg}</p> : (
        <ul className="item-list">
          {items.map(it => (
            <li key={it.id} className="item">
              <button className="item-name" onClick={() => toggle(it)}>{it.nome}</button>
              <div className="menu-wrapper" onClick={e => e.stopPropagation()}>
                <button className="menu-btn" onClick={() => p.setOpenItemMenu(p.openItemMenu === it.id ? null : it.id)}>⋮</button>
                <div className={'menu-dropdown' + (p.openItemMenu === it.id ? ' open' : '')}>
                  <button onClick={() => p.onEdit(it)}>✎ Editar</button>
                  <button className="danger" onClick={() => del(it)}>✕ Apagar</button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}
    </details>
  )

  return (
    <div id="listsRegion">
      {section('Artigos a comprar', detail.a_comprar, 'Nenhum artigo para comprar.')}
      <hr className="divider" />
      {section('Despensa', detail.despensa, 'Despensa vazia.')}
    </div>
  )
}

// ── Add bar (with search mode + duplicate banner) ──
function AddBar(p: { lid: number; setDetail: (d: ListDetail) => void }) {
  const [nome, setNome] = useState('')
  const [searchMode, setSearchMode] = useState(false)
  const [sugs, setSugs] = useState<string[]>([])
  const [match, setMatch] = useState<string>('')
  const inputRef = useRef<HTMLInputElement>(null)
  const deb = useRef<number | undefined>(undefined)

  useEffect(() => {
    window.clearTimeout(deb.current)
    const q = nome.trim()
    if (q.length < 2) { setSugs([]); setMatch(''); return }
    deb.current = window.setTimeout(async () => {
      if (searchMode) {
        try { setSugs((await api.search(q)).suggestions) } catch { setSugs([]) }
      } else {
        try {
          const m = await api.match(p.lid, q)
          setMatch(m.found ? `«${m.nome}» já existe ${m.na_despensa ? 'na despensa' : 'a comprar'}.` : '')
        } catch { setMatch('') }
      }
    }, 200)
  }, [nome, searchMode, p.lid])

  async function add(e: React.FormEvent) {
    e.preventDefault()
    if (searchMode) return
    const v = nome.trim(); if (!v) return
    p.setDetail(await api.addItem(p.lid, v))
    setNome(''); setMatch(''); inputRef.current?.focus()
  }

  return (
    <div className="add-bar">
      {match && <div className="match-banner">{match}</div>}
      {searchMode && sugs.length > 0 && (
        <ul className="suggest">
          {sugs.map(s => <li key={s} onClick={() => { setNome(s); setSearchMode(false); setSugs([]); inputRef.current?.focus() }}>{s}</li>)}
        </ul>
      )}
      <form className="add-form" onSubmit={add}>
        <input ref={inputRef} type="text" value={nome} onChange={e => setNome(e.target.value)}
          placeholder={searchMode ? 'Pesquisar artigos…' : 'Novo artigo...'} autoComplete="off" />
        {!searchMode && <button type="submit" className="btn-submit" title="Adicionar">+</button>}
        <button type="button" className={'btn-search-toggle' + (searchMode ? ' active' : '')}
          title="Pesquisar" onClick={() => { setSearchMode(!searchMode); setSugs([]) }}><SearchIcon /></button>
      </form>
    </div>
  )
}

// ── Edit item modal ──
function EditModal(p: { item: Item | null; lid: number; onClose: () => void; setDetail: (d: ListDetail) => void }) {
  const [nome, setNome] = useState('')
  useEffect(() => { if (p.item) setNome(p.item.nome) }, [p.item])
  if (!p.item) return null
  const save = async (e: React.FormEvent) => {
    e.preventDefault()
    p.setDetail(await api.editItem(p.lid, p.item!.id, nome.trim(), p.item!.quantidade))
    p.onClose()
  }
  return (
    <Modal open={!!p.item} onClose={p.onClose} title="Editar artigo">
      <form onSubmit={save} className="modal-form">
        <input type="text" value={nome} onChange={e => setNome(e.target.value)} placeholder="Nome" required />
        <button type="submit" className="btn-primary">Guardar</button>
        <button type="button" className="btn-ghost" onClick={p.onClose}>Cancelar</button>
      </form>
    </Modal>
  )
}

// ── New list modal ──
function NewListModal(p: { open: boolean; onClose: () => void; onCreate: (name: string) => void }) {
  const [name, setName] = useState('')
  const ref = useRef<HTMLInputElement>(null)
  useEffect(() => { if (p.open) { setName(''); setTimeout(() => ref.current?.focus(), 50) } }, [p.open])
  if (!p.open) return null
  const submit = (e: React.FormEvent) => { e.preventDefault(); if (name.trim()) p.onCreate(name.trim()) }
  return (
    <Modal open={p.open} onClose={p.onClose} title="Nova lista">
      <form onSubmit={submit} className="modal-form">
        <input ref={ref} type="text" value={name} onChange={e => setName(e.target.value)} placeholder="Nome da lista" required />
        <button type="submit" className="btn-primary">Criar</button>
        <button type="button" className="btn-ghost" onClick={p.onClose}>Cancelar</button>
      </form>
    </Modal>
  )
}

// ── Share with user modal ──
function ShareModal(p: { open: boolean; detail: ListDetail; onClose: () => void; setDetail: (d: ListDetail) => void }) {
  const [username, setUsername] = useState('')
  const [fb, setFb] = useState<{ ok: boolean; msg: string } | null>(null)
  const submit = async (e: React.FormEvent) => {
    e.preventDefault()
    try {
      p.setDetail(await api.share(p.detail.id, username.trim()))
      setUsername(''); setFb({ ok: true, msg: 'Partilhada.' })
    } catch (err) { setFb({ ok: false, msg: (err as Error).message }) }
  }
  return (
    <Modal open={p.open} onClose={p.onClose} title={`Partilhar lista «${p.detail.nome}»`}>
      <form onSubmit={submit} style={{ display: 'flex', gap: 8 }}>
        <input type="text" value={username} onChange={e => setUsername(e.target.value)}
          placeholder="Nome de utilizador" style={{ flex: '1 1 auto', minWidth: 0 }} required />
        <button type="submit" className="btn-primary" style={{ width: 'auto', flex: '0 0 auto', whiteSpace: 'nowrap' }}>Adicionar</button>
      </form>
      {fb && <div className={fb.ok ? 'ok' : 'erro'} style={{ marginTop: 10 }}>{fb.msg}</div>}
      <h4>Já partilhada com:</h4>
      {p.detail.partilhas.length === 0 ? <p className="empty-msg">Ninguém ainda.</p> : (
        <ul className="share-list">
          {p.detail.partilhas.map(u => (
            <li key={u.utilizador_id}>
              <span>{u.username}</span>
              <button className="icon-btn danger" title="Remover"
                onClick={async () => p.setDetail(await api.unshare(p.detail.id, u.utilizador_id))}>✕</button>
            </li>
          ))}
        </ul>
      )}
      <button className="btn-ghost" style={{ marginTop: 14 }} onClick={p.onClose}>Fechar</button>
    </Modal>
  )
}

// ── Share by link modal ──
function LinkModal(p: { open: boolean; detail: ListDetail; onClose: () => void; setDetail: (d: ListDetail) => void }) {
  const [duracao, setDuracao] = useState(24)
  const [unidade, setUnidade] = useState('horas')
  const [perms, setPerms] = useState({ pode_toggle: false, pode_adicionar: false, pode_editar: false, pode_apagar: false })
  const gen = async (e: React.FormEvent) => {
    e.preventDefault()
    p.setDetail(await api.createLink(p.detail.id, { duracao, unidade, ...perms }))
  }
  const fullUrl = (u: string) => window.location.origin + u
  return (
    <Modal open={p.open} onClose={p.onClose} title={`Partilhar «${p.detail.nome}» por link`}>
      <form onSubmit={gen}>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 12 }}>
          <span style={{ color: '#888', fontSize: '.9rem' }}>Expira em</span>
          <input type="number" min={1} value={duracao} onChange={e => setDuracao(+e.target.value)} style={{ width: 70 }} />
          <select value={unidade} onChange={e => setUnidade(e.target.value)}>
            <option value="minutos">minutos</option><option value="horas">horas</option><option value="dias">dias</option>
          </select>
        </div>
        <div className="perm-grid">
          {([['pode_toggle', 'Marcar comprado'], ['pode_adicionar', 'Adicionar artigos'], ['pode_editar', 'Editar artigos'], ['pode_apagar', 'Apagar artigos']] as const).map(([k, label]) => (
            <label key={k}><input type="checkbox" checked={perms[k]} onChange={e => setPerms({ ...perms, [k]: e.target.checked })} /> {label}</label>
          ))}
        </div>
        <button type="submit" className="btn-primary" style={{ marginTop: 12 }}>Gerar link</button>
      </form>
      <h4 style={{ marginTop: 18, color: '#888', fontSize: '.9rem' }}>Links activos:</h4>
      {p.detail.links.length === 0 ? <p className="empty-msg">Sem links activos.</p> : (
        <ul className="share-list">
          {p.detail.links.map(lk => (
            <li key={lk.id} style={{ flexDirection: 'column', alignItems: 'stretch', gap: 6 }}>
              <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                <input type="text" readOnly value={fullUrl(lk.url)} onClick={e => (e.target as HTMLInputElement).select()}
                  style={{ flex: 1, minWidth: 0, fontFamily: 'monospace', fontSize: '.85rem' }} />
                <button type="button" className="icon-btn" onClick={e => copyText(fullUrl(lk.url), e.currentTarget)}>Copiar</button>
                <button className="icon-btn danger" title="Apagar"
                  onClick={async () => { if (confirm('Apagar este link?')) p.setDetail(await api.deleteLink(p.detail.id, lk.id)) }}>✕</button>
              </div>
              <div style={{ fontSize: '.78rem', color: '#666' }}>expira {lk.expira_em_str}</div>
            </li>
          ))}
        </ul>
      )}
      <button className="btn-ghost" style={{ marginTop: 14 }} onClick={p.onClose}>Fechar</button>
    </Modal>
  )
}
