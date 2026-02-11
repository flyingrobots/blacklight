<template>
  <div class="storage-view">
    <h2>Storage</h2>

    <div v-if="loading" class="loading">Loading...</div>
    <div v-else-if="error" class="error">{{ error }}</div>
    <template v-else-if="storage">
      <div class="stats-grid">
        <div class="stat-card">
          <div class="stat-value">{{ storage.unique_blobs.toLocaleString() }}</div>
          <div class="stat-label">Unique Blobs</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ formatBytes(storage.total_bytes) }}</div>
          <div class="stat-label">Total Content</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ storage.total_references.toLocaleString() }}</div>
          <div class="stat-label">Total References</div>
        </div>
        <div class="stat-card">
          <div class="stat-value">{{ (storage.dedup_ratio * 100).toFixed(1) }}%</div>
          <div class="stat-label">Dedup Ratio</div>
        </div>
      </div>

      <div class="section" v-if="storage.by_kind.length">
        <h3>Content by Kind</h3>
        <table class="data-table">
          <thead>
            <tr>
              <th>Kind</th>
              <th class="right">Blob Count</th>
              <th class="right">Total Size</th>
              <th class="right">% of Total</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="k in storage.by_kind" :key="k.kind">
              <td><code>{{ k.kind }}</code></td>
              <td class="right">{{ k.blob_count.toLocaleString() }}</td>
              <td class="right">{{ formatBytes(k.total_bytes) }}</td>
              <td class="right">{{ ((k.total_bytes / storage.total_bytes) * 100).toFixed(1) }}%</td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api/client'
import type { StorageOverview } from '@/types'

const loading = ref(true)
const error = ref('')
const storage = ref<StorageOverview | null>(null)

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i]
}

onMounted(async () => {
  try {
    storage.value = await api.storage()
  } catch (e: any) {
    error.value = e.message
  } finally {
    loading.value = false
  }
})
</script>

<style scoped>
.storage-view h2 { margin-bottom: 1.5rem; }
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 1rem;
  margin-bottom: 2rem;
}
.stat-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1.25rem;
}
.stat-value { font-size: 1.5rem; font-weight: 600; color: var(--accent); }
.stat-label { color: var(--text-secondary); font-size: 0.875rem; margin-top: 0.25rem; }
.section { margin-bottom: 2rem; }
.section h3 { margin-bottom: 1rem; color: var(--text-secondary); }
.data-table { width: 100%; border-collapse: collapse; font-size: 0.875rem; }
.data-table th, .data-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--border);
  text-align: left;
}
.data-table th { color: var(--text-secondary); font-weight: 500; }
.data-table .right { text-align: right; }
.loading, .error { padding: 2rem; text-align: center; }
.error { color: var(--danger); }
</style>
