<template>
  <div class="heatmap-container" ref="containerRef">
    <svg ref="svgRef"></svg>
    <div v-if="!data.length" class="empty-state">No activity data available</div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted } from 'vue'
import * as d3 from 'd3'
import type { DailyStats } from '@/types'

const props = defineProps<{
  data: DailyStats[]
}>()

const containerRef = ref<HTMLElement | null>(null)
const svgRef = ref<SVGSVGElement | null>(null)

function renderHeatmap() {
  if (!svgRef.value || !props.data.length) return

  const data = props.data.map(d => ({
    date: new Date(d.date),
    count: d.session_count || 0
  })).sort((a, b) => a.date.getTime() - b.date.getTime())

  const width = containerRef.value?.clientWidth || 800
  const cellSize = 12
  const cellPadding = 2
  const height = (cellSize + cellPadding) * 7 + 20 // 7 days of week + labels

  const svg = d3.select(svgRef.value)
  svg.selectAll('*').remove()

  svg.attr('width', width)
     .attr('height', height)

  const colorScale = d3.scaleThreshold<number, string>()
    .domain([1, 2, 5, 10])
    .range(['var(--bl-bg-3)', '#0e4429', '#006d32', '#26a641', '#39d353'])

  const dayFormat = d3.timeFormat('%w')
  const weekFormat = d3.timeFormat('%U')
  const monthFormat = d3.timeFormat('%b')

  // Group by week
  const firstDate = data[0].date
  const lastDate = data[data.length - 1].date
  
  const g = svg.append('g').attr('transform', 'translate(30, 20)')

  // Month labels
  const months = d3.timeMonths(d3.timeMonth.offset(firstDate, -1), lastDate)
  g.selectAll('.month-label')
    .data(months)
    .enter()
    .append('text')
    .attr('class', 'month-label')
    .attr('x', d => {
      const weekDiff = d3.timeWeek.count(firstDate, d)
      return weekDiff * (cellSize + cellPadding)
    })
    .attr('y', -5)
    .text(d => monthFormat(d))
    .style('font-size', '10px')
    .style('fill', 'var(--bl-text-3)')
    .style('font-family', 'var(--bl-font-mono)')

  // Day labels
  const days = ['M', 'W', 'F']
  g.selectAll('.day-label')
    .data(days)
    .enter()
    .append('text')
    .attr('class', 'day-label')
    .attr('x', -15)
    .attr('y', (d, i) => (i * 2 + 1) * (cellSize + cellPadding) + cellSize / 2 + 4)
    .text(d => d)
    .style('font-size', '10px')
    .style('fill', 'var(--bl-text-3)')
    .style('font-family', 'var(--bl-font-mono)')

  // Cells
  g.selectAll('.day')
    .data(data)
    .enter()
    .append('rect')
    .attr('class', 'day')
    .attr('width', cellSize)
    .attr('height', cellSize)
    .attr('x', d => {
      const weekDiff = d3.timeWeek.count(firstDate, d.date)
      return weekDiff * (cellSize + cellPadding)
    })
    .attr('y', d => d.date.getDay() * (cellSize + cellPadding))
    .attr('rx', 2)
    .attr('ry', 2)
    .attr('fill', d => colorScale(d.count))
    .append('title')
    .text(d => `${d.date.toDateString()}: ${d.count} sessions`)
}

onMounted(() => {
  renderHeatmap()
  window.addEventListener('resize', renderHeatmap)
})

onUnmounted(() => {
  window.removeEventListener('resize', renderHeatmap)
})

watch(() => props.data, renderHeatmap, { deep: true })
</script>

<style scoped>
.heatmap-container {
  width: 100%;
  display: flex;
  justify-content: center;
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-sm);
  padding: 1.5rem 1rem;
}
svg {
  max-width: 100%;
}
.empty-state {
  padding: 2rem;
  text-align: center;
  color: var(--bl-text-3);
  font-family: var(--bl-font-mono);
  font-size: var(--bl-text-xs);
}
</style>
