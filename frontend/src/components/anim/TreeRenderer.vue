<script setup>
import { computed } from 'vue'

const props = defineProps({
  step: Object,
  treeData: Object,
  animState: Object
})

const nodes = computed(() => props.treeData?.nodes || [])
const nodePath = computed(() => props.step?.nodePath || [])
const currentNodeId = computed(() => props.step?.nodeId || '')

const nodeMap = computed(() => {
  const m = {}
  nodes.value.forEach(n => { m[n.id] = n })
  return m
})

function nodeColor(id) {
  if (id === currentNodeId.value) return '#3b82f6'
  if (nodePath.value.includes(id)) return '#1e40af'
  return '#374151'
}

function nodeStroke(id) {
  if (id === currentNodeId.value) return '#60a5fa'
  if (nodePath.value.includes(id)) return '#3b82f6'
  return '#4b5563'
}

const nodeRadius = 22

function edgeColor(parentId, childId) {
  if (nodePath.value.includes(parentId) && nodePath.value.includes(childId)) return '#3b82f6'
  return '#4b5563'
}

function edgeWidth(parentId, childId) {
  if (nodePath.value.includes(parentId) && nodePath.value.includes(childId)) return 2.5
  return 1.5
}

const treeSize = computed(() => {
  let maxX = 100, maxY = 100
  nodes.value.forEach(n => {
    if (n.x + 30 > maxX) maxX = n.x + 30
    if (n.y + 30 > maxY) maxY = n.y + 30
  })
  return { width: maxX + 60, height: maxY + 60 }
})
</script>

<template>
  <svg :viewBox="`0 0 ${treeSize.width} ${treeSize.height}`" width="100%" preserveAspectRatio="xMidYMid meet" class="tree-renderer">
    <!-- Edges -->
    <g v-for="n in nodes" :key="'edge-'+n.id">
      <line
        v-if="n.left && nodeMap[n.left]"
        :x1="n.x" :y1="n.y + nodeRadius"
        :x2="nodeMap[n.left].x" :y2="nodeMap[n.left].y - nodeRadius"
        :stroke="edgeColor(n.id, n.left)"
        :stroke-width="edgeWidth(n.id, n.left)"
        style="transition: all 0.3s ease"
      />
      <line
        v-if="n.right && nodeMap[n.right]"
        :x1="n.x" :y1="n.y + nodeRadius"
        :x2="nodeMap[n.right].x" :y2="nodeMap[n.right].y - nodeRadius"
        :stroke="edgeColor(n.id, n.right)"
        :stroke-width="edgeWidth(n.id, n.right)"
        style="transition: all 0.3s ease"
      />
    </g>

    <!-- Nodes -->
    <g v-for="n in nodes" :key="'node-'+n.id">
      <circle
        :cx="n.x" :cy="n.y"
        :r="nodeRadius"
        :fill="nodeColor(n.id)"
        :stroke="nodeStroke(n.id)"
        :stroke-width="n.id === currentNodeId ? 3 : 2"
        style="transition: all 0.3s ease"
      />
      <text
        :x="n.x" :y="n.y + 5"
        text-anchor="middle"
        fill="#f3f4f6"
        font-size="14"
        font-weight="bold"
      >{{ n.value }}</text>
    </g>
  </svg>
</template>

<style scoped>
.tree-renderer {
  display: block;
  margin: 0 auto;
}
</style>
