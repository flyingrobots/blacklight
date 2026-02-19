<template>
  <div class="heatmap-wrapper">
    <div class="heatmap-container" ref="containerRef">
      <svg ref="svgRef"></svg>
      <div v-if="!data.length" class="empty-state">No activity data available</div>
      
      <!-- Tooltip -->
      <div 
        v-if="tooltip.visible" 
        class="heatmap-tooltip"
        :style="{ left: tooltip.x + 'px', top: tooltip.y + 'px' }"
      >
        <div class="tooltip-date">{{ tooltip.date }}</div>
        <div class="tooltip-total">{{ tooltip.count }} Sessions</div>
        <div class="tooltip-divider"></div>
        <div class="tooltip-projects">
          <div v-for="p in tooltip.projects" :key="p.project_slug" class="tooltip-project">
            <span class="project-name">{{ p.project_slug }}</span>
            <span class="project-count">{{ p.session_count }}</span>
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
  visible: false,
  x: 0,
  y: 0,
  date: '',
  count: 0,
  projects: [] as DailyProjectStats[]
})

function renderHeatmap() {
  if (!svgRef.value || !props.data.length) return

  const data = props.data.map(d => ({
    date: new Date(d.date),
    dateStr: d.date,
    count: d.session_count || 0
  })).sort((a, b) => a.date.getTime() - b.date.getTime())

  const width = containerRef.value?.clientWidth || 800
  const cellSize = 14
  const cellPadding = 3
  const height = (cellSize + cellPadding) * 7 + 30 

  const svg = d3.select(svgRef.value)
  svg.selectAll('*').remove()

  svg.attr('width', width)
     .attr('height', height)

  // Use CSS variables for the color scale
  const colorScale = d3.scaleThreshold<number, string>()
    .domain([1, 2, 5, 10])
    .range(['var(--bl-bg-3)', 'rgba(59, 130, 246, 0.2)', 'rgba(59, 130, 246, 0.4)', 'rgba(59, 130, 246, 0.7)', 'var(--bl-accent)'])

  const monthFormat = d3.timeFormat('%b')

  const firstDate = data[0].date
  const lastDate = data[data.length - 1].date
  
  const g = svg.append('g').attr('transform', 'translate(40, 25)')

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
    .attr('y', -8)
    .text(d => monthFormat(d))
    .style('font-size', '10px')
    .style('fill', 'var(--bl-text-3)')
    .style('font-family', 'var(--bl-font-mono)')
    .style('font-weight', 'bold')

  // Day labels
  const days = ['Mon', 'Wed', 'Fri']
  g.selectAll('.day-label')
    .data(days)
    .enter()
    .append('text')
    .attr('class', 'day-label')
    .attr('x', -35)
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
    .style('cursor', 'pointer')
    .on('mouseenter', (event, d) => {
      const projects = props.projectData.filter(p => p.date === d.dateStr)
      tooltip.visible = true
      tooltip.date = d.date.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric', year: 'numeric' })
      tooltip.count = d.count
      tooltip.projects = projects
      
      const rect = event.target.getBoundingClientRect()
      const containerRect = containerRef.value?.getBoundingClientRect() || { left: 0, top: 0 }
      
      tooltip.x = rect.left - containerRect.left + cellSize + 10
      tooltip.y = rect.top - containerRect.top - 20
    })
    .on('mouseleave', () => {
      tooltip.visible = false
    })
}

onMounted(() => {
  renderHeatmap()
  window.addEventListener('resize', renderHeatmap)
})

onUnmounted(() => {
  window.removeEventListener('resize', renderHeatmap)
})

watch(() => [props.data, props.projectData], renderHeatmap, { deep: true })
</script>

<style scoped>
.heatmap-wrapper {
  width: 100%;
  display: flex;
  justify-content: center;
}
.heatmap-container {
  position: relative;
  width: fit-content;
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-sm);
  padding: 1.5rem 1.5rem 1rem;
}
svg {
  display: block;
}
.empty-state {
  padding: 2rem;
  text-align: center;
  color: var(--bl-text-3);
  font-family: var(--bl-font-mono);
  font-size: var(--bl-text-xs);
}

.heatmap-tooltip {
  position: absolute;
  z-index: 1000;
  background: var(--bl-bg-3);
  border: 1px solid var(--bl-accent);
  border-radius: var(--bl-radius-sm);
  padding: 0.75rem;
  font-family: var(--bl-font-mono);
  box-shadow: var(--bl-shadow-lg);
  pointer-events: none;
  min-width: 180px;
}

.tooltip-date {
  font-size: 10px;
  color: var(--bl-text-3);
  text-transform: uppercase;
  margin-bottom: 2px;
}

.tooltip-total {
  font-size: 14px;
  color: var(--bl-text);
  font-weight: bold;
  margin-bottom: 8px;
}

.tooltip-divider {
  height: 1px;
  background: var(--bl-border);
  margin: 8px 0;
}

.tooltip-projects {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tooltip-project {
  display: flex;
  justify-content: space-between;
  gap: 1rem;
  font-size: 11px;
}

.project-name {
  color: var(--bl-text-2);
}

.project-count {
  color: var(--bl-accent);
  font-weight: bold;
}
</style>
