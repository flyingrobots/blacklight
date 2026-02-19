import { ref, watchEffect } from 'vue'

export interface ThemePalette {
  name: string
  colors: [string, string, string, string, string]
}

export const themes: ThemePalette[] = [
  {
    name: 'Slate',
    colors: ['#72728f', '#ca5edd', '#bf68e8', '#8e77f2', '#ad86fa'],
  },
  {
    name: 'Indigo',
    colors: ['#0f172a', '#3b82f6', '#60a5fa', '#93c5fd', '#bfdbfe'],
  },
  {
    name: 'Orchid',
    colors: ['#2d1b4d', '#a855f7', '#c084fc', '#d8b4fe', '#f3e8ff'],
  },
  {
    name: 'Quartz',
    colors: ['#f8fafc', '#64748b', '#475569', '#334155', '#1e293b'],
  },
]

const STORAGE_KEY = 'blacklight-theme'

const currentThemeIndex = ref(loadSavedTheme())

function loadSavedTheme(): number {
  try {
    const saved = localStorage.getItem(STORAGE_KEY)
    if (saved !== null) {
      const idx = parseInt(saved, 10)
      if (idx >= 0 && idx < themes.length) return idx
    }
  } catch { /* ignore */ }
  return 0
}

function applyTheme(palette: ThemePalette) {
  const root = document.documentElement
  const isLight = palette.name === 'Quartz'
  const [c1, c2, c3, c4, c5] = palette.colors

  if (isLight) {
    root.style.setProperty('--bl-bg', '#ffffff')
    root.style.setProperty('--bl-bg-2', '#f1f5f9')
    root.style.setProperty('--bl-bg-3', '#e2e8f0')
    root.style.setProperty('--bl-border', '#cbd5e1')
    root.style.setProperty('--bl-text', '#0f172a')
    root.style.setProperty('--bl-text-2', '#475569')
    root.style.setProperty('--bl-accent', '#3b82f6')
    root.style.setProperty('--bl-accent-hover', '#2563eb')
    root.style.setProperty('--bl-purple', '#8b5cf6')
    root.style.filter = 'none'
  } else {
    root.style.setProperty('--bl-accent', c3)
    root.style.setProperty('--bl-accent-hover', c4)
    root.style.setProperty('--bl-purple', c5)

    // Higher multipliers for background so it's not "pitch black"
    root.style.setProperty('--bl-bg', blend(c1, '#000000', 0.85))
    root.style.setProperty('--bl-bg-2', blend(c1, '#000000', 0.75))
    root.style.setProperty('--bl-bg-3', blend(c1, '#000000', 0.65))
    root.style.setProperty('--bl-border', blend(c1, '#ffffff', 0.15))
    root.style.setProperty('--bl-text', '#f8fafc')
    root.style.setProperty('--bl-text-2', '#94a3b8')
  }
}

/** Blend two hex colors. */
function blend(hex1: string, hex2: string, ratio: number): string {
  const r1 = parseInt(hex1.slice(1, 3), 16)
  const g1 = parseInt(hex1.slice(3, 5), 16)
  const b1 = parseInt(hex1.slice(5, 7), 16)
  const r2 = parseInt(hex2.slice(1, 3), 16)
  const g2 = parseInt(hex2.slice(3, 5), 16)
  const b2 = parseInt(hex2.slice(5, 7), 16)

  const r = Math.round(r1 * (1 - ratio) + r2 * ratio)
  const g = Math.round(g1 * (1 - ratio) + g2 * ratio)
  const b = Math.round(b1 * (1 - ratio) + b2 * ratio)

  return `#${r.toString(16).padStart(2, '0')}${g.toString(16).padStart(2, '0')}${b.toString(16).padStart(2, '0')}`
}

export function useTheme() {
  watchEffect(() => {
    applyTheme(themes[currentThemeIndex.value])
    try {
      localStorage.setItem(STORAGE_KEY, String(currentThemeIndex.value))
    } catch { /* ignore */ }
  })

  function setTheme(index: number) {
    if (index >= 0 && index < themes.length) {
      currentThemeIndex.value = index
    }
  }

  function cycleTheme() {
    currentThemeIndex.value = (currentThemeIndex.value + 1) % themes.length
  }

  return {
    themes,
    currentThemeIndex,
    currentTheme: () => themes[currentThemeIndex.value],
    setTheme,
    cycleTheme,
  }
}
