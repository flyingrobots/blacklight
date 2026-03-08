<template>
  <div class="app-layout">
    <!-- Sidebar -->
    <aside class="sidebar">
      <div class="sidebar-brand">
        <span class="brand-mark">BL</span>
        <span class="brand-text">Blacklight</span>
      </div>

      <nav class="sidebar-nav">
        <router-link to="/" class="nav-item" :class="{ active: isActive('/') }">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M6.906.664a1.749 1.749 0 0 1 2.188 0l5.143 4.114A1.75 1.75 0 0 1 15 6.032V13.5a1.75 1.75 0 0 1-1.75 1.75h-2.5a.75.75 0 0 1-.75-.75v-4h-4v4a.75.75 0 0 1-.75.75h-2.5A1.75 1.75 0 0 1 1 13.5V6.032a1.75 1.75 0 0 1 .763-1.254L6.906.664z"/></svg>
          <span>Home</span>
        </router-link>
        <router-link to="/search" class="nav-item" :class="{ active: isActive('/search') }">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M10.68 11.74a6 6 0 0 1-7.922-8.982 6 6 0 0 1 8.982 7.922l3.04 3.04a.749.749 0 1 1-1.06 1.06l-3.04-3.04zM11.5 7a4.499 4.499 0 1 0-8.997 0A4.499 4.499 0 0 0 11.5 7z"/></svg>
          <span>Search</span>
        </router-link>
        <router-link to="/projects" class="nav-item" :class="{ active: isActive('/projects') }">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M1.75 0h12.5C15.216 0 16 .784 16 1.75v12.5A1.75 1.75 0 0 1 14.25 16H1.75A1.75 1.75 0 0 1 0 14.25V1.75C0 .784.784 0 1.75 0zM1.5 1.75v12.5c0 .138.112.25.25.25H6.5v-13H1.75a.25.25 0 0 0-.25.25zM8 1.5v13h6.25a.25.25 0 0 0 .25-.25V1.75a.25.25 0 0 0-.25-.25H8z"/></svg>
          <span>Projects</span>
        </router-link>
        <router-link to="/insights" class="nav-item" :class="{ active: isActive('/insights') }">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M1.5 1.75V13.5h13.75a.75.75 0 0 1 0 1.5H.75a.75.75 0 0 1-.75-.75V1.75a.75.75 0 0 1 1.5 0zm14.28 2.53-5.25 5.25a.75.75 0 0 1-1.06 0L7 7.06 4.28 9.78a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042l3.25-3.25a.75.75 0 0 1 1.06 0L10 7.94l4.72-4.72a.751.751 0 0 1 1.042.018.751.751 0 0 1 .018 1.042z"/></svg>
          <span>Insights</span>
        </router-link>
        <router-link to="/operations" class="nav-item" :class="{ active: isActive('/operations') }">
          <svg class="nav-icon" viewBox="0 0 16 16" fill="currentColor"><path d="M7.429 1.525a3.751 3.751 0 0 1 1.142 0l.272.043a.249.249 0 0 0 .286-.169l.09-.27a3.75 3.75 0 0 1 .808-.987l.222-.19a.749.749 0 0 1 1.08.166l.455.739a.251.251 0 0 0 .318.1l.258-.113a3.75 3.75 0 0 1 1.232-.28l.29-.012a.749.749 0 0 1 .775.656l.106.825a.252.252 0 0 0 .23.218l.275.03a3.75 3.75 0 0 1 1.083.476l.243.148a.749.749 0 0 1 .305 1.046l-.407.727a.25.25 0 0 0 .05.331l.206.188c.35.319.636.7.84 1.125l.123.26a.749.749 0 0 1-.373.983l-.764.335a.25.25 0 0 0-.143.293l.069.267c.12.466.16.95.121 1.428l-.023.29a.749.749 0 0 1-.729.69l-.832.044a.25.25 0 0 0-.233.197l-.06.271a3.75 3.75 0 0 1-.588 1.275l-.164.227a.749.749 0 0 1-1.076.199l-.66-.493a.252.252 0 0 0-.32-.015l-.228.177a3.75 3.75 0 0 1-1.188.578l-.28.07a.749.749 0 0 1-.894-.544l-.207-.808a.25.25 0 0 0-.268-.185l-.275.035a3.746 3.746 0 0 1-.941 0l-.276-.035a.25.25 0 0 0-.268.186l-.207.806a.749.749 0 0 1-.894.544l-.281-.07a3.75 3.75 0 0 1-1.188-.578l-.228-.177a.249.249 0 0 0-.32.014l-.66.494a.749.749 0 0 1-1.076-.199l-.163-.227a3.75 3.75 0 0 1-.588-1.275l-.061-.271a.25.25 0 0 0-.232-.197l-.833-.044a.749.749 0 0 1-.728-.69l-.024-.29a3.75 3.75 0 0 1 .12-1.428l.07-.267a.25.25 0 0 0-.143-.293l-.764-.335a.749.749 0 0 1-.374-.983l.124-.26c.204-.424.49-.805.84-1.125l.205-.188a.25.25 0 0 0 .05-.33l-.406-.728a.749.749 0 0 1 .304-1.046l.243-.148a3.75 3.75 0 0 1 1.083-.476l.275-.03a.252.252 0 0 0 .23-.218l.107-.825a.749.749 0 0 1 .774-.656l.29.012a3.75 3.75 0 0 1 1.233.28l.257.113a.251.251 0 0 0 .319-.1l.455-.739a.749.749 0 0 1 1.08-.166l.221.19c.3.258.554.567.808.987l.09.27a.249.249 0 0 0 .286.168l.272-.043zM8 12.5a4.5 4.5 0 1 0 0-9 4.5 4.5 0 0 0 0 9zm0-3a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3z"/></svg>
          <span>Operations</span>
          <span v-if="pendingReviewCount > 0" class="nav-badge">{{ pendingReviewCount }}</span>
        </router-link>
      </nav>

      <div class="sidebar-footer">
        <div class="status-indicator" :class="statusClass">
          <span class="status-dot"></span>
          <span class="status-label">{{ statusLabel }}</span>
        </div>
      </div>
    </aside>

    <!-- Main -->
    <div class="main-wrapper">
      <!-- Top bar with search -->
      <header class="topbar">
        <button class="search-trigger" @click="openCommandPalette">
          <svg class="search-icon" viewBox="0 0 16 16" fill="currentColor" width="14" height="14"><path d="M10.68 11.74a6 6 0 0 1-7.922-8.982 6 6 0 0 1 8.982 7.922l3.04 3.04a.749.749 0 1 1-1.06 1.06l-3.04-3.04zM11.5 7a4.499 4.499 0 1 0-8.997 0A4.499 4.499 0 0 0 11.5 7z"/></svg>
          <span>Search sessions...</span>
          <kbd>/</kbd>
        </button>
      </header>

      <main class="main-content">
        <router-view />
      </main>
    </div>

    <!-- Command palette overlay -->
    <Teleport to="body">
      <div v-if="showPalette" class="palette-overlay" @click.self="closePalette">
        <div class="palette-box">
          <div class="palette-input-row">
            <svg viewBox="0 0 16 16" fill="currentColor" width="16" height="16" class="palette-search-icon"><path d="M10.68 11.74a6 6 0 0 1-7.922-8.982 6 6 0 0 1 8.982 7.922l3.04 3.04a.749.749 0 1 1-1.06 1.06l-3.04-3.04zM11.5 7a4.499 4.499 0 1 0-8.997 0A4.499 4.499 0 0 0 11.5 7z"/></svg>
            <input
              ref="paletteInput"
              v-model="paletteQuery"
              placeholder="Search sessions, projects, content..."
              class="palette-input"
              @keydown.enter="onPaletteSearch"
              @keydown.escape="closePalette"
            />
          </div>
          <div class="palette-hint">Press Enter to search, Esc to close</div>
        </div>
      </div>
    </Teleport>

    <NotificationContainer />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import NotificationContainer from '@/components/NotificationContainer.vue'
import { useNotifications } from '@/composables/useNotifications'
import { api } from '@/api/client'

const router = useRouter()
const route = useRoute()
const { connectWs, disconnectWs } = useNotifications()

const pendingReviewCount = ref(0)
const indexerStatus = ref('idle')
const showPalette = ref(false)
const paletteQuery = ref('')
const paletteInput = ref<HTMLInputElement>()

let pollTimer: ReturnType<typeof setInterval>

function isActive(path: string): boolean {
  if (path === '/') return route.path === '/'
  return route.path.startsWith(path)
}

const statusClass = computed(() => {
  if (indexerStatus.value === 'running') return 'status-running'
  if (indexerStatus.value === 'paused') return 'status-paused'
  return 'status-idle'
})

const statusLabel = computed(() => {
  if (indexerStatus.value === 'running') return 'Indexing...'
  if (indexerStatus.value === 'paused') return 'Paused'
  return 'Idle'
})

function openCommandPalette() {
  showPalette.value = true
  paletteQuery.value = ''
  setTimeout(() => paletteInput.value?.focus(), 50)
}

function closePalette() {
  showPalette.value = false
}

function onPaletteSearch() {
  const q = paletteQuery.value.trim()
  if (!q) return
  closePalette()
  router.push({ path: '/search', query: { q } })
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === '/' && !isInputFocused()) {
    e.preventDefault()
    openCommandPalette()
  }
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault()
    openCommandPalette()
  }
}

function isInputFocused(): boolean {
  const el = document.activeElement
  if (!el) return false
  const tag = el.tagName.toLowerCase()
  return tag === 'input' || tag === 'textarea' || tag === 'select' || (el as HTMLElement).isContentEditable
}

async function pollStatus() {
  try {
    const data = await api.enrichment.pendingCount()
    pendingReviewCount.value = data.count
  } catch { /* ignore */ }
  try {
    const status = await api.indexer.status()
    indexerStatus.value = status.status
  } catch { /* ignore */ }
}

onMounted(() => {
  connectWs()
  pollStatus()
  pollTimer = setInterval(pollStatus, 15_000)
  document.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  disconnectWs()
  clearInterval(pollTimer)
  document.removeEventListener('keydown', onKeydown)
})
</script>

<style>
.app-layout {
  display: flex;
  min-height: 100vh;
  background: var(--bl-bg);
}

/* ── Sidebar ── */
.sidebar {
  width: 200px;
  flex-shrink: 0;
  background: var(--bl-sidebar-bg);
  display: flex;
  flex-direction: column;
  border-right: 1px solid rgba(255, 255, 255, 0.06);
  position: sticky;
  top: 0;
  height: 100vh;
  overflow-y: auto;
}

.sidebar-brand {
  padding: 1.25rem 1rem;
  display: flex;
  align-items: center;
  gap: 0.625rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.brand-mark {
  width: 28px;
  height: 28px;
  background: var(--bl-c5);
  color: var(--bl-c1);
  font-weight: 800;
  font-size: 0.75rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--bl-radius-md);
}

.brand-text {
  font-weight: 700;
  font-size: var(--bl-text-md);
  color: rgba(255, 255, 255, 0.9);
  letter-spacing: 0.02em;
}

.sidebar-nav {
  flex: 1;
  padding: 0.5rem;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  padding: 0.5rem 0.75rem;
  border-radius: var(--bl-radius-md);
  color: var(--bl-sidebar-text);
  text-decoration: none;
  font-size: var(--bl-text-sm);
  font-weight: 500;
  transition: background 0.15s, color 0.15s;
  position: relative;
}

.nav-item:hover {
  background: var(--bl-sidebar-hover);
  color: rgba(255, 255, 255, 0.9);
  opacity: 1;
}

.nav-item.active {
  background: var(--bl-sidebar-active);
  color: var(--bl-c5);
  font-weight: 600;
}

.nav-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
  opacity: 0.7;
}

.nav-item.active .nav-icon {
  opacity: 1;
}

.nav-badge {
  margin-left: auto;
  background: var(--bl-c5);
  color: var(--bl-c1);
  font-size: 0.625rem;
  font-weight: 700;
  min-width: 18px;
  height: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--bl-radius-pill);
  padding: 0 5px;
}

.sidebar-footer {
  padding: 0.75rem 1rem;
  border-top: 1px solid rgba(255, 255, 255, 0.06);
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: var(--bl-text-xs);
  color: var(--bl-sidebar-text);
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--bl-gray-500);
}

.status-running .status-dot {
  background: var(--bl-success);
  box-shadow: 0 0 6px rgba(63, 185, 80, 0.5);
  animation: pulse-dot 2s ease-in-out infinite;
}

.status-paused .status-dot {
  background: var(--bl-warning);
}

@keyframes pulse-dot {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

/* ── Main wrapper ── */
.main-wrapper {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.topbar {
  padding: 0.75rem 1.5rem;
  border-bottom: 1px solid var(--bl-border);
  background: var(--bl-surface);
  position: sticky;
  top: 0;
  z-index: 100;
}

.search-trigger {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  max-width: 480px;
  padding: 0.5rem 0.75rem;
  background: var(--bl-surface-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  color: var(--bl-text-2);
  font-size: var(--bl-text-sm);
  cursor: pointer;
  transition: border-color 0.15s;
}

.search-trigger:hover {
  border-color: var(--bl-text-3);
}

.search-trigger kbd {
  margin-left: auto;
  background: var(--bl-surface-3);
  border: 1px solid var(--bl-border);
  border-radius: 3px;
  padding: 1px 6px;
  font-size: var(--bl-text-xs);
  font-family: var(--bl-font-sans);
  color: var(--bl-text-2);
}

.search-icon {
  opacity: 0.5;
}

.main-content {
  flex: 1;
  padding: 1.5rem;
  max-width: 1400px;
  width: 100%;
}

/* ── Command Palette ── */
.palette-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  z-index: 10000;
  display: flex;
  justify-content: center;
  padding-top: 20vh;
}

.palette-box {
  width: 560px;
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-xl);
  box-shadow: var(--bl-shadow-lg);
  overflow: hidden;
}

.palette-input-row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 1rem 1.25rem;
}

.palette-search-icon {
  color: var(--bl-text-2);
  flex-shrink: 0;
}

.palette-input {
  flex: 1;
  background: none;
  border: none;
  outline: none;
  font-size: var(--bl-text-lg);
  color: var(--bl-text);
}

.palette-input::placeholder {
  color: var(--bl-text-3);
}

.palette-hint {
  padding: 0.5rem 1.25rem;
  font-size: var(--bl-text-xs);
  color: var(--bl-text-3);
  border-top: 1px solid var(--bl-border);
}

/* ── Responsive ── */
@media (max-width: 768px) {
  .sidebar {
    width: 60px;
  }
  .brand-text,
  .nav-item span:not(.nav-badge),
  .status-label {
    display: none;
  }
  .sidebar-brand {
    justify-content: center;
  }
  .nav-item {
    justify-content: center;
    padding: 0.625rem;
  }
  .main-content {
    padding: 1rem;
  }
}
</style>
