<template>
  <div class="heatmap-wrapper">
    <div class="heatmap-container" ref="containerRef">
      <svg ref="svgRef"></svg>
      <div v-if="!data.length" class="empty-state">No activity data</div>

      <div
        v-if="tooltip.visible"
        class="heatmap-tooltip"
        :style="{ left: tooltip.x + 'px', top: tooltip.y + 'px' }"
      >
        <div class="tooltip-date">{{ tooltip.date }}</div>
        <div class="tooltip-total">{{ tooltip.count }} sessions</div>
        <div v-if="tooltip.projects.length" class="tooltip-projects">
          <div v-for="p in tooltip.projects" :key="p.project_slug" class="tooltip-project">
            <span>{{ p.project_slug }}</span>
            <span>{{ p.session_count }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted, reactive } from 'vue'
import * as d3 from 'd3'
import type { DailyStats, DailyProjectStats } from '@/types'

const props = defineProps<{
  data: DailyStats[]
  projectData: DailyProjectStats[]
}>()

const containerRef = ref<HTMLElement | null>(null)
const svgRef = ref<SVGSVGElement | null>(null)

const tooltip = reactive({
  visible: false, x: 0, y: 0, date: '', count: 0, projects: [] as DailyProjectStats[]
})

function renderHeatmap() {
  if (!svgRef.value || !props.data.length) return

  const data = props.data.map(d => ({
    date: new Date(d.date), dateStr: d.date, count: d.session_count || 0
  })).sort((a, b) => a.date.getTime() - b.date.getTime())

  const width = containerRef.value?.clientWidth || 800
  const cellSize = 12
  const cellPadding = 3
  const height = (cellSize + cellPadding) * 7 + 28

  const svg = d3.select(svgRef.value)
  svg.selectAll('*').remove()
  svg.attr('width', width).attr('height', height)

  const monthFormat = d3.timeFormat('%b')
  const firstDate = data[0].date
  const lastDate = data[data.length - 1].date
  const g = svg.append('g').attr('transform', 'translate(32, 22)')

  // Month labels
  const months = d3.timeMonths(d3.timeMonth.offset(firstDate, -1), lastDate)
  g.selectAll('.month-label')
    .data(months).enter().append('text')
    .attr('class', 'month-label')
    .attr('x', d => d3.timeWeek.count(firstDate, d) * (cellSize + cellPadding))
    .attr('y', -6)
    .text(d => monthFormat(d))
    .style('font-size', '10px')
    .style('fill', 'var(--bl-text-3)')
    .style('font-weight', '500')

  // Day labels
  g.selectAll('.day-label')
    .data(['M', 'W', 'F']).enter().append('text')
    .attr('class', 'day-label')
    .attr('x', -20)
    .attr('y', (_, i) => (i * 2 + 1) * (cellSize + cellPadding) + cellSize / 2 + 3)
    .text(d => d)
    .style('font-size', '9px')
    .style('fill', 'var(--bl-text-3)')

  // Cells
  g.selectAll('.day')
    .data(data).enter().append('rect')
    .attr('class', 'day')
    .attr('width', cellSize).attr('height', cellSize)
    .attr('rx', 2)
    .attr('x', d => d3.timeWeek.count(firstDate, d.date) * (cellSize + cellPadding))
    .attr('y', d => d.date.getDay() * (cellSize + cellPadding))
    .attr('fill', d => {
      if (d.count === 0) return 'var(--bl-surface-2)'
      const intensity = Math.min(d.count / 8, 1)
      return `rgba(93, 204, 203, ${0.15 + intensity * 0.85})`
    })
    .on('mouseenter', (event, d) => {
      tooltip.visible = true
      tooltip.date = d.date.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })
      tooltip.count = d.count
      tooltip.projects = props.projectData.filter(p => p.date === d.dateStr)
      const rect = event.target.getBoundingClientRect()
      const cr = containerRef.value?.getBoundingClientRect() || { left: 0, top: 0 }
      tooltip.x = rect.left - cr.left + cellSize + 8
      tooltip.y = rect.top - cr.top - 10
    })
    .on('mouseleave', () => { tooltip.visible = false })
}

onMounted(() => { renderHeatmap(); window.addEventListener('resize', renderHeatmap) })
onUnmounted(() => window.removeEventListener('resize', renderHeatmap))
watch(() => [props.data, props.projectData], renderHeatmap, { deep: true })
</script>

<style scoped>
.heatmap-wrapper { width: 100%; }

.heatmap-container {
  position: relative;
  width: 100%;
  overflow-x: auto;
}

.empty-state {
  padding: 2rem;
  text-align: center;
  color: var(--bl-text-3);
  font-size: var(--bl-text-sm);
}

.heatmap-tooltip {
  position: absolute;
  z-index: 1000;
  background: var(--bl-surface-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-md);
  padding: 0.625rem 0.75rem;
  box-shadow: var(--bl-shadow-md);
  pointer-events: none;
  min-width: 160px;
}

.tooltip-date {
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  margin-bottom: 2px;
}

.tooltip-total {
  font-size: var(--bl-text-base);
  font-weight: 600;
  color: var(--bl-text);
  margin-bottom: 0.375rem;
}

.tooltip-projects {
  border-top: 1px solid var(--bl-border);
  padding-top: 0.375rem;
}

.tooltip-project {
  display: flex;
  justify-content: space-between;
  font-size: var(--bl-text-xs);
  color: var(--bl-text-2);
  gap: 1rem;
}
</style>
