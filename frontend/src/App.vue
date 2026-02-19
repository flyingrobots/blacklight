<template>
  <div class="app">
    <header class="app-header">
      <nav class="nav-bar">
        <ul class="nav-links">
          <li><router-link to="/">Dashboard</router-link></li>
          <li><router-link to="/sessions" :class="{ 'router-link-active': $route.path.startsWith('/sessions') }">Sessions</router-link></li>
          <li><router-link to="/projects">Projects</router-link></li>
          <li><router-link to="/analytics">Analytics</router-link></li>
          <li><router-link to="/files">Files</router-link></li>
          <li><router-link to="/storage">Storage</router-link></li>
          <li class="nav-review">
            <router-link to="/review">
              Review
              <span v-if="pendingReviewCount > 0" class="review-badge">{{ pendingReviewCount }}</span>
            </router-link>
          </li>
        </ul>
      </nav>

      <div class="masthead">
        <div class="logo-wrap" ref="logoWrapRef">
          <img :src="logoUrl" alt="Blacklight" class="logo-base" ref="logoRef" :style="{ filter: logoFilter }" />
          <svg class="logo-circles" ref="circlesRef" preserveAspectRatio="xMinYMid meet"></svg>
        </div>
      </div>

      <div class="toolbar">
        <div class="search-container">
          <input
            v-model="searchQuery"
            placeholder="Search..."
            class="search-input"
            @keydown.enter="onSearch"
          />
        </div>
        <ThemeSwitcher />
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
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { gsap } from 'gsap'
import logoUrl from '@/assets/BLACKLIGHT.svg'
import IndexerHUD from '@/components/IndexerHUD.vue'
import NotificationContainer from '@/components/NotificationContainer.vue'
import ThemeSwitcher from '@/components/ThemeSwitcher.vue'
import { useNotifications } from '@/composables/useNotifications'
import { useTheme } from '@/composables/useTheme'
import { api } from '@/api/client'

// Initialize theme on app load
const { currentThemeIndex, themes } = useTheme()

const router = useRouter()
const route = useRoute()
const logoRef = ref<HTMLImageElement>()
const logoWrapRef = ref<HTMLElement>()
const circlesRef = ref<SVGSVGElement>()
const searchQuery = ref('')
const pendingReviewCount = ref(0)
let reviewPollTimer: ReturnType<typeof setInterval> | null = null
const { connectWs, disconnectWs } = useNotifications()

const isLightMode = computed(() => themes[currentThemeIndex.value].name === 'Quartz')

function onSearch() {
  const q = searchQuery.value.trim()
  if (!q) return
  router.push({ path: '/search', query: { q } })
}

const BASE_FILTER = 'invert(1) brightness(0.85) sepia(1) hue-rotate(180deg) saturate(3)'
const LIGHT_FILTER = 'none'

const logoFilter = computed(() => isLightMode.value ? LIGHT_FILTER : BASE_FILTER)

// Letter positions in SVG viewBox coordinates.
function getLetterBounds(): { cx: number; cy: number; r: number }[] {
  const LETTERS = 10
  const VB_W = 685.8
  const VB_H = 151.6
  const letterW = VB_W / LETTERS
  const cy = VB_H / 2

  return Array.from({ length: LETTERS }, (_, i) => ({
    cx: letterW * (i + 0.5),
    cy,
    r: letterW / 2,
  }))
}

async function pollPendingReview() {
  try {
    const data = await api.enrichment.pendingCount()
    pendingReviewCount.value = data.count
  } catch { /* ignore */ }
}

onMounted(() => {
  connectWs()
  // Delay initial poll to allow backend to start up
  setTimeout(pollPendingReview, 2000)
  reviewPollTimer = setInterval(pollPendingReview, 30_000)

  if (!circlesRef.value || !logoWrapRef.value) return

  const VB_W = 685.8
  const VB_H = 151.6

  const bounds = getLetterBounds()
  const svgEl = circlesRef.value
  const wrap = logoWrapRef.value
  svgEl.setAttribute('viewBox', `-10 -40 ${VB_W} ${VB_H}`)

  bounds.forEach(({ cx, cy, r }, i) => {
    const rainbowImg = document.createElement('img')
    rainbowImg.src = logoUrl
    rainbowImg.className = 'logo-letter-rainbow'
    const leftPct = (i / 10) * 100
    const rightPct = 100 - ((i + 1) / 10) * 100
    rainbowImg.style.clipPath = `inset(0 ${rightPct}% 0 ${leftPct}%)`
    rainbowImg.style.opacity = '0'
    wrap.appendChild(rainbowImg)

    const proxy = { hue: 0, opacity: 0 }
    let wantStop = true

    const tween = gsap.to(proxy, {
      hue: 360,
      duration: 2,
      ease: 'none',
      repeat: -1,
      paused: true,
      onUpdate: () => {
        if (!wantStop && proxy.opacity < 1) {
          proxy.opacity = Math.min(1, proxy.opacity + 0.1)
        }
        if (wantStop) {
          proxy.opacity = Math.max(0, proxy.opacity - 0.05)
          if (proxy.opacity <= 0) {
            tween.pause()
          }
        }
        rainbowImg.style.opacity = String(proxy.opacity)
        const filter = isLightMode.value ? 'none' : BASE_FILTER
        rainbowImg.style.filter = filter === 'none' ? `hue-rotate(${proxy.hue}deg)` : `${filter} hue-rotate(${proxy.hue}deg)`
      },
    })

    const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect')
    rect.setAttribute('x', String(cx - r - 10))
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
  if (reviewPollTimer) {
    clearInterval(reviewPollTimer)
    reviewPollTimer = null
  }
})
</script>

<style>
.app {
  min-height: 100vh;
  background: var(--bl-bg);
  color: var(--bl-text);
  transition: background 0.3s, color 0.3s;
}

/* Masthead */
.masthead {
  display: flex;
  justify-content: center;
  padding: 2rem 2rem 1.5rem;
}

.logo-wrap {
  position: relative;
  width: 100%;
  max-width: 1000px;
}
.logo-base {
  width: 100%;
  display: block;
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
  border-bottom: 1px solid var(--bl-border);
  background: var(--bl-bg);
  z-index: 100;
}
.nav-links {
  list-style: none;
  display: flex;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  max-width: 1200px;
  margin: 0 auto;
}
.nav-links li a {
  display: block;
  padding: 0.5rem 1rem;
  border-radius: var(--bl-radius-md);
  color: var(--bl-text-2);
  font-size: var(--bl-text-sm);
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  transition: all 0.2s;
}
.nav-links li a:hover {
  color: var(--bl-text);
  background: var(--bl-bg-2);
  text-decoration: none;
}
.nav-links li a.router-link-active {
  color: var(--bl-accent);
  background: var(--bl-bg-2);
  text-decoration: none;
  box-shadow: 0 2px 0 var(--bl-accent);
}

/* Sub-header with search and theme */
.toolbar {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  padding: 1.5rem 2rem;
  max-width: 1200px;
  margin: 0 auto;
}
.search-container {
  position: relative;
  width: 100%;
  max-width: 540px;
}
.search-container .search-input {
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  color: var(--bl-text);
  font-size: var(--bl-text-md);
  padding: 0.6rem 1rem;
  width: 100%;
  outline: none;
  transition: all 0.2s;
}
.search-container .search-input:focus {
  border-color: var(--bl-accent);
  box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
}

/* Review badge */
.nav-review a {
  position: relative;
}
.review-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 18px;
  height: 18px;
  padding: 0 5px;
  border-radius: 9px;
  background: var(--bl-danger);
  color: #fff;
  font-size: 0.6875rem;
  font-weight: 700;
  margin-left: 0.35rem;
  vertical-align: middle;
  line-height: 1;
}

/* Content area */
.content {
  max-width: 1200px;
  margin: 0 auto;
  padding: 1rem 2rem 4rem;
}
</style>
