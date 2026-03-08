<template>
  <div class="search-view">
    <div class="search-bar">
      <input
        ref="searchInput"
        v-model="query"
        placeholder="Search content..."
        class="search-input"
        @keydown.enter="doSearch"
      />
      <select v-model="kindFilter" class="filter-select" @change="doSearch">
        <option value="">All kinds</option>
        <option value="text">Text</option>
        <option value="tool_output">Tool Output</option>
        <option value="thinking">Thinking</option>
        <option value="tool_input">Tool Input</option>
      </select>
      <select v-model="projectFilter" class="filter-select" @change="doSearch">
        <option value="">All projects</option>
        <option v-for="p in projectList" :key="p" :value="p">{{ p }}</option>
      </select>
    </div>

    <div v-if="loading" class="search-loading">
      <div class="spinner"></div>
    </div>

    <template v-else-if="hasSearched">
      <div class="search-meta">{{ total }} results</div>

      <div v-if="results.length" class="results-list">
        <SearchResult v-for="hit in results" :key="hit.hash + hit.message_id" :hit="hit" />
      </div>
      <div v-else class="search-empty">No results found for "{{ query }}"</div>

      <div class="pagination" v-if="total > limit">
        <button :disabled="offset === 0" @click="prevPage" class="page-btn">Previous</button>
        <span class="page-info">{{ currentPage }} / {{ totalPages }}</span>
        <button :disabled="offset + limit >= total" @click="nextPage" class="page-btn">Next</button>
      </div>
    </template>

    <div v-else class="search-placeholder">
      <p>Search across all indexed session content.</p>
      <p class="hint">Supports full-text search with BM25 ranking.</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '@/api/client'
import type { SearchHit } from '@/types'
import SearchResult from '@/components/SearchResult.vue'

const route = useRoute()
const searchInput = ref<HTMLInputElement>()
const query = ref('')
const kindFilter = ref('')
const projectFilter = ref('')
const results = ref<SearchHit[]>([])
const total = ref(0)
const limit = ref(20)
const offset = ref(0)
const loading = ref(false)
const hasSearched = ref(false)
const projectList = ref<string[]>([])

const currentPage = computed(() => Math.floor(offset.value / limit.value) + 1)
const totalPages = computed(() => Math.ceil(total.value / limit.value))

async function doSearch() {
  if (!query.value.trim()) return
  loading.value = true
  hasSearched.value = true
  try {
    const result = await api.search({
      q: query.value,
      kind: kindFilter.value || undefined,
      project: projectFilter.value || undefined,
      limit: limit.value,
      offset: offset.value,
    })
    results.value = result.items
    total.value = result.total
  } catch (e: any) {
    console.error(e)
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

onMounted(async () => {
  // Load project list for filter
  try {
    const projects = await api.projects()
    projectList.value = projects.map(p => p.project_slug).sort()
  } catch { /* ignore */ }

  const q = route.query.q as string
  if (q) {
    query.value = q
    doSearch()
  } else {
    searchInput.value?.focus()
  }
})

watch(() => route.query.q, (q) => {
  if (q && typeof q === 'string' && q !== query.value) {
    query.value = q
    offset.value = 0
    doSearch()
  }
})
</script>

<style scoped>
.search-view {
  max-width: 900px;
}

.search-bar {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1.5rem;
}

.search-input {
  flex: 1;
  min-width: 200px;
  padding: 0.625rem 0.875rem;
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  font-size: var(--bl-text-md);
  color: var(--bl-text);
  outline: none;
}

.search-input:focus {
  border-color: var(--bl-accent);
}

.filter-select {
  padding: 0.5rem 0.75rem;
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  font-size: var(--bl-text-sm);
  color: var(--bl-text);
  cursor: pointer;
}

.search-meta {
  font-size: var(--bl-text-sm);
  color: var(--bl-text-2);
  margin-bottom: 0.75rem;
}

.results-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.search-empty, .search-placeholder {
  text-align: center;
  padding: 3rem;
  color: var(--bl-text-2);
}

.hint {
  font-size: var(--bl-text-sm);
  margin-top: 0.5rem;
  opacity: 0.6;
}

.search-loading {
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

.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  margin-top: 1.5rem;
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

.page-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.page-btn:hover:not(:disabled) {
  border-color: var(--bl-text-3);
}

.page-info {
  font-size: var(--bl-text-sm);
  color: var(--bl-text-2);
}
</style>
