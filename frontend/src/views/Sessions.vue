<template>
  <div class="sessions-view">
    <h2>Sessions</h2>

    <div class="filters">
      <input
        v-model="projectFilter"
        placeholder="Filter by project..."
        class="input search-input"
        @input="debouncedFetch"
      />
      <input
        v-model="fromDate"
        type="date"
        class="input"
        @change="fetchSessions"
      />
      <input
        v-model="toDate"
        type="date"
        class="input"
        @change="fetchSessions"
      />
    </div>

    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else>
      <div class="meta">
        Showing {{ sessions.length }} of {{ total }} sessions
        <span class="nav-hint">[j/k to navigate, enter to open]</span>
      </div>
      <div class="session-list">
        <SessionCard
          v-for="(session, index) in sessions"
          :key="session.id"
          :session="session"
          :data-nav-index="index"
          :class="{ 'selected': selectedIndex === index }"
        />
      </div>
      <div class="pagination" v-if="total > limit">
        <button :disabled="offset === 0" @click="prevPage" class="btn">Previous</button>
        <span class="page-info">Page {{ currentPage }} of {{ totalPages }}</span>
        <button :disabled="offset + limit >= total" @click="nextPage" class="btn">Next</button>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { api } from '@/api/client'
import type { SessionSummary } from '@/types'
import SessionCard from '@/components/SessionCard.vue'
import { useKeyboardNavigation } from '@/composables/useKeyboardNavigation'

const router = useRouter()
const sessions = ref<SessionSummary[]>([])
const total = ref(0)
const limit = ref(20)
const offset = ref(0)
const loading = ref(true)
const error = ref('')
const projectFilter = ref('')
const fromDate = ref('')
const toDate = ref('')

let debounceTimer: ReturnType<typeof setTimeout>

const currentPage = computed(() => Math.floor(offset.value / limit.value) + 1)
const totalPages = computed(() => Math.ceil(total.value / limit.value))

const { selectedIndex } = useKeyboardNavigation(
  computed(() => sessions.value.length) as any,
  (index) => {
    const session = sessions.value[index]
    if (session) {
      router.push(`/sessions/${session.id}`)
    }
  }
)

function debouncedFetch() {
  clearTimeout(debounceTimer)
  debounceTimer = setTimeout(fetchSessions, 300)
}

async function fetchSessions() {
  loading.value = true
  error.value = ''
  try {
    const result = await api.sessions.list({
      project: projectFilter.value || undefined,
      from: fromDate.value || undefined,
      to: toDate.value || undefined,
      limit: limit.value,
      offset: offset.value,
    })
    sessions.value = result.items
    total.value = result.total
    // Reset selection on fetch
    selectedIndex.value = -1
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

function prevPage() {
  offset.value = Math.max(0, offset.value - limit.value)
  fetchSessions()
}

function nextPage() {
  offset.value += limit.value
  fetchSessions()
}

onMounted(fetchSessions)
</script>

<style scoped>
.sessions-view h2 { margin-bottom: 1.5rem; }
.filters {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1rem;
  flex-wrap: wrap;
}
.input {
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  padding: 0.5rem 0.75rem;
  color: var(--bl-text);
  font-size: var(--bl-text-md);
  font-family: var(--bl-font-mono);
}
.input:focus { outline: none; border-color: var(--bl-accent); }
.meta { 
  color: var(--bl-text-2); 
  font-size: var(--bl-text-md); 
  margin-bottom: 1rem;
  display: flex;
  justify-content: space-between;
}
.nav-hint {
  font-family: var(--bl-font-mono);
  font-size: var(--bl-text-xs);
  opacity: 0.6;
}
.session-list { display: flex; flex-direction: column; gap: 0.75rem; }

/* Keyboard Selection Style */
.session-list :deep(.session-card.selected) {
  outline: none;
}

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  margin-top: 1.5rem;
}
.page-info { color: var(--bl-text-2); font-size: var(--bl-text-md); font-family: var(--bl-font-mono); }
.btn {
  background: var(--bl-bg-3);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  padding: 0.5rem 1rem;
  color: var(--bl-text);
  cursor: pointer;
  font-family: var(--bl-font-mono);
}
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
.btn:hover:not(:disabled) { border-color: var(--bl-accent); }
.loading, .error { padding: 2rem; text-align: center; }
.error { color: var(--bl-danger); }
</style>
