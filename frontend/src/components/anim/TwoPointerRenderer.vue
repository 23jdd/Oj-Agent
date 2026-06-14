<script setup>
import { computed } from 'vue'

const props = defineProps({
  step: Object,
  arrayData: Object,
  animState: Object
})

const values = computed(() => props.step?.values || props.arrayData?.values || [])
const left = computed(() => props.step?.pointerLeft ?? -1)
const right = computed(() => props.step?.pointerRight ?? -1)
const highlightIdx = computed(() => props.step?.highlightIdx || [])
const compareIdx = computed(() => props.step?.compareIdx || [])
const resultIdx = computed(() => props.step?.resultIdx || [])

const maxVal = computed(() => Math.max(...values.value, 1))

const boxWidth = 48
const boxHeight = 44
const gap = 8
const paddingLeft = 30
const paddingTop = 60

function cellColor(idx) {
  if (resultIdx.value.includes(idx)) return '#10b981'
  if (compareIdx.value.includes(idx)) return '#f59e0b'
  if (highlightIdx.value.includes(idx)) return '#3b82f6'
  return '#374151'
}

function cellBorder(idx) {
  if (idx === left.value) return '#3b82f6'
  if (idx === right.value) return '#ef4444'
  return 'transparent'
}

function cellBorderWidth(idx) {
  if (idx === left.value || idx === right.value) return 3
  return 0
}

const svgW = computed(() => paddingLeft * 2 + values.value.length * (boxWidth + gap))
const viewBox = computed(() => `0 0 ${svgW.value} 180`)
</script>

<template>
  <svg :viewBox="viewBox" width="100%" preserveAspectRatio="xMidYMid meet" class="twopointer-renderer">
    <!-- Pointer labels -->
    <g v-if="left >= 0">
      <text
        :x="paddingLeft + left * (boxWidth + gap) + boxWidth / 2"
        :y="paddingTop - 16"
        text-anchor="middle"
        fill="#3b82f6"
        font-size="12"
        font-weight="bold"
      >▲ L</text>
    </g>
    <g v-if="right >= 0">
      <text
        :x="paddingLeft + right * (boxWidth + gap) + boxWidth / 2"
        :y="paddingTop - 16"
        text-anchor="middle"
        fill="#ef4444"
        font-size="12"
        font-weight="bold"
      >▲ R</text>
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
        :stroke="cellBorder(i)"
        :stroke-width="cellBorderWidth(i)"
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
.twopointer-renderer {
  display: block;
  margin: 0 auto;
}
</style>
