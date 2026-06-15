export interface Me {
  id: number
  username: string
  email: string
  needs_email: boolean
}

// ── List/item shapes (mirror the Rust JSON DTOs) ──
export interface Item {
  id: number
  nome: string
  quantidade: string
  comprar: boolean
}
export interface Partilha {
  utilizador_id: number
  username: string
}
export interface ShareLink {
  id: number
  token: string
  url: string
  expira_em_str: string
  pode_adicionar: boolean
  pode_editar: boolean
  pode_apagar: boolean
  pode_toggle: boolean
}
export interface ListSummary {
  id: number
  nome: string
  shared_users: boolean
  shared_link: boolean
}
export interface ListsResponse {
  lists: ListSummary[]
  active_id: number
}
export interface ListDetail {
  id: number
  nome: string
  is_owner: boolean
  a_comprar: Item[]
  despensa: Item[]
  partilhas: Partilha[]
  links: ShareLink[]
}
export interface PublicView {
  lista_nome: string
  a_comprar: Item[]
  despensa: Item[]
  expira_em_str: string
  pode_adicionar: boolean
  pode_editar: boolean
  pode_apagar: boolean
  pode_toggle: boolean
  already_logged_in: boolean
}

interface ApiError {
  error: string
}

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    ...init,
  })
  if (res.status === 204) return undefined as T
  const text = await res.text()
  const data = text ? JSON.parse(text) : null
  if (!res.ok) {
    const msg = (data as ApiError | null)?.error ?? `HTTP ${res.status}`
    throw new Error(msg)
  }
  return data as T
}

const body = (o: unknown) => JSON.stringify(o)

export const api = {
  // auth
  me: () => call<Me>('/api/me'),
  login: (username: string, password: string) =>
    call<Me>('/api/login', { method: 'POST', body: body({ username, password }) }),
  register: (username: string, password: string) =>
    call<Me>('/api/register', { method: 'POST', body: body({ username, password }) }),
  setEmail: (email: string) =>
    call<Me>('/api/email', { method: 'POST', body: body({ email }) }),
  logout: () => call<void>('/api/logout', { method: 'POST' }),
  recover: (email: string) =>
    call<{ sent: boolean }>('/api/password/recover', { method: 'POST', body: body({ email }) }),
  resetCheck: (token: string) => call<{ valid: boolean }>(`/api/password/reset/${token}`),
  resetPassword: (token: string, new_password: string) =>
    call<void>('/api/password/reset', { method: 'POST', body: body({ token, new_password }) }),
  changePassword: (old_password: string, new_password: string) =>
    call<void>('/api/password/change', { method: 'POST', body: body({ old_password, new_password }) }),
  changeUsername: (username: string) =>
    call<Me>('/api/username', { method: 'POST', body: body({ username }) }),

  // lists
  lists: () => call<ListsResponse>('/api/lists'),
  createList: (nome: string) => call<ListDetail>('/api/lists', { method: 'POST', body: body({ nome }) }),
  selectList: (id: number) => call<ListDetail>(`/api/lists/${id}/select`, { method: 'POST' }),
  listDetail: (id: number) => call<ListDetail>(`/api/lists/${id}`),
  deleteList: (id: number) => call<void>(`/api/lists/${id}`, { method: 'DELETE' }),

  // items
  addItem: (lid: number, nome: string, quantidade = '1') =>
    call<ListDetail>(`/api/lists/${lid}/items`, { method: 'POST', body: body({ nome, quantidade }) }),
  toggleItem: (lid: number, iid: number, destino: 'comprar' | 'despensa' | '' = '') =>
    call<ListDetail>(`/api/lists/${lid}/items/${iid}/toggle`, { method: 'POST', body: body({ destino }) }),
  editItem: (lid: number, iid: number, nome: string, quantidade: string) =>
    call<ListDetail>(`/api/lists/${lid}/items/${iid}`, { method: 'POST', body: body({ nome, quantidade }) }),
  deleteItem: (lid: number, iid: number) =>
    call<ListDetail>(`/api/lists/${lid}/items/${iid}`, { method: 'DELETE' }),
  qtyItem: (lid: number, iid: number, direcao: 'mais' | 'menos') =>
    call<ListDetail>(`/api/lists/${lid}/items/${iid}/qty`, { method: 'POST', body: body({ direcao }) }),
  search: (q: string) => call<{ suggestions: string[] }>(`/api/search?q=${encodeURIComponent(q)}`),
  match: (lid: number, q: string) =>
    call<{ found: boolean; id: number | null; nome: string | null; na_despensa: boolean | null }>(
      `/api/lists/${lid}/match?q=${encodeURIComponent(q)}`),

  // share
  share: (lid: number, username: string) =>
    call<ListDetail>(`/api/lists/${lid}/share`, { method: 'POST', body: body({ username }) }),
  unshare: (lid: number, uid: number) =>
    call<ListDetail>(`/api/lists/${lid}/share/${uid}`, { method: 'DELETE' }),
  createLink: (lid: number, opts: { duracao: number; unidade: string; pode_adicionar: boolean; pode_editar: boolean; pode_apagar: boolean; pode_toggle: boolean }) =>
    call<ListDetail>(`/api/lists/${lid}/links`, { method: 'POST', body: body(opts) }),
  deleteLink: (lid: number, linkId: number) =>
    call<ListDetail>(`/api/lists/${lid}/links/${linkId}`, { method: 'DELETE' }),

  // public share link
  publicView: (token: string) => call<PublicView>(`/api/public/${token}`),
  publicStash: (token: string) => call<void>(`/api/public/${token}/stash`, { method: 'POST' }),
  publicAdd: (token: string, nome: string) =>
    call<PublicView>(`/api/public/${token}/items`, { method: 'POST', body: body({ nome }) }),
  publicToggle: (token: string, iid: number, destino: 'comprar' | 'despensa' | '' = '') =>
    call<PublicView>(`/api/public/${token}/items/${iid}/toggle`, { method: 'POST', body: body({ destino }) }),
  publicDelete: (token: string, iid: number) =>
    call<PublicView>(`/api/public/${token}/items/${iid}`, { method: 'DELETE' }),
}
