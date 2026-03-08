<template>
  <div class="daily-chart">
    <canvas ref="chartRef"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import {
  Chart, LineController, LineElement, PointElement,
  LinearScale, CategoryScale, Filler, Tooltip, Legend,
} from 'chart.js'
import type { DailyStats } from '@/types'

Chart.register(LineController, LineElement, PointElement, LinearScale, CategoryScale, Filler, Tooltip, Legend)

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
          borderColor: '#5dcccb',
          backgroundColor: 'rgba(93, 204, 203, 0.08)',
          fill: true, tension: 0.3, pointRadius: 0, borderWidth: 1.5,
        },
        {
          label: 'Sessions',
          data: props.data.map(d => d.session_count ?? 0),
          borderColor: '#3fb950',
          backgroundColor: 'rgba(63, 185, 80, 0.05)',
          fill: true, tension: 0.3, pointRadius: 0, borderWidth: 1.5,
        },
        {
          label: 'Tool Calls',
          data: props.data.map(d => d.tool_call_count ?? 0),
          borderColor: '#bc8cff',
          backgroundColor: 'rgba(188, 140, 255, 0.05)',
          fill: true, tension: 0.3, pointRadius: 0, borderWidth: 1.5,
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: { mode: 'index', intersect: false },
      plugins: {
        legend: {
          labels: { color: '#8b949e', font: { size: 11 }, boxWidth: 12, padding: 16 },
        },
        tooltip: {
          backgroundColor: '#1c2128',
          borderColor: '#2d333b',
          borderWidth: 1,
          titleColor: '#e6edf3',
          bodyColor: '#8b949e',
          padding: 10,
        },
      },
      scales: {
        x: {
          ticks: { color: '#505860', maxTicksLimit: 12, font: { size: 10 } },
          grid: { color: 'rgba(45, 51, 59, 0.5)', lineWidth: 0.5 },
        },
        y: {
          ticks: { color: '#505860', font: { size: 10 } },
          grid: { color: 'rgba(45, 51, 59, 0.5)', lineWidth: 0.5 },
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
  height: 280px;
}
</style>
