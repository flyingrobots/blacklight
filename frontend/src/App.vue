<template>
  <div class="app">
    <header class="app-header">
      <nav class="nav-bar">
        <ul class="nav-links">
          <li><router-link to="/">Dashboard</router-link></li>
          <li><router-link to="/sessions">Sessions</router-link></li>
          <li><router-link to="/projects">Projects</router-link></li>
          <li><router-link to="/analytics">Analytics</router-link></li>
          <li><router-link to="/files">Files</router-link></li>
          <li><router-link to="/storage">Storage</router-link></li>
        </ul>
      </nav>
      <div class="masthead">
        <div class="logo-wrap" ref="logoWrapRef">
          <img :src="logoUrl" alt="Blacklight" class="logo-base" ref="logoRef" />
          <svg class="logo-circles" ref="circlesRef" preserveAspectRatio="xMinYMid meet"></svg>
        </div>
      </div>
      <div class="nav-search">
        <input
          v-model="searchQuery"
          placeholder="Search..."
          class="search-input"
          @keydown.enter="onSearch"
        />
      </div>
    </header>
    <main class="content">
      <router-view />
    </main>
    <IndexerHUD />
    <NotificationContainer />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { gsap } from 'gsap'
import logoUrl from '@/assets/BLACKLIGHT.svg'
import IndexerHUD from '@/components/IndexerHUD.vue'
import NotificationContainer from '@/components/NotificationContainer.vue'
import { useNotifications } from '@/composables/useNotifications'

const router = useRouter()
const logoRef = ref<HTMLImageElement>()
const logoWrapRef = ref<HTMLElement>()
const circlesRef = ref<SVGSVGElement>()
const searchQuery = ref('')
const { connectWs, disconnectWs } = useNotifications()

function onSearch() {
  const q = searchQuery.value.trim()
  if (!q) return
  router.push({ path: '/search', query: { q } })
}

const BASE_FILTER = 'invert(1) brightness(0.85) sepia(1) hue-rotate(180deg) saturate(3)'

// Letter positions in SVG viewBox coordinates.
// Content spans x=0..66.8, y=0..71.6 within viewBox "-10 -40 685.8 151.6".
// BLACKLIGHT = 10 letters, monospaced pixel-art font.
function getLetterBounds(): { cx: number; cy: number; r: number }[] {
  const LETTERS = 10
  const CONTENT_X = 0
  const CONTENT_W = 665.8
  const CONTENT_Y = 0
  const CONTENT_H = 71.6
  const letterW = CONTENT_W / LETTERS
  const cy = CONTENT_Y + CONTENT_H / 2

  return Array.from({ length: LETTERS }, (_, i) => ({
    cx: CONTENT_X + letterW * (i + 0.5),
    cy,
    r: letterW / 2,
  }))
}

onMounted(() => {
  connectWs()

  if (!circlesRef.value || !logoWrapRef.value) return

  const VB_W = 685.8
  const VB_H = 151.6

  const bounds = getLetterBounds()
  const svgEl = circlesRef.value
  const wrap = logoWrapRef.value
  svgEl.setAttribute('viewBox', `-10 -40 ${VB_W} ${VB_H}`)

  bounds.forEach(({ cx, cy, r }) => {
    // --- Rainbow layer: clipped copy that cycles hue on hover ---
    const rainbowImg = document.createElement('img')
    rainbowImg.src = logoUrl
    rainbowImg.className = 'logo-letter-rainbow'
    // clip-path uses % of the element's own dimensions.
    // The img renders the full viewBox (-10 -40 685.8 151.6).
    // Content coords cx,r are in viewBox content space (0-based).
    // Offset by 10 to account for viewBox starting at x=-10.
    const leftPct = Math.max(0, ((cx - r - 3 + 10) / VB_W) * 100)
    const rightPct = Math.max(0, (1 - (cx + r + 3 + 10) / VB_W) * 100)
    rainbowImg.style.clipPath = `inset(0 ${rightPct}% 0 ${leftPct}%)`
    rainbowImg.style.opacity = '0'
    wrap.appendChild(rainbowImg)

    // Per-letter rainbow tween — paused by default
    const proxy = { hue: 0, opacity: 0 }
    let wantStop = false

    const tween = gsap.to(proxy, {
      hue: 360,
      duration: 3,
      ease: 'none',
      repeat: -1,
      paused: true,
      onUpdate: () => {
        if (!wantStop && proxy.opacity < 1) {
          proxy.opacity = Math.min(1, proxy.opacity + 0.05)
        }
        if (wantStop) {
          proxy.opacity = Math.max(0, proxy.opacity - 0.03)
          if (proxy.opacity <= 0) {
            tween.pause()
            wantStop = false
            proxy.hue = 0
          }
        }
        rainbowImg.style.opacity = String(proxy.opacity)
        rainbowImg.style.filter = `${BASE_FILTER} hue-rotate(${proxy.hue}deg)`
      },
    })

    // --- Hit zone: use a rect covering this letter's full vertical column ---
    // SVG viewBox coords: content cx,r are 0-based, viewBox origin is -10,-40
    const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect')
    rect.setAttribute('x', String(cx - r))
    rect.setAttribute('y', String(-40))
    rect.setAttribute('width', String(r * 2))
    rect.setAttribute('height', String(VB_H))
    rect.setAttribute('fill', 'transparent')
    rect.style.cursor = 'pointer'
    rect.style.pointerEvents = 'all'

    rect.addEventListener('mouseenter', () => {
      wantStop = false
      if (tween.paused()) tween.play()
    })

    rect.addEventListener('mouseleave', () => {
      wantStop = true
    })

    svgEl.appendChild(rect)
  })
})

onUnmounted(() => {
  disconnectWs()
})
</script>

<style>
.app {
  min-height: 100vh;
}

/* Masthead */
.masthead {
  display: flex;
  justify-content: center;
  padding: 5rem 2rem 2.5rem;
}
.logo-wrap {
  position: relative;
  width: 100%;
  max-width: 1400px;
}
.logo-base {
  width: 100%;
  display: block;
  filter: invert(1) brightness(0.85) sepia(1) hue-rotate(180deg) saturate(3);
}
.logo-letter-rainbow {
  position: absolute;
  inset: 0;
  width: 100%;
  display: block;
  pointer-events: none;
  opacity: 0;
}
.logo-circles {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
}

/* Horizontal nav */
.nav-bar {
  padding-top: 1rem;
}
.nav-links {
  list-style: none;
  display: flex;
  justify-content: center;
  gap: 0.25rem;
  padding: 0 1rem 0.5rem;
}
.nav-links li a {
  display: block;
  padding: 0.4rem 0.9rem;
  border-radius: 4px;
  color: var(--text-secondary);
  font-size: 0.8125rem;
  font-weight: 500;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  transition: color 0.15s, background 0.15s;
}
.nav-links li a:hover {
  color: var(--text);
  background: var(--bg-tertiary);
  text-decoration: none;
}
.nav-links li a.router-link-active {
  color: var(--accent, #0af);
  background: var(--bg-tertiary);
  text-decoration: none;
  box-shadow: inset 0 -2px 0 var(--accent, #0af);
}

/* Header search */
.nav-search {
  display: flex;
  justify-content: center;
  padding: 0 2rem 1.25rem;
}
.nav-search .search-input {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text);
  font-size: 0.875rem;
  padding: 0.5rem 1rem;
  width: 100%;
  max-width: 480px;
  outline: none;
  transition: border-color 0.15s;
}
.nav-search .search-input:focus {
  border-color: var(--accent, #0af);
}
.nav-search .search-input::placeholder {
  color: var(--text-secondary);
  opacity: 0.6;
}

/* Content area */
.content {
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem;
}
</style>
