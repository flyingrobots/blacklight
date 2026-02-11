<template>
  <div class="files-view">
    <h2>File Provenance</h2>

    <div class="search-bar">
      <input
        v-model="pathFilter"
        placeholder="Search file paths..."
        class="input search-input"
        @input="debouncedFetch"
      />
    </div>

    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else>
      <div class="meta">
        Showing {{ files.length }} of {{ total }} file references
      </div>
      <table class="data-table">
        <thead>
          <tr>
            <th>File Path</th>
            <th>Operation</th>
            <th>Session</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="f in files" :key="f.file_path + f.message_id">
            <td><code>{{ f.file_path }}</code></td>
            <td>
              <span class="op-badge" :class="f.operation.toLowerCase()">{{ f.operation }}</span>
            </td>
            <td>
              <router-link :to="`/sessions/${f.session_id}`">
                {{ f.session_id.slice(0, 8) }}...
              </router-link>
            </td>
          </tr>
        </tbody>
      </table>
      <div class="pagination" v-if="total > limit">
        <button :disabled="offset === 0" @click="prevPage" class="btn">Previous</button>
        <span class="page-info">Page {{ currentPage }} of {{ totalPages }}</span>
        <button :disabled="offset + limit >= total" @click="nextPage" class="btn">Next</button>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { api } from '@/api/client'
import type { FileReference } from '@/types'

const files = ref<FileReference[]>([])
const total = ref(0)
const limit = ref(50)
const offset = ref(0)
const loading = ref(true)
const error = ref('')
const pathFilter = ref('')

let debounceTimer: ReturnType<typeof setTimeout>

const currentPage = computed(() => Math.floor(offset.value / limit.value) + 1)
const totalPages = computed(() => Math.ceil(total.value / limit.value))

function debouncedFetch() {
  clearTimeout(debounceTimer)
  debounceTimer = setTimeout(fetchFiles, 300)
}

async function fetchFiles() {
  loading.value = true
  error.value = ''
  try {
    const result = await api.files({
      path: pathFilter.value || undefined,
      limit: limit.value,
      offset: offset.value,
    })
    files.value = result.items
    total.value = result.total
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
}

function prevPage() {
  offset.value = Math.max(0, offset.value - limit.value)
  fetchFiles()
}

function nextPage() {
  offset.value += limit.value
  fetchFiles()
}

onMounted(fetchFiles)
</script>

<style scoped>
.files-view h2 { margin-bottom: 1.5rem; }
.search-bar { margin-bottom: 1rem; }
.search-input { width: 100%; max-width: 400px; }
.input {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
  color: var(--text);
  font-size: 0.875rem;
}
.input:focus { outline: none; border-color: var(--accent); }
.meta { color: var(--text-secondary); font-size: 0.875rem; margin-bottom: 1rem; }
.data-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
.data-table th, .data-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--border);
  text-align: left;
}
.data-table th { color: var(--text-secondary); font-weight: 500; }
.data-table code { font-size: 0.8125rem; }
.op-badge {
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: uppercase;
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
}
.op-badge.read { background: #1a3a2a; color: var(--success); }
.op-badge.write { background: #3a2a1a; color: var(--warning); }
.op-badge.edit { background: #1a2a3a; color: var(--accent); }
.pagination {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  margin-top: 1.5rem;
}
.page-info { color: var(--text-secondary); font-size: 0.875rem; }
.btn {
  background: var(--bg-tertiary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.5rem 1rem;
  color: var(--text);
  cursor: pointer;
}
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
.btn:hover:not(:disabled) { border-color: var(--accent); }
.loading, .error { padding: 2rem; text-align: center; }
.error { color: var(--danger); }
</style>
