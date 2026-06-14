<script setup>
import { computed } from 'vue'

const props = defineProps({
  step: Object,
  arrayData: Object,
  animState: Object
})

const values = computed(() => props.step?.values || props.arrayData?.values || [])
const winStart = computed(() => props.step?.windowStart ?? -1)
const winEnd = computed(() => props.step?.windowEnd ?? -1)
const highlightIdx = computed(() => props.step?.highlightIdx || [])
const resultIdx = computed(() => props.step?.resultIdx || [])

const boxWidth = 48
const boxHeight = 42
const gap = 8
const paddingLeft = 30
const paddingTop = 50

function cellColor(idx) {
  if (resultIdx.value.includes(idx)) return '#10b981'
  if (highlightIdx.value.includes(idx)) return '#3b82f6'
  if (idx >= winStart.value && idx <= winEnd.value && winStart.value >= 0) return '#1e3a5f'
  return '#374151'
}

const svgW = computed(() => paddingLeft * 2 + values.value.length * (boxWidth + gap))
const viewBox = computed(() => `0 0 ${svgW.value} 180`)

const windowLabelX = computed(() => {
  if (winStart.value < 0 || winEnd.value < 0) return null
  const x1 = paddingLeft + winStart.value * (boxWidth + gap)
  const x2 = paddingLeft + winEnd.value * (boxWidth + gap) + boxWidth
  return { x1, x2, mid: (x1 + x2) / 2 }
})
</script>

<template>
  <svg :viewBox="viewBox" width="100%" preserveAspectRatio="xMidYMid meet" class="slidingwindow-renderer">
    <!-- Window bracket -->
    <g v-if="windowLabelX && winStart >= 0">
      <line :x1="windowLabelX.x1" :y1="paddingTop - 8" :x2="windowLabelX.x2" :y2="paddingTop - 8" stroke="#60a5fa" stroke-width="2"/>
      <line :x1="windowLabelX.x1" :y1="paddingTop - 12" :x2="windowLabelX.x1" :y2="paddingTop - 4" stroke="#60a5fa" stroke-width="2"/>
      <line :x1="windowLabelX.x2" :y1="paddingTop - 12" :x2="windowLabelX.x2" :y2="paddingTop - 4" stroke="#60a5fa" stroke-width="2"/>
      <text :x="windowLabelX.mid" :y="paddingTop - 18" text-anchor="middle" fill="#60a5fa" font-size="11">
        window [{{ winStart }}, {{ winEnd }}]
      </text>
    </g>

    <!-- Value boxes -->
    <g v-for="(v, i) in values" :key="i">
      <rect
        :x="paddingLeft + i * (boxWidth + gap)"
        :y="paddingTop"
        :width="boxWidth"
        :height="boxHeight"
        rx="6"
        :fill="cellColor(i)"
        :stroke="i >= winStart && i <= winEnd && winStart >= 0 ? '#3b82f6' : 'transparent'"
        :stroke-width="i >= winStart && i <= winEnd && winStart >= 0 ? 2 : 0"
        style="transition: all 0.3s ease"
      />
      <text
        :x="paddingLeft + i * (boxWidth + gap) + boxWidth / 2"
        :y="paddingTop + boxHeight / 2 + 5"
        text-anchor="middle"
        fill="#f3f4f6"
        font-size="14"
        font-weight="bold"
      >{{ v }}</text>
      <text
        :x="paddingLeft + i * (boxWidth + gap) + boxWidth / 2"
        :y="paddingTop + boxHeight + 16"
        text-anchor="middle"
        fill="#9ca3af"
        font-size="10"
      >{{ i }}</text>
    </g>
  </svg>
</template>

<style scoped>
.slidingwindow-renderer {
  display: block;
  margin: 0 auto;
}
</style>
