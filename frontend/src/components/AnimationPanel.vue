<script setup>
import { ref, watch, onMounted, onBeforeUnmount } from 'vue'

const props = defineProps({
  steps: Array,
  isAnimating: Boolean
})

const emit = defineEmits(['play', 'pause', 'reset'])

const canvasRef = ref(null)
const containerRef = ref(null)
const canvasWidth = ref(360)
const canvasHeight = ref(500)
const currentStep = ref(-1)
const progress = ref(0)
let animationTimer = null
let resizeObserver = null

const stepDuration = 2000
const dpr = window.devicePixelRatio || 1

const updateCanvasSize = () => {
  if (!containerRef.value) return
  const rect = containerRef.value.getBoundingClientRect()
  canvasWidth.value = rect.width - 32
  canvasHeight.value = rect.height - 32
}

const startAnimation = () => {
  if (!props.steps || props.steps.length === 0) return

  if (currentStep.value >= props.steps.length - 1) {
    currentStep.value = -1
    progress.value = 0
  }

  currentStep.value++
  progress.value = 0

  const stepProgress = setInterval(() => {
    progress.value += (100 / (stepDuration / 50))
    if (progress.value >= 100) {
      progress.value = 100
      clearInterval(stepProgress)
    }
  }, 50)

  animationTimer = setTimeout(() => {
    clearInterval(stepProgress)
    progress.value = 0
    if (currentStep.value < props.steps.length - 1) {
      startAnimation()
    } else {
      emit('pause')
    }
  }, stepDuration)
}

watch(() => props.isAnimating, (val) => {
  if (val) {
    startAnimation()
  } else {
    if (animationTimer) clearTimeout(animationTimer)
  }
})

const reset = () => {
  if (animationTimer) clearTimeout(animationTimer)
  currentStep.value = -1
  progress.value = 0
  emit('reset')
}

const drawCanvas = () => {
  const canvas = canvasRef.value
  if (!canvas) return

  const w = canvasWidth.value
  const h = canvasHeight.value

  canvas.width = w * dpr
  canvas.height = h * dpr

  const ctx = canvas.getContext('2d')
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

  ctx.clearRect(0, 0, w, h)

  ctx.fillStyle = '#1f2937'
  ctx.fillRect(0, 0, w, h)

  if (!props.steps || props.steps.length === 0) {
    ctx.fillStyle = '#6b7280'
    ctx.font = '14px sans-serif'
    ctx.textAlign = 'center'
    ctx.fillText('输入题目后，此处将展示算法执行动画', w / 2, h / 2)
    return
  }

  const padding = 20
  const stepGap = 80
  const barThickness = 28
  const startY = 60

  props.steps.forEach((step, idx) => {
    const y = startY + idx * stepGap

    if (idx <= currentStep.value) {
      ctx.fillStyle = idx === currentStep.value ? '#3b82f6' : '#1e40af'
    } else {
      ctx.fillStyle = '#374151'
    }

    const barWidth = w - 2 * padding
    const filledWidth = idx === currentStep.value ? barWidth * (progress.value / 100) : barWidth

    const radius = 6
    const x = padding
    const bw = idx <= currentStep.value ? filledWidth : barWidth
    const bh = barThickness

    ctx.beginPath()
    ctx.moveTo(x + radius, y)
    ctx.lineTo(x + bw - radius, y)
    ctx.quadraticCurveTo(x + bw, y, x + bw, y + radius)
    ctx.lineTo(x + bw, y + bh - radius)
    ctx.quadraticCurveTo(x + bw, y + bh, x + bw - radius, y + bh)
    ctx.lineTo(x + radius, y + bh)
    ctx.quadraticCurveTo(x, y + bh, x, y + bh - radius)
    ctx.lineTo(x, y + radius)
    ctx.quadraticCurveTo(x, y, x + radius, y)
    ctx.closePath()
    ctx.fill()

    ctx.fillStyle = idx <= currentStep.value ? '#e5e7eb' : '#9ca3af'
    ctx.font = `bold ${Math.max(12, Math.min(14, w / 26))}px sans-serif`
    ctx.textAlign = 'left'
    ctx.fillText(step.text || `步骤 ${idx + 1}`, padding, y - 12)
  })

  if (currentStep.value >= 0 && progress.value > 0) {
    const y = startY + currentStep.value * stepGap + barThickness + 18
    ctx.fillStyle = '#9ca3af'
    ctx.font = '12px sans-serif'
    ctx.textAlign = 'left'
    ctx.fillText(`执行中... ${Math.round(progress.value)}%`, padding, y)
  }
}

onMounted(() => {
  updateCanvasSize()
  drawCanvas()

  resizeObserver = new ResizeObserver(() => {
    updateCanvasSize()
    drawCanvas()
  })
  if (containerRef.value) {
    resizeObserver.observe(containerRef.value)
  }
})

onBeforeUnmount(() => {
  if (resizeObserver) resizeObserver.disconnect()
  if (animationTimer) clearTimeout(animationTimer)
})

watch(() => [props.steps, currentStep.value, progress.value], drawCanvas, { deep: true })
</script>

<template>
  <div class="animation-panel">
    <div class="panel-header">
      <span>动画演示</span>
      <div class="controls" v-if="steps && steps.length > 0">
        <button class="ctrl-btn" @click="$emit('play')" title="播放">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        </button>
        <button class="ctrl-btn" @click="$emit('pause')" title="暂停">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
        </button>
        <button class="ctrl-btn" @click="reset" title="重置">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><rect x="2" y="2" width="20" height="20" rx="2"/></svg>
        </button>
      </div>
    </div>
    <div ref="containerRef" class="canvas-container">
      <canvas ref="canvasRef" :style="{ width: canvasWidth + 'px', height: canvasHeight + 'px' }"></canvas>
    </div>
    <div v-if="steps && steps.length > 0" class="step-indicator">
      <div v-for="(step, idx) in steps" :key="idx"
           :class="['dot', { active: idx <= currentStep, current: idx === currentStep }]"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.animation-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #1a1f2e;
  border-left: 1px solid #2d3748;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: #161b26;
  border-bottom: 1px solid #2d3748;
  font-size: 14px;
  font-weight: 600;
  color: #e5e7eb;
}

.controls {
  display: flex;
  gap: 4px;
}

.ctrl-btn {
  width: 32px;
  height: 32px;
  border: 1px solid #374151;
  border-radius: 6px;
  background: #1f2937;
  color: #9ca3af;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.ctrl-btn:hover {
  background: #374151;
  color: #e5e7eb;
  border-color: #4b5563;
}

.canvas-container {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  overflow: hidden;
}

.canvas-container canvas {
  border-radius: 8px;
  display: block;
}

.step-indicator {
  display: flex;
  justify-content: center;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid #2d3748;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #374151;
  transition: all 0.3s;
}

.dot.active {
  background: #1e40af;
}

.dot.current {
  background: #3b82f6;
  box-shadow: 0 0 6px #3b82f6;
}
</style>
