<template>
  <div class="sessions-view">
    <div class="sessions-header">
      <h1>Sessions</h1>
      <span class="total-count" v-if="total">{{ total }} total</span>
    </div>

    <div class="filters">
      <input
        v-model="projectFilter"
        placeholder="Filter by project..."
        class="filter-input"
        @input="debouncedFetch"
      />
      <input v-model="fromDate" type="date" class="date-input" @change="fetchSessions" />
      <input v-model="toDate" type="date" class="date-input" @change="fetchSessions" />
      <select v-model="outcomeFilter" class="select-sm" @change="fetchSessions">
        <option value="">All Outcomes</option>
        <option value="success">Success</option>
        <option value="partial">Partial</option>
        <option value="failed">Failed</option>
        <option value="abandoned">Abandoned</option>
      </select>
    </div>

    <div v-if="loading" class="loading-state"><div class="spinner"></div></div>

    <template v-else>
      <div v-if="sessions.length" class="session-list">
        <SessionCard
          v-for="session in sessions"
          :key="session.id"
          :session="session"
        />
      </div>
      <div v-else class="empty-state">No sessions found.</div>

      <div class="pagination" v-if="total > limit">
        <button :disabled="offset === 0" @click="prevPage" class="page-btn">Previous</button>
        <span class="page-info">{{ currentPage }} / {{ totalPages }}</span>
        <button :disabled="offset + limit >= total" @click="nextPage" class="page-btn">Next</button>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api/client'
import type { SessionSummary } from '@/types'
import SessionCard from '@/components/SessionCard.vue'

const route = useRoute()
const sessions = ref<SessionSummary[]>([])
const total = ref(0)
const limit = ref(30)
const offset = ref(0)
const loading = ref(true)
const projectFilter = ref('')
const fromDate = ref('')
const toDate = ref('')
const outcomeFilter = ref('')

let debounceTimer: ReturnType<typeof setTimeout>
const currentPage = computed(() => Math.floor(offset.value / limit.value) + 1)
const totalPages = computed(() => Math.ceil(total.value / limit.value))

function debouncedFetch() {
  clearTimeout(debounceTimer)
  debounceTimer = setTimeout(fetchSessions, 300)
}

async function fetchSessions() {
  loading.value = true
  try {
    const result = await api.sessions.list({
      project: projectFilter.value || undefined,
      from: fromDate.value || undefined,
      to: toDate.value || undefined,
      limit: limit.value,
      offset: offset.value,
      outcome: outcomeFilter.value || undefined,
    })
    sessions.value = result.items
    total.value = result.total
  } catch (e: any) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

function prevPage() { offset.value = Math.max(0, offset.value - limit.value); fetchSessions() }
function nextPage() { offset.value += limit.value; fetchSessions() }

onMounted(() => {
  const q = route.query.project
  if (q && typeof q === 'string') projectFilter.value = q
  fetchSessions()
})
</script>

<style scoped>
.sessions-view {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.sessions-header {
  display: flex;
  align-items: baseline;
  gap: 0.75rem;
}

.sessions-header h1 {
  font-size: var(--bl-text-xl);
}

.total-count {
  font-size: var(--bl-text-sm);
  color: var(--bl-text-2);
}

.filters {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.filter-input {
  flex: 0 1 260px;
  padding: 0.5rem 0.75rem;
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  font-size: var(--bl-text-sm);
  color: var(--bl-text);
  outline: none;
}

.filter-input:focus { border-color: var(--bl-accent); }

.date-input {
  padding: 0.5rem 0.625rem;
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  font-size: var(--bl-text-xs);
  color: var(--bl-text);
  color-scheme: dark;
}

.session-list {
  display: flex;
  flex-direction: column;
  gap: 1px;
  background: var(--bl-border);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  overflow: hidden;
}

.loading-state {
  display: flex;
  justify-content: center;
  padding: 3rem;
}

.spinner {
  width: 24px;
  height: 24px;
  border: 2px solid var(--bl-border);
  border-top-color: var(--bl-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.empty-state {
  text-align: center;
  padding: 3rem;
  color: var(--bl-text-2);
}

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  margin-top: 0.5rem;
}

.page-btn {
  padding: 0.375rem 0.875rem;
  background: var(--bl-surface-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  color: var(--bl-text);
  font-size: var(--bl-text-sm);
  cursor: pointer;
}

.page-btn:disabled { opacity: 0.3; cursor: not-allowed; }
.page-btn:hover:not(:disabled) { border-color: var(--bl-text-3); }

.page-info {
  font-size: var(--bl-text-sm);
  color: var(--bl-text-2);
}
</style>
