<template>
  <div class="bar-chart-container" ref="containerRef">
    <div class="chart-header">
      <h4 v-if="title">{{ title }}</h4>
    </div>
    <svg ref="svgRef"></svg>
    <div v-if="!data.length" class="empty-state">No data available</div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, onUnmounted } from 'vue'
import * as d3 from 'd3'

interface ChartData {
  label: string
  value: number
}

const props = defineProps<{
  data: ChartData[]
  title?: string
  color?: string
}>()

const containerRef = ref<HTMLElement | null>(null)
const svgRef = ref<SVGSVGElement | null>(null)

function renderChart() {
  if (!svgRef.value || !props.data.length) return

  const margin = { top: 10, right: 20, bottom: 40, left: 100 }
  const width = (containerRef.value?.clientWidth || 400) - margin.left - margin.right
  const height = Math.max(props.data.length * 25, 100)

  const svg = d3.select(svgRef.value)
  svg.selectAll('*').remove()

  svg.attr('width', width + margin.left + margin.right)
     .attr('height', height + margin.top + margin.bottom)

  const g = svg.append('g')
    .attr('transform', `translate(${margin.left},${margin.top})`)

  const x = d3.scaleLinear()
    .domain([0, d3.max(props.data, d => d.value) || 0])
    .range([0, width])

  const y = d3.scaleBand()
    .range([0, height])
    .domain(props.data.map(d => d.label))
    .padding(0.2)

  // X Axis
  g.append('g')
    .attr('transform', `translate(0,${height})`)
    .call(d3.axisBottom(x).ticks(5).tickSizeOuter(0))
    .style('font-family', 'var(--bl-font-mono)')
    .style('font-size', '10px')
    .selectAll('text')
    .style('fill', 'var(--bl-text-3)')

  // Y Axis
  g.append('g')
    .call(d3.axisLeft(y).tickSize(0))
    .style('font-family', 'var(--bl-font-mono)')
    .style('font-size', '10px')
    .selectAll('text')
    .style('fill', 'var(--bl-text-2)')
    .attr('dx', '-10px')

  // Bars
  g.selectAll('.bar')
    .data(props.data)
    .enter()
    .append('rect')
    .attr('class', 'bar')
    .attr('y', d => y(d.label) || 0)
    .attr('height', y.bandwidth())
    .attr('x', 0)
    .attr('width', d => x(d.value))
    .attr('fill', props.color || 'var(--bl-accent)')
    .attr('rx', 2)
    .attr('ry', 2)
    .style('opacity', 0.8)

  // Labels on bars
  g.selectAll('.label')
    .data(props.data)
    .enter()
    .append('text')
    .attr('class', 'label')
    .attr('y', d => (y(d.label) || 0) + y.bandwidth() / 2 + 4)
    .attr('x', d => x(d.value) + 5)
    .text(d => d.value.toLocaleString())
    .style('font-size', '10px')
    .style('font-family', 'var(--bl-font-mono)')
    .style('fill', 'var(--bl-text-3)')

  svg.selectAll('.domain, .tick line').style('stroke', 'var(--bl-border)')
}

onMounted(() => {
  renderChart()
  window.addEventListener('resize', renderChart)
})

onUnmounted(() => {
  window.removeEventListener('resize', renderChart)
})

watch(() => props.data, renderChart, { deep: true })
</script>

<style scoped>
.bar-chart-container {
  width: 100%;
  background: var(--bl-bg-2);
  border: 1px solid var(--bl-border);
  border-radius: var(--bl-radius-sm);
  padding: 1rem;
}
.chart-header h4 {
  font-family: var(--bl-font-mono);
  font-size: var(--bl-text-xs);
  text-transform: uppercase;
  color: var(--bl-text-2);
  margin-bottom: 1rem;
}
.empty-state {
  padding: 2rem;
  text-align: center;
  color: var(--bl-text-3);
  font-family: var(--bl-font-mono);
  font-size: var(--bl-text-xs);
}
</style>
