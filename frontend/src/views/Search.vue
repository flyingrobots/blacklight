<template>
  <div class="search-view">
    <h2>Search</h2>

    <div class="search-bar">
      <input
        v-model="query"
        placeholder="Search content..."
        class="input search-input"
        @keydown.enter="doSearch"
      />
      <select v-model="kindFilter" class="input">
        <option value="">All kinds</option>
        <option value="text">Text</option>
        <option value="tool_output">Tool Output</option>
        <option value="thinking">Thinking</option>
        <option value="tool_input">Tool Input</option>
      </select>
      <button @click="doSearch" class="btn primary">Search</button>
    </div>

    <div v-if="loading" class="loading">Searching...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else-if="hasSearched">
      <div class="meta">{{ total }} results</div>
      <div class="results">
        <SearchResult v-for="hit in results" :key="hit.hash + hit.message_id" :hit="hit" />
      </div>
      <div v-if="!results.length" class="empty">No results found</div>
      <div class="pagination" v-if="total > limit">
        <button :disabled="offset === 0" @click="prevPage" class="btn">Previous</button>
        <span class="page-info">Page {{ currentPage }} of {{ totalPages }}</span>
        <button :disabled="offset + limit >= total" @click="nextPage" class="btn">Next</button>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { api } from '@/api/client'
import type { SearchHit } from '@/types'
import SearchResult from '@/components/SearchResult.vue'

const query = ref('')
const kindFilter = ref('')
const results = ref<SearchHit[]>([])
const total = ref(0)
const limit = ref(20)
const offset = ref(0)
const loading = ref(false)
const error = ref('')
const hasSearched = ref(false)

const currentPage = computed(() => Math.floor(offset.value / limit.value) + 1)
const totalPages = computed(() => Math.ceil(total.value / limit.value))

async function doSearch() {
  if (!query.value.trim()) return
  loading.value = true
  error.value = ''
  hasSearched.value = true
  try {
    const result = await api.search({
      q: query.value,
      kind: kindFilter.value || undefined,
      limit: limit.value,
      offset: offset.value,
    })
    results.value = result.items
    total.value = result.total
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

function prevPage() {
  offset.value = Math.max(0, offset.value - limit.value)
  doSearch()
}

function nextPage() {
  offset.value += limit.value
  doSearch()
}
</script>

<style scoped>
.search-view h2 { margin-bottom: 1.5rem; }
.search-bar {
  display: flex;
  gap: 0.75rem;
  margin-bottom: 1.5rem;
  flex-wrap: wrap;
}
.search-input { flex: 1; min-width: 200px; }
.input {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
  color: var(--text);
  font-size: 0.875rem;
}
.input:focus { outline: none; border-color: var(--accent); }
.btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.5rem 1rem;
  color: var(--text);
  cursor: pointer;
}
.btn.primary { background: #1a3a5c; border-color: var(--accent); }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
.btn:hover:not(:disabled) { border-color: var(--accent); }
.meta { color: var(--text-secondary); font-size: 0.875rem; margin-bottom: 1rem; }
.results { display: flex; flex-direction: column; gap: 0.75rem; }
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  margin-top: 1.5rem;
}
.page-info { color: var(--text-secondary); font-size: 0.875rem; }
.empty { color: var(--text-secondary); padding: 2rem; text-align: center; }
.loading, .error { padding: 2rem; text-align: center; }
.error { color: var(--danger); }
</style>
