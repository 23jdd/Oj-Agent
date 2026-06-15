<script setup>
import { computed, reactive, watch } from 'vue'

const props = defineProps({
  elements: { type: Array, default: () => [] },
  frames: { type: Array, default: () => [] },
  currentStep: { type: Number, default: -1 },
  svgW: { type: Number, default: 400 },
  svgH: { type: Number, default: 250 }
})

const state = reactive({})

function initState() {
  Object.keys(state).forEach(k => delete state[k])
  props.elements.forEach(el => {
    state[el.id] = {
      id: el.id,
      kind: el.kind,
      x: el.x ?? 0,
      y: el.y ?? 0,
      w: el.w ?? 0,
      h: el.h ?? 0,
      r: el.r ?? 0,
      x2: el.x2 ?? 0,
      y2: el.y2 ?? 0,
      text: el.text ?? '',
      style: el.style ?? 'normal',
      rx: el.rx ?? 0,
      visible: el.visible !== false,
      points: el.points ?? '',
      arrow: el.arrow ?? false,
      showGrid: el.showGrid ?? false,
    }
  })
}

function applyFrame(stepIdx) {
  if (stepIdx < 0 || stepIdx >= props.frames.length) {
    initState()
    return
  }
  const frame = props.frames[stepIdx]
  if (!frame?.delta) return
  for (const [id, changes] of Object.entries(frame.delta)) {
    if (!state[id]) continue
    if (changes.x !== undefined) state[id].x = changes.x
    if (changes.y !== undefined) state[id].y = changes.y
    if (changes.x2 !== undefined) state[id].x2 = changes.x2
    if (changes.y2 !== undefined) state[id].y2 = changes.y2
    if (changes.text !== undefined) state[id].text = changes.text
    if (changes.style !== undefined) state[id].style = changes.style
    if (changes.visible !== undefined) state[id].visible = changes.visible
    if (changes.points !== undefined) state[id].points = changes.points
    if (changes.w !== undefined) state[id].w = changes.w
    if (changes.showGrid !== undefined) state[id].showGrid = changes.showGrid
  }
}

watch(() => props.elements, initState, { immediate: true, deep: true })
watch(() => [props.currentStep, props.frames], ([step, frames]) => {
  initState()
  applyFrame(step)
}, { immediate: true })

function fillColor(style) {
  switch (style) {
    case 'highlight': return '#3b82f6'
    case 'compare': return '#f59e0b'
    case 'swap': return '#ef4444'
    case 'result': return '#10b981'
    case 'pivot': return '#8b5cf6'
    case 'dim': return '#1e293b'
    default: return '#374151'
  }
}

function glowColor(style) {
  switch (style) {
    case 'highlight': return 'rgba(59,130,246,0.45)'
    case 'compare': return 'rgba(245,158,11,0.45)'
    case 'swap': return 'rgba(239,68,68,0.45)'
    case 'result': return 'rgba(16,185,129,0.45)'
    case 'pivot': return 'rgba(139,92,246,0.45)'
    default: return 'transparent'
  }
}

function strokeColor(style) {
  if (style === 'highlight') return '#60a5fa'
  if (style === 'swap') return '#f87171'
  if (style === 'compare') return '#fbbf24'
  if (style === 'pivot') return '#a78bfa'
  if (style === 'result') return '#34d399'
  return '#4b5563'
}

function textColor(style) {
  if (style === 'dim') return '#6b7280'
  return '#f3f4f6'
}

function hasGlow(style) {
  return ['highlight', 'compare', 'swap', 'result', 'pivot'].includes(style)
}

function lineDash(style) {
  return style === 'dim' ? '4,4' : 'none'
}

function lineWidth(style) {
  if (style === 'highlight' || style === 'result' || style === 'swap') return 2.5
  return 1.5
}

function arrowPoints(x2, y2, size) {
  return `${x2 - size},${y2 - size / 2} ${x2},${y2} ${x2 - size},${y2 + size / 2}`
}

function lineColor(s) {
  const st = s || ''
  if (st.startsWith('#')) return st
  return strokeColor(st)
}

const viewBox = computed(() => `0 0 ${props.svgW} ${props.svgH}`)

const gridDefs = computed(() => [...props.elements.filter(e => e.showGrid)])
</script>

<template>
  <svg :viewBox="viewBox" width="100%" preserveAspectRatio="xMidYMid meet" class="universal-renderer">
    <defs>
      <filter v-for="e in elements.filter(x => x.showGrid)" :key="'grid-'+e.id" :id="'grid-'+e.id" x="0" y="0" :width="e.w||100" :height="e.h||100">
        <feFlood flood-color="rgba(59,130,246,0.04)"/>
        <feComposite in="SourceGraphic"/>
      </filter>
    </defs>

    <rect :width="svgW" :height="svgH" fill="#0f1419" />

    <!-- Grid backgrounds for DP tables -->
    <g v-for="el in elements.filter(e => e.showGrid)" :key="'gridbg-'+el.id">
      <rect :x="el.x" :y="el.y" :width="el.w" :height="el.h" fill="rgba(59,130,246,0.03)" stroke="rgba(59,130,246,0.08)" rx="4" />
    </g>

    <!-- Lines -->
    <g v-for="el in elements" :key="'line-'+el.id">
      <g v-if="state[el.id]?.kind === 'line' && state[el.id]?.visible">
        <line
          :x1="state[el.id].x" :y1="state[el.id].y"
          :x2="state[el.id].x2" :y2="state[el.id].y2"
          :stroke="lineColor(state[el.id].style)"
          :stroke-width="lineWidth(state[el.id].style)"
          :stroke-dasharray="lineDash(state[el.id].style)"
          stroke-linecap="round"
          style="transition: all 0.3s ease"
        />
        <polygon
          v-if="state[el.id]?.arrow"
          :points="arrowPoints(state[el.id].x2, state[el.id].y2, 8)"
          :fill="lineColor(state[el.id].style)"
          style="transition: all 0.3s ease"
        />
        <!-- Glow on highlighted lines -->
        <line
          v-if="state[el.id]?.style === 'highlight'"
          :x1="state[el.id].x" :y1="state[el.id].y"
          :x2="state[el.id].x2" :y2="state[el.id].y2"
          :stroke="glowColor(state[el.id].style)"
          stroke-width="6"
          stroke-linecap="round"
          opacity="0.5"
          style="transition: all 0.3s ease"
        />
      </g>
    </g>

    <!-- Rectangles -->
    <g v-for="el in elements" :key="'rect-'+el.id">
      <g v-if="state[el.id]?.kind === 'rect' && state[el.id]?.visible">
        <rect
          v-if="hasGlow(state[el.id].style)"
          :x="state[el.id].x - 2" :y="state[el.id].y - 2"
          :width="state[el.id].w + 4" :height="state[el.id].h + 4"
          :rx="(state[el.id].rx || 4) + 2"
          :fill="glowColor(state[el.id].style)"
          class="glow-pulse"
          style="transition: all 0.3s ease"
        />
        <rect
          :x="state[el.id].x" :y="state[el.id].y"
          :width="state[el.id].w" :height="state[el.id].h"
          :rx="state[el.id].rx || 4"
          :fill="fillColor(state[el.id].style)"
          :stroke="strokeColor(state[el.id].style)"
          :stroke-width="state[el.id].style !== 'normal' && state[el.id].style !== 'dim' ? 2 : 1"
          style="transition: all 0.3s ease"
        />
        <text
          :x="state[el.id].x + state[el.id].w / 2"
          :y="state[el.id].y + state[el.id].h / 2 + 5"
          text-anchor="middle"
          :fill="textColor(state[el.id].style)"
          font-size="13"
          font-weight="bold"
          style="transition: all 0.3s ease"
        >{{ state[el.id].text }}</text>
      </g>
    </g>

    <!-- Circles -->
    <g v-for="el in elements" :key="'circle-'+el.id">
      <g v-if="state[el.id]?.kind === 'circle' && state[el.id]?.visible">
        <circle
          v-if="hasGlow(state[el.id].style)"
          :cx="state[el.id].x" :cy="state[el.id].y"
          :r="(state[el.id].r || 20) + 3"
          :fill="glowColor(state[el.id].style)"
          class="glow-pulse"
          style="transition: all 0.3s ease"
        />
        <circle
          :cx="state[el.id].x" :cy="state[el.id].y"
          :r="state[el.id].r || 20"
          :fill="fillColor(state[el.id].style)"
          :stroke="strokeColor(state[el.id].style)"
          :stroke-width="state[el.id].style !== 'normal' && state[el.id].style !== 'dim' ? 2.5 : 1.5"
          style="transition: all 0.3s ease"
        />
        <text
          :x="state[el.id].x" :y="state[el.id].y + 5"
          text-anchor="middle"
          :fill="textColor(state[el.id].style)"
          font-size="14"
          font-weight="bold"
          style="transition: all 0.3s ease"
        >{{ state[el.id].text }}</text>
      </g>
    </g>

    <!-- Labels -->
    <g v-for="el in elements" :key="'label-'+el.id">
      <text
        v-if="state[el.id]?.kind === 'label' && state[el.id]?.visible && state[el.id]?.text"
        :x="state[el.id].x"
        :y="state[el.id].y"
        text-anchor="middle"
        :fill="state[el.id].style === 'dim' ? '#6b7280' : '#9ca3af'"
        font-size="12"
        style="transition: all 0.3s ease"
      >{{ state[el.id].text }}</text>
    </g>
  </svg>
</template>

<style scoped>
.universal-renderer {
  display: block;
  margin: 0 auto;
  overflow: visible;
}

.glow-pulse {
  animation: pulseSoft 2s ease-in-out infinite;
}

@keyframes pulseSoft {
  0%, 100% { opacity: 0.35; }
  50% { opacity: 0.65; }
}
</style>
