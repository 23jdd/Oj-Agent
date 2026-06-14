<script setup>
import { ref, watch, onMounted, onBeforeUnmount, computed } from 'vue'
import ArrayRenderer from './anim/ArrayRenderer.vue'
import TwoPointerRenderer from './anim/TwoPointerRenderer.vue'
import TreeRenderer from './anim/TreeRenderer.vue'
import DpTableRenderer from './anim/DpTableRenderer.vue'

const props = defineProps({
  animationData: Object,
})

const emit = defineEmits(['play', 'pause', 'reset'])

const isPlaying = ref(false)
const currentStepIndex = ref(-1)
let animationTimer = null
const stepDuration = 2000

const steps = computed(() => props.animationData?.steps || [])
const animType = computed(() => props.animationData?.type || '')
const arrayData = computed(() => props.animationData?.array || null)
const treeData = computed(() => props.animationData?.tree || null)
const tableData = computed(() => props.animationData?.table || null)
const currentStep = computed(() => steps.value[currentStepIndex.value] || null)

const maxStepIndex = computed(() => Math.max(0, steps.value.length - 1))
const progress = computed(() => currentStepIndex.value >= 0 ? ((currentStepIndex.value + 1) / steps.value.length * 100) : 0)

const animState = computed(() => ({
  activeIdx: currentStepIndex.value,
  totalSteps: steps.value.length,
}))

function startAnimation() {
  if (steps.value.length === 0) return

  if (currentStepIndex.value >= steps.value.length - 1) {
    currentStepIndex.value = -1
  }

  currentStepIndex.value++
  if (animationTimer) clearTimeout(animationTimer)

  animationTimer = setTimeout(() => {
    if (currentStepIndex.value < steps.value.length - 1 && isPlaying.value) {
      startAnimation()
    } else {
      pause()
    }
  }, stepDuration)
}

function play() {
  isPlaying.value = true
  startAnimation()
}

function pause() {
  isPlaying.value = false
  if (animationTimer) clearTimeout(animationTimer)
}

function stepForward() {
  if (currentStepIndex.value < steps.value.length - 1) {
    currentStepIndex.value++
  }
}

function stepBackward() {
  if (currentStepIndex.value > 0) {
    currentStepIndex.value--
  }
}

function reset() {
  pause()
  currentStepIndex.value = -1
}

watch(() => props.animationData, () => {
  reset()
})

onMounted(() => {
})

onBeforeUnmount(() => {
  if (animationTimer) clearTimeout(animationTimer)
})

const rendererComponent = computed(() => {
  switch (animType.value) {
    case 'twopointer': return TwoPointerRenderer
    case 'tree': return TreeRenderer
    case 'dptable': return DpTableRenderer
    case 'array':
    default: return ArrayRenderer
  }
})
</script>

<template>
  <div class="animation-panel">
    <div class="panel-header">
      <span>
        动画演示
        <span v-if="animType" class="type-badge">{{ animType }}</span>
      </span>
      <div class="controls" v-if="steps.length > 0">
        <button class="ctrl-btn" @click="stepBackward" :disabled="currentStepIndex <= 0" title="上一步">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/></svg>
        </button>
        <button v-if="!isPlaying" class="ctrl-btn" @click="play" title="播放">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        </button>
        <button v-else class="ctrl-btn" @click="pause" title="暂停">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
        </button>
        <button class="ctrl-btn" @click="stepForward" :disabled="currentStepIndex >= maxStepIndex" title="下一步">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>
        </button>
        <button class="ctrl-btn reset-btn" @click="reset" title="重置">&#8634;</button>
      </div>
    </div>

    <div class="canvas-container">
      <div v-if="steps.length === 0" class="empty-state">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="#4b5563" stroke-width="1.5">
          <rect x="2" y="3" width="20" height="14" rx="2"/>
          <path d="M8 21h8M12 17v4"/>
        </svg>
        <p>输入算法题目后，此处将展示解题动画</p>
      </div>

      <div v-else class="renderer-area">
        <component
          :is="rendererComponent"
          :step="currentStep"
          :array-data="arrayData"
          :tree-data="treeData"
          :table-data="tableData"
          :anim-state="animState"
        />
        <div class="step-description" v-if="currentStep">
          <span class="step-num">{{ currentStepIndex + 1 }} / {{ steps.length }}</span>
          {{ currentStep.description }}
        </div>
      </div>
    </div>

    <!-- Progress bar -->
    <div v-if="steps.length > 0" class="progress-bar-wrap">
      <div class="progress-bar" :style="{ width: progress + '%' }"></div>
      <div class="step-dots">
        <span
          v-for="(_, i) in steps"
          :key="i"
          :class="['dot', { done: i <= currentStepIndex, active: i === currentStepIndex }]"
          @click="currentStepIndex = i; pause()"
        ></span>
      </div>
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
  flex-shrink: 0;
}

.type-badge {
  font-size: 11px;
  font-weight: normal;
  background: #374151;
  color: #9ca3af;
  padding: 2px 8px;
  border-radius: 10px;
  margin-left: 8px;
  text-transform: uppercase;
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
  font-size: 16px;
}

.ctrl-btn:hover:not(:disabled) {
  background: #374151;
  color: #e5e7eb;
  border-color: #4b5563;
}

.ctrl-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.reset-btn {
  font-family: sans-serif;
  font-size: 18px;
}

.canvas-container {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: auto;
  padding: 16px;
}

.empty-state {
  text-align: center;
  color: #6b7280;
}

.empty-state svg {
  margin: 0 auto 12px;
  display: block;
}

.empty-state p {
  font-size: 13px;
  max-width: 200px;
  line-height: 1.5;
}

.renderer-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  width: 100%;
}

.step-description {
  text-align: center;
  font-size: 13px;
  color: #d1d5db;
  line-height: 1.4;
  padding: 8px 16px;
  background: #111827;
  border-radius: 8px;
  max-width: 90%;
}

.step-num {
  font-size: 11px;
  color: #6b7280;
  margin-right: 8px;
}

.progress-bar-wrap {
  flex-shrink: 0;
  padding: 8px 16px 12px;
  border-top: 1px solid #2d3748;
  position: relative;
}

.progress-bar {
  height: 3px;
  background: #3b82f6;
  border-radius: 2px;
  transition: width 0.3s ease;
  margin-bottom: 8px;
}

.step-dots {
  display: flex;
  justify-content: center;
  gap: 6px;
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #374151;
  cursor: pointer;
  transition: all 0.3s;
}

.dot:hover {
  background: #6b7280;
}

.dot.done {
  background: #1e40af;
}

.dot.active {
  background: #3b82f6;
  box-shadow: 0 0 6px #3b82f6;
  transform: scale(1.3);
}
</style>
