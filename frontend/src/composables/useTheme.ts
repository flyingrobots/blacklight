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
    colors: ['#0a0271', '#050572', '#5248a2', '#a98ad2', '#8cd5d4'],
  },
  {
    name: 'Orchid',
    colors: ['#9c28c5', '#9f48c6', '#c84ede', '#e963f6', '#b2b2f0'],
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
  const [c1, c2, c3, c4, c5] = palette.colors

  // Map the 5-color palette to CSS variables
  // c1 = muted base, c2 = primary accent, c3 = accent hover, c4 = secondary accent, c5 = soft highlight
  root.style.setProperty('--accent', c3)
  root.style.setProperty('--accent-hover', c4)
  root.style.setProperty('--purple', c5)

  // Derive tinted backgrounds from the darkest color
  root.style.setProperty('--bg', blendDark(c1, 0.06))
  root.style.setProperty('--bg-secondary', blendDark(c1, 0.10))
  root.style.setProperty('--bg-tertiary', blendDark(c1, 0.15))
  root.style.setProperty('--border', blendDark(c2, 0.18))
}

/** Blend a hex color toward black at the given lightness (0..1). */
function blendDark(hex: string, lightness: number): string {
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  const lr = Math.round(r * lightness)
  const lg = Math.round(g * lightness)
  const lb = Math.round(b * lightness)
  return `#${lr.toString(16).padStart(2, '0')}${lg.toString(16).padStart(2, '0')}${lb.toString(16).padStart(2, '0')}`
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
