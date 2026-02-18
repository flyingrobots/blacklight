<template>
  <div class="daily-chart">
    <canvas ref="chartRef"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import {
  Chart,
  LineController,
  LineElement,
  PointElement,
  LinearScale,
  CategoryScale,
  Filler,
  Tooltip,
  Legend,
} from 'chart.js'
import type { DailyStats } from '@/types'

Chart.register(
  LineController,
  LineElement,
  PointElement,
  LinearScale,
  CategoryScale,
  Filler,
  Tooltip,
  Legend,
)

const props = defineProps<{ data: DailyStats[] }>()
const chartRef = ref<HTMLCanvasElement | null>(null)
let chart: Chart | null = null

function renderChart() {
  if (!chartRef.value || !props.data.length) return
  if (chart) chart.destroy()

  const labels = props.data.map(d => d.date)
  chart = new Chart(chartRef.value, {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'Messages',
          data: props.data.map(d => d.message_count ?? 0),
          borderColor: '#58a6ff',
          backgroundColor: 'rgba(88, 166, 255, 0.1)',
          fill: true,
          tension: 0.3,
          pointRadius: 0,
        },
        {
          label: 'Sessions',
          data: props.data.map(d => d.session_count ?? 0),
          borderColor: '#3fb950',
          backgroundColor: 'rgba(63, 185, 80, 0.1)',
          fill: true,
          tension: 0.3,
          pointRadius: 0,
        },
        {
          label: 'Tool Calls',
          data: props.data.map(d => d.tool_call_count ?? 0),
          borderColor: '#bc8cff',
          backgroundColor: 'rgba(188, 140, 255, 0.1)',
          fill: true,
          tension: 0.3,
          pointRadius: 0,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: 'index', intersect: false },
      plugins: {
        legend: {
          labels: { color: '#8b949e', font: { size: 12 } },
        },
      },
      scales: {
        x: {
          ticks: { color: '#8b949e', maxTicksLimit: 15, font: { size: 11 } },
          grid: { color: 'rgba(48, 54, 61, 0.5)' },
        },
        y: {
          ticks: { color: '#8b949e', font: { size: 11 } },
          grid: { color: 'rgba(48, 54, 61, 0.5)' },
        },
      },
    },
  })
}

onMounted(renderChart)
watch(() => props.data, renderChart)
</script>

<style scoped>
.daily-chart {
  height: 300px;
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-lg);
  padding: 1rem;
}
</style>
