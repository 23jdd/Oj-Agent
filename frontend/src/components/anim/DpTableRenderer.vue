<script setup>
import { computed } from 'vue'

const props = defineProps({
  step: Object,
  tableData: Object,
  animState: Object
})

const rows = computed(() => props.step?.tableGrid || [])
const rowHeaders = computed(() => props.tableData?.rowHeaders || [])
const colHeaders = computed(() => props.tableData?.colHeaders || [])
const currentRow = computed(() => props.step?.row ?? -1)
const currentCol = computed(() => props.step?.col ?? -1)

const cellW = 48
const cellH = 34
const headerW = 100

function cellFill(r, c) {
  if (r === 0 && c === 0) return '#1a1f2e'
  if (r === currentRow.value && c === currentCol.value) return '#3b82f6'
  if (r === currentRow.value && currentCol.value >= 0) return '#1e3a5f'
  return '#1f2937'
}
</script>

<template>
  <svg :width="headerW + (colHeaders.length) * cellW + 30" :height="(rows.length + 1) * cellH + 30" class="dptable-renderer">
    <!-- Column headers -->
    <g v-for="(h, ci) in colHeaders" :key="'ch-'+ci">
      <rect :x="headerW + ci * cellW" :y="0" :width="cellW" :height="cellH" fill="#111827" stroke="#374151" />
      <text :x="headerW + ci * cellW + cellW / 2" :y="cellH / 2 + 5" text-anchor="middle" fill="#9ca3af" font-size="11">{{ h }}</text>
    </g>

    <!-- Row headers and cells -->
    <g v-for="(row, ri) in rows" :key="'row-'+ri">
      <rect :x="0" :y="cellH + ri * cellH" :width="headerW" :height="cellH" fill="#111827" stroke="#374151" />
      <text :x="headerW / 2" :y="cellH + ri * cellH + cellH / 2 + 4" text-anchor="middle" fill="#9ca3af" font-size="11">
        {{ rowHeaders[ri] || ('i=' + ri) }}
      </text>

      <g v-for="(val, ci) in row" :key="'cell-'+ri+'-'+ci">
        <rect
          :x="headerW + ci * cellW" :y="cellH + ri * cellH"
          :width="cellW" :height="cellH"
          :fill="cellFill(ri, ci)"
          :stroke="ri === currentRow && ci === currentCol ? '#60a5fa' : '#374151'"
          :stroke-width="ri === currentRow && ci === currentCol ? 2.5 : 1"
          style="transition: all 0.3s ease"
        />
        <text
          :x="headerW + ci * cellW + cellW / 2"
          :y="cellH + ri * cellH + cellH / 2 + 5"
          text-anchor="middle"
          :fill="ri === currentRow && ci === currentCol ? '#fff' : '#d1d5db'"
          font-size="13"
          font-weight="bold"
        >{{ val }}</text>
      </g>
    </g>
  </svg>
</template>

<style scoped>
.dptable-renderer {
  display: block;
  margin: 0 auto;
}
</style>
