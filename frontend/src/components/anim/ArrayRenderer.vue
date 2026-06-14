<script setup>
import { computed } from 'vue'

const props = defineProps({
  step: Object,
  arrayData: Object,
  animState: Object
})

const values = computed(() => props.step?.values || props.arrayData?.values || [])
const highlightIdx = computed(() => props.step?.highlightIdx || [])
const compareIdx = computed(() => props.step?.compareIdx || [])
const resultIdx = computed(() => props.step?.resultIdx || [])

const maxVal = computed(() => Math.max(...values.value, 1))

const boxWidth = 52
const boxHeight = 48
const gap = 6
const paddingLeft = 20
const paddingTop = 40
const barHeight = 30

function cellColor(idx) {
  if (resultIdx.value.includes(idx)) return '#10b981'
  if (compareIdx.value.includes(idx)) return '#f59e0b'
  if (highlightIdx.value.includes(idx)) return '#3b82f6'
  return '#374151'
}

const svgW = computed(() => paddingLeft * 2 + values.value.length * (boxWidth + gap))
const viewBox = computed(() => `0 0 ${svgW.value} 240`)
</script>

<template>
  <svg :viewBox="viewBox" width="100%" preserveAspectRatio="xMidYMid meet" class="array-renderer">
    <!-- Bar chart background -->
    <g v-for="(v, i) in values" :key="'bar-'+i">
      <rect
        :x="paddingLeft + i * (boxWidth + gap)"
        :y="paddingTop + (barHeight + 10) * (1 - v / maxVal) + 10"
        :width="boxWidth"
        :height="(barHeight + 10) * (v / maxVal) + 10"
        rx="4"
        :fill="cellColor(i)"
        :opacity="highlightIdx.includes(i) || compareIdx.includes(i) || resultIdx.includes(i) ? 1 : 0.6"
        style="transition: all 0.3s ease"
      />
    </g>

    <!-- Value labels -->
    <g v-for="(v, i) in values" :key="'val-'+i">
      <rect
        :x="paddingLeft + i * (boxWidth + gap) - 2"
        :y="paddingTop + barHeight + 25"
        :width="boxWidth + 4"
        :height="boxHeight - 4"
        rx="6"
        :fill="cellColor(i)"
        :stroke="i === animState?.activeIdx ? '#60a5fa' : 'transparent'"
        :stroke-width="i === animState?.activeIdx ? 2 : 0"
        style="transition: all 0.3s ease"
      />
      <text
        :x="paddingLeft + i * (boxWidth + gap) + boxWidth / 2"
        :y="paddingTop + barHeight + 22 + boxHeight / 2 + 5"
        text-anchor="middle"
        fill="#f3f4f6"
        font-size="15"
        font-weight="bold"
      >{{ v }}</text>
      <text
        :x="paddingLeft + i * (boxWidth + gap) + boxWidth / 2"
        :y="paddingTop + barHeight + 22 + boxHeight + 16"
        text-anchor="middle"
        fill="#9ca3af"
        font-size="11"
      >idx={{ i }}</text>
    </g>
  </svg>
</template>

<style scoped>
.array-renderer {
  display: block;
  margin: 0 auto;
}
</style>
