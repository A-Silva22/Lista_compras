// Theme modes: 'auto' follows the browser/OS (prefers-color-scheme),
// 'light'/'dark' force a choice. Persisted in localStorage under 'theme'.
export type ThemeMode = 'auto' | 'light' | 'dark'

function systemLight(): boolean {
  return typeof window !== 'undefined'
    && window.matchMedia('(prefers-color-scheme: light)').matches
}

function applyMode(mode: ThemeMode) {
  const effectiveLight = mode === 'light' || (mode === 'auto' && systemLight())
  if (effectiveLight) document.documentElement.setAttribute('data-theme', 'light')
  else document.documentElement.removeAttribute('data-theme')
}

/** Effective light/dark right now, resolving 'auto' against the OS. */
export function isLight(): boolean {
  const m = getMode()
  return m === 'light' || (m === 'auto' && systemLight())
}

export function getMode(): ThemeMode {
  try {
    const v = localStorage.getItem('theme')
    if (v === 'light' || v === 'dark' || v === 'auto') return v
  } catch { /* ignore */ }
  return 'auto'
}

export function setMode(mode: ThemeMode) {
  try { localStorage.setItem('theme', mode) } catch { /* ignore */ }
  applyMode(mode)
}

export function initTheme() {
  applyMode(getMode())
  // Re-apply when the OS theme changes, but only while in 'auto'.
  try {
    window.matchMedia('(prefers-color-scheme: light)')
      .addEventListener('change', () => { if (getMode() === 'auto') applyMode('auto') })
  } catch { /* ignore */ }
}
