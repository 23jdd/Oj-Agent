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
const swapIdx = computed(() => props.step?.swapIdx || [])
const pivotIdx = computed(() => props.step?.pivotIdx ?? -1)
const resultIdx = computed(() => props.step?.resultIdx || [])

const boxW = 44
const boxH = 40
const gap = 8
const padL = 30
const padT = 60

function cellColor(idx) {
  if (resultIdx.value.includes(idx)) return '#10b981'
  if (swapIdx.value.includes(idx)) return '#ef4444'
  if (compareIdx.value.includes(idx)) return '#f59e0b'
  if (highlightIdx.value.includes(idx)) return '#3b82f6'
  if (idx === pivotIdx.value) return '#8b5cf6'
  return '#374151'
}

function cellBorder(idx) {
  if (idx === pivotIdx.value) return '#a78bfa'
  return 'transparent'
}

const svgW = computed(() => padL * 2 + values.value.length * (boxW + gap))
const viewBox = computed(() => `0 0 ${svgW.value} 170`)
</script>

<template>
  <svg :viewBox="viewBox" width="100%" preserveAspectRatio="xMidYMid meet" class="sorting-renderer">
    <!-- Pivot triangle -->
    <g v-if="pivotIdx >= 0">
      <polygon
        :points="`${padL + pivotIdx * (boxW + gap) + boxW / 2 - 6},${padT - 20} ${padL + pivotIdx * (boxW + gap) + boxW / 2 + 6},${padT - 20} ${padL + pivotIdx * (boxW + gap) + boxW / 2},${padT - 8}`"
        fill="#8b5cf6"
      />
      <text
        :x="padL + pivotIdx * (boxW + gap) + boxW / 2"
        :y="padT - 26"
        text-anchor="middle"
        fill="#a78bfa"
        font-size="11"
      >pivot</text>
    </g>

    <!-- Boxes -->
    <g v-for="(v, i) in values" :key="i">
      <rect
        :x="padL + i * (boxW + gap)" :y="padT"
        :width="boxW" :height="boxH"
        rx="6"
        :fill="cellColor(i)"
        :stroke="cellBorder(i)"
        :stroke-width="i === pivotIdx ? 2.5 : 0"
        style="transition: all 0.4s ease"
      />
      <text
        :x="padL + i * (boxW + gap) + boxW / 2"
        :y="padT + boxH / 2 + 5"
        text-anchor="middle"
        fill="#f3f4f6"
        font-size="14"
        font-weight="bold"
      >{{ v }}</text>
      <text
        :x="padL + i * (boxW + gap) + boxW / 2"
        :y="padT + boxH + 16"
        text-anchor="middle"
        fill="#9ca3af"
        font-size="10"
      >{{ i }}</text>
    </g>
  </svg>
</template>

<style scoped>
.sorting-renderer {
  display: block;
  margin: 0 auto;
}
</style>
