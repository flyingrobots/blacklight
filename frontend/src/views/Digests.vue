<template>
  <div class="digests-view">
    <div class="header">
      <h1>Weekly Decision Digests</h1>
      <button class="btn btn-primary" @click="generateLatest" :disabled="generating">
        {{ generating ? 'Generating...' : 'Generate Latest Digest' }}
      </button>
    </div>

    <div v-if="loading && digests.length === 0" class="loading-state">
      <div class="spinner"></div>
    </div>

    <div v-else class="digests-list">
      <article v-for="digest in digests" :key="digest.id" class="digest-card">
        <header class="dc-header">
          <div class="dc-title">
            Week of {{ fmtDate(digest.start_date) }} – {{ fmtDate(digest.end_date) }}
          </div>
          <div class="dc-meta">Created {{ fmtTime(digest.created_at) }}</div>
        </header>

        <div class="dc-stats">
          <div class="stat">
            <span class="val">{{ digest.session_count }}</span>
            <span class="lab">Sessions</span>
          </div>
          <div class="stat success">
            <span class="val">{{ digest.success_count }}</span>
            <span class="lab">Success</span>
          </div>
          <div class="stat failed">
            <span class="val">{{ digest.failed_count }}</span>
            <span class="lab">Failed</span>
          </div>
          <div class="stat partial">
            <span class="val">{{ digest.partial_count }}</span>
            <span class="lab">Partial</span>
          </div>
          <div class="stat">
            <span class="val">{{ digest.message_count.toLocaleString() }}</span>
            <span class="lab">Messages</span>
          </div>
        </div>

        <div class="dc-content markdown-body" v-html="renderMarkdown(digest.content)"></div>
      </article>

      <div v-if="digests.length === 0" class="empty-state">
        No digests generated yet. Click the button above to analyze your recent work.
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api/client'
import type { WeeklyDigest } from '@/types'
import { marked } from 'marked'
import DOMPurify from 'dompurify'

const digests = ref<WeeklyDigest[]>([])
const loading = ref(true)
const generating = ref(false)

async function fetchDigests() {
  try {
    const result = await api.digests.list({ limit: 10 })
    digests.value = result
  } catch (e) {
    console.error('Failed to fetch digests:', e)
  } finally {
    loading.value = false
  }
}

async function generateLatest() {
  generating.value = true
  try {
    // Determine last 7 days
    const end = new Date()
    const start = new Date()
    start.setDate(end.getDate() - 7)
    
    await api.digests.generate({
      start_date: start.toISOString(),
      end_date: end.toISOString()
    })
    await fetchDigests()
  } catch (e: any) {
    alert('Failed to generate digest: ' + e.message)
  } finally {
    generating.value = false
  }
}

function renderMarkdown(content: string) {
  const html = marked.parse(content) as string
  return DOMPurify.sanitize(html)
}

function fmtDate(iso: string) {
  return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })
}

function fmtTime(iso: string) {
  return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' })
}

onMounted(fetchDigests)
</script>

<style scoped>
.digests-view {
  max-width: 800px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header h1 { font-size: var(--bl-text-xl); }

.digests-list {
  display: flex;
  flex-direction: column;
  gap: 2.5rem;
}

.digest-card {
  background: var(--bl-surface);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  overflow: hidden;
}

.dc-header {
  padding: 1.25rem 1.5rem;
  border-bottom: 1px solid var(--bl-border-2);
  background: var(--bl-surface-2);
}

.dc-title {
  font-size: var(--bl-text-lg);
  font-weight: 600;
  color: var(--bl-text);
  margin-bottom: 0.25rem;
}

.dc-meta {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-3);
}

.dc-stats {
  display: flex;
  gap: 2rem;
  padding: 1rem 1.5rem;
  border-bottom: 1px solid var(--bl-border-2);
  background: var(--bl-surface);
}

.stat {
  display: flex;
  flex-direction: column;
}

.stat .val {
  font-size: var(--bl-text-md);
  font-weight: 700;
  font-family: var(--bl-font-mono);
}

.stat .lab {
  font-size: var(--bl-text-2xs);
  color: var(--bl-text-3);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.stat.success .val { color: var(--bl-success); }
.stat.failed .val { color: var(--bl-danger); }
.stat.partial .val { color: var(--bl-warning); }

.dc-content {
  padding: 1.5rem;
  line-height: 1.6;
}

/* Markdown adjustments */
:deep(.markdown-body h1), :deep(.markdown-body h2) {
  margin-top: 1.5rem;
  margin-bottom: 1rem;
  font-size: var(--bl-text-md);
  border-bottom: 1px solid var(--bl-border-2);
  padding-bottom: 0.3rem;
}

:deep(.markdown-body p) { margin-bottom: 1rem; }
:deep(.markdown-body ul) { margin-bottom: 1rem; padding-left: 1.5rem; }
:deep(.markdown-body li) { margin-bottom: 0.25rem; }

.loading-state {
  display: flex;
  justify-content: center;
  padding: 4rem;
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--bl-border);
  border-top-color: var(--bl-accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }

.empty-state {
  text-align: center;
  padding: 4rem;
  color: var(--bl-text-3);
  background: var(--bl-surface-2);
  border-radius: var(--bl-radius-lg);
  border: 2px dashed var(--bl-border);
}
</style>
