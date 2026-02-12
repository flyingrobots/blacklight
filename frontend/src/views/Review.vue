<template>
  <div class="review-page">
    <div class="review-header">
      <h1>Review Enrichments</h1>
      <div class="review-actions">
        <span v-if="total > 0" class="pending-count">{{ total }} pending</span>
        <button
          v-if="items.length > 0"
          class="btn btn-primary"
          @click="approveAll"
        >Approve All</button>
      </div>
    </div>

    <div v-if="loading" class="review-loading">Loading...</div>

    <div v-else-if="items.length === 0" class="review-empty">
      No enrichments pending review.
    </div>

    <div v-else class="review-list">
      <div
        v-for="item in items"
        :key="item.session_id"
        class="review-card"
      >
        <div class="review-card-header">
          <div class="review-card-title">
            <router-link :to="`/sessions/${item.session_id}`" class="session-link">
              {{ item.title }}
            </router-link>
            <span class="review-project">{{ item.project_slug }}</span>
          </div>
          <div class="review-card-actions">
            <button class="btn btn-approve" @click="approve(item.session_id)">Approve</button>
            <button class="btn btn-reject" @click="reject(item.session_id)">Reject</button>
          </div>
        </div>

        <p class="review-summary">{{ item.summary }}</p>

        <div class="review-tags">
          <span
            v-for="tag in item.tags"
            :key="tag.tag"
            :class="['tag', { 'tag-low': tag.confidence < 0.80 }]"
          >
            {{ tag.tag }}
            <span class="tag-conf">{{ (tag.confidence * 100).toFixed(0) }}%</span>
          </span>
        </div>

        <div class="review-meta">
          <span v-if="item.first_prompt" class="review-prompt" :title="item.first_prompt">
            {{ truncate(item.first_prompt, 120) }}
          </span>
          <span class="review-date">{{ formatDate(item.enriched_at) }}</span>
        </div>
      </div>

      <!-- Pagination -->
      <div v-if="total > limit" class="review-pagination">
        <button
          class="btn btn-secondary"
          :disabled="offset === 0"
          @click="loadPage(offset - limit)"
        >Previous</button>
        <span class="page-info">{{ offset + 1 }}–{{ Math.min(offset + limit, total) }} of {{ total }}</span>
        <button
          class="btn btn-secondary"
          :disabled="offset + limit >= total"
          @click="loadPage(offset + limit)"
        >Next</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/api/client'
import type { ReviewItem } from '@/types'

const items = ref<ReviewItem[]>([])
const total = ref(0)
const limit = 20
const offset = ref(0)
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    const result = await api.review.list({ limit, offset: offset.value })
    items.value = result.items
    total.value = result.total
  } catch {
    // ignore
  } finally {
    loading.value = false
  }
}

async function loadPage(newOffset: number) {
  offset.value = Math.max(0, newOffset)
  await load()
}

async function approve(sessionId: string) {
  try {
    await api.review.approve(sessionId)
    items.value = items.value.filter(i => i.session_id !== sessionId)
    total.value = Math.max(0, total.value - 1)
  } catch { /* ignore */ }
}

async function reject(sessionId: string) {
  try {
    await api.review.reject(sessionId)
    items.value = items.value.filter(i => i.session_id !== sessionId)
    total.value = Math.max(0, total.value - 1)
  } catch { /* ignore */ }
}

async function approveAll() {
  try {
    await api.review.approveAll()
    items.value = []
    total.value = 0
  } catch { /* ignore */ }
}

function truncate(s: string, max: number) {
  return s.length <= max ? s : s.slice(0, max) + '...'
}

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })
}

onMounted(load)
</script>

<style scoped>
.review-page {
  max-width: 900px;
  margin: 0 auto;
}

.review-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 1.5rem;
}
.review-header h1 {
  font-size: 1.5rem;
  font-weight: 600;
}
.review-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}
.pending-count {
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.review-loading,
.review-empty {
  text-align: center;
  color: var(--text-secondary);
  padding: 3rem 0;
  font-size: 0.9375rem;
}

.review-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.review-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 1rem;
}
.review-card-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
}
.review-card-title {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}
.session-link {
  font-weight: 600;
  font-size: 1rem;
  color: var(--accent);
}
.session-link:hover {
  text-decoration: underline;
}
.review-project {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.review-card-actions {
  display: flex;
  gap: 0.375rem;
  flex-shrink: 0;
}

.review-summary {
  font-size: 0.875rem;
  color: var(--text);
  margin-bottom: 0.5rem;
  line-height: 1.4;
}

.review-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 0.375rem;
  margin-bottom: 0.5rem;
}
.tag {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  padding: 0.2rem 0.5rem;
  border-radius: 12px;
  font-size: 0.75rem;
  background: var(--bg-tertiary);
  color: var(--text);
}
.tag-low {
  border: 1px solid #d97706;
  color: #d97706;
}
.tag-conf {
  font-size: 0.6875rem;
  color: var(--text-secondary);
}

.review-meta {
  display: flex;
  justify-content: space-between;
  font-size: 0.75rem;
  color: var(--text-secondary);
}
.review-prompt {
  max-width: 70%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Pagination */
.review-pagination {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
  padding: 1rem 0;
}
.page-info {
  font-size: 0.8125rem;
  color: var(--text-secondary);
}

/* Buttons */
.btn {
  padding: 0.3rem 0.65rem;
  border: none;
  border-radius: 6px;
  font-size: 0.75rem;
  font-weight: 500;
  cursor: pointer;
}
.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.btn-primary { background: var(--accent); color: #fff; }
.btn-primary:hover:not(:disabled) { opacity: 0.9; }
.btn-secondary { background: var(--bg-tertiary); color: var(--text); }
.btn-secondary:hover:not(:disabled) { opacity: 0.85; }
.btn-approve { background: var(--success, #22c55e); color: #fff; }
.btn-approve:hover { opacity: 0.9; }
.btn-reject { background: var(--danger, #ef4444); color: #fff; }
.btn-reject:hover { opacity: 0.9; }
</style>
