<template>
  <div class="time-slider">
    <div class="slider-header">
      <span class="label">Time Window</span>
      <span class="value">{{ activeLabel }}</span>
    </div>
    <input
      type="range"
      min="0"
      :max="options.length - 1"
      step="1"
      v-model="selectedIndex"
      class="range-input"
    />
    <div class="ticks">
      <span v-for="(opt, i) in options" :key="opt.label" :class="{ active: i === Number(selectedIndex) }">
        {{ opt.shortLabel || opt.label }}
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'

export interface TimeOption {
  label: string
  shortLabel?: string
  days: number | null // null for 'All Time'
}

const options: TimeOption[] = [
  { label: 'Last 7 Days', shortLabel: '7d', days: 7 },
  { label: 'Last 30 Days', shortLabel: '30d', days: 30 },
  { label: 'Last 90 Days', shortLabel: '90d', days: 90 },
  { label: 'Last Year', shortLabel: '1y', days: 365 },
  { label: 'All Time', shortLabel: 'All', days: null },
]

const selectedIndex = ref(4) // Default to All Time

const activeLabel = computed(() => options[Number(selectedIndex.value)].label)

const emit = defineEmits<{
  (e: 'change', value: TimeOption): void
}>()

watch(selectedIndex, (newVal) => {
  emit('change', options[Number(newVal)])
})
</script>

<style scoped>
.time-slider {
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-sm);
  padding: 1rem;
  font-family: var(--bl-font-mono);
}
.slider-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 0.75rem;
}
.label {
  font-size: var(--bl-text-xs);
  text-transform: uppercase;
  color: var(--bl-text-2);
}
.value {
  font-size: var(--bl-text-sm);
  color: var(--bl-accent);
  font-weight: bold;
}
.range-input {
  width: 100%;
  appearance: none;
  background: var(--bl-bg-3);
  height: 4px;
  border-radius: 2px;
  outline: none;
  margin-bottom: 0.5rem;
}
.range-input::-webkit-slider-thumb {
  appearance: none;
  width: 12px;
  height: 12px;
  background: var(--bl-accent);
  border-radius: 50%;
  cursor: pointer;
}
.ticks {
  display: flex;
  justify-content: space-between;
}
.ticks span {
  font-size: 10px;
  color: var(--bl-text-3);
}
.ticks span.active {
  color: var(--bl-text);
  font-weight: bold;
}
</style>
