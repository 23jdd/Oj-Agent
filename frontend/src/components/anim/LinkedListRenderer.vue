<script setup>
import { computed } from 'vue'

const props = defineProps({
  step: Object,
  listData: Object,
  animState: Object
})

const nodes = computed(() => props.step?.listNodes || props.listData?.nodes || [])
const currentNodeId = computed(() => props.step?.nodeId || '')

const nodeMap = computed(() => {
  const m = {}
  nodes.value.forEach(n => { m[n.id] = n })
  return m
})

const nodeW = 56
const nodeH = 42
const gap = 56
const startX = 30
const startY = 50

function nodeColor(id) {
  if (id === currentNodeId.value) return '#3b82f6'
  return '#374151'
}

function nodeStroke(id) {
  if (id === currentNodeId.value) return '#60a5fa'
  return '#4b5563'
}

const svgW = computed(() => startX * 2 + nodes.value.length * (nodeW + gap))
const viewBox = computed(() => `0 0 ${svgW.value} 180`)
</script>

<template>
  <svg :viewBox="viewBox" width="100%" preserveAspectRatio="xMidYMid meet" class="linkedlist-renderer">
    <!-- Edges -->
    <g v-for="(n, i) in nodes" :key="'edge-'+n.id">
      <g v-if="i < nodes.length - 1">
        <line
          :x1="startX + i * (nodeW + gap) + nodeW"
          :y1="startY + nodeH / 2"
          :x2="startX + (i + 1) * (nodeW + gap)"
          :y2="startY + nodeH / 2"
          :stroke="nodeColor(n.id) !== '#374151' ? '#3b82f6' : '#4b5563'"
          stroke-width="2"
        />
        <polygon
          :points="`${startX + (i + 1) * (nodeW + gap) - 6},${startY + nodeH / 2 - 5} ${startX + (i + 1) * (nodeW + gap)},${startY + nodeH / 2} ${startX + (i + 1) * (nodeW + gap) - 6},${startY + nodeH / 2 + 5}`"
          :fill="nodeColor(n.id) !== '#374151' ? '#3b82f6' : '#4b5563'"
        />
      </g>
    </g>

    <!-- Nodes -->
    <g v-for="(n, i) in nodes" :key="'node-'+n.id">
      <!-- First node indicator -->
      <text
        v-if="i === 0"
        :x="startX + i * (nodeW + gap) + nodeW / 2"
        :y="startY - 16"
        text-anchor="middle"
        fill="#6b7280"
        font-size="11"
      >head</text>

      <!-- Node rect with value + next partition -->
      <g>
        <!-- Value part -->
        <rect
          :x="startX + i * (nodeW + gap)"
          :y="startY"
          :width="nodeW"
          :height="nodeH"
          rx="6"
          :fill="nodeColor(n.id)"
          :stroke="nodeStroke(n.id)"
          :stroke-width="n.id === currentNodeId ? 2.5 : 1"
          style="transition: all 0.3s ease"
        />
        <!-- Divider -->
        <line
          :x1="startX + i * (nodeW + gap) + nodeW * 0.65"
          :y1="startY"
          :x2="startX + i * (nodeW + gap) + nodeW * 0.65"
          :y2="startY + nodeH"
          stroke="#4b5563"
          stroke-width="1"
        />

        <!-- Value -->
        <text
          :x="startX + i * (nodeW + gap) + nodeW * 0.35"
          :y="startY + nodeH / 2 + 5"
          text-anchor="middle"
          fill="#f3f4f6"
          font-size="14"
          font-weight="bold"
        >{{ n.value }}</text>

        <!-- Next pointer -->
        <text
          :x="startX + i * (nodeW + gap) + nodeW * 0.82"
          :y="startY + nodeH / 2 + 5"
          text-anchor="middle"
          fill="#9ca3af"
          font-size="10"
        >{{ n.next ? '●' : '◎' }}</text>
      </g>

      <!-- Last node indicator -->
      <text
        v-if="i === nodes.length - 1 && !n.next"
        :x="startX + i * (nodeW + gap) + nodeW / 2"
        :y="startY + nodeH + 22"
        text-anchor="middle"
        fill="#6b7280"
        font-size="11"
      >null</text>
    </g>
  </svg>
</template>

<style scoped>
.linkedlist-renderer {
  display: block;
  margin: 0 auto;
}
</style>
