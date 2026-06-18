<script setup>
import { ref, watch, onBeforeUnmount, onMounted, onUnmounted, computed, inject } from 'vue'
import UniversalRenderer from './anim/UniversalRenderer.vue'

const props = defineProps({ animationData: Array })

const activeAnimIdx = inject('activeAnimIdx', ref(0))

const animList = computed(() => {
  if (!props.animationData || !Array.isArray(props.animationData)) return []
  return props.animationData
})

const currentAnim = computed(() => {
  if (animList.value.length === 0) return null
  const idx = Math.min(activeAnimIdx.value, animList.value.length - 1)
  return animList.value[idx] || null
})

const isPlaying = ref(false)
const currentStepIndex = ref(-1)
const isFullscreen = ref(false)
let animationTimer = null
const speedOptions = [0.5, 1, 1.5, 2, 3]
const speedIdx = ref(1)
const speed = computed(() => speedOptions[speedIdx.value])

const steps = computed(() => currentAnim.value?.frames || [])
const elements = computed(() => currentAnim.value?.elements || [])
const svgW = computed(() => currentAnim.value?.svgW || 400)
const svgH = computed(() => currentAnim.value?.svgH || 250)
const maxStep = computed(() => Math.max(0, steps.value.length - 1))
const stepDuration = computed(() => 2000 / speed.value)
const progress = computed(() => currentStepIndex.value >= 0 ? ((currentStepIndex.value + 1) / steps.value.length * 100) : 0)

const showSizeHint = computed(() => {
  if (steps.value.length === 0) return false
  const elCount = elements.value.length
  return elCount > 10 || svgW.value > 500 || svgH.value > 400
})

const fsScale = computed(() => {
  if (typeof window === 'undefined') return 1
  const pad = 160
  const maxW = window.innerWidth - pad
  const maxH = window.innerHeight - 180
  const sx = maxW / (svgW.value || 1)
  const sy = maxH / (svgH.value || 1)
  return Math.min(sx, sy, 3)
})

const fsWidth = computed(() => svgW.value * fsScale.value)
const fsHeight = computed(() => svgH.value * fsScale.value)

function currentDesc() {
  if (currentStepIndex.value < 0 || currentStepIndex.value >= steps.value.length) return ''
  return steps.value[currentStepIndex.value]?.desc || ''
}

function start() {
  if (steps.value.length === 0) return
  if (currentStepIndex.value >= steps.value.length - 1) currentStepIndex.value = -1
  currentStepIndex.value++
  if (animationTimer) clearTimeout(animationTimer)
  animationTimer = setTimeout(() => {
    if (currentStepIndex.value < steps.value.length - 1 && isPlaying.value) start()
    else isPlaying.value = false
  }, stepDuration.value)
}

function play() { isPlaying.value = true; start() }
function pause() { isPlaying.value = false; if (animationTimer) clearTimeout(animationTimer) }
function stepNext() { if (currentStepIndex.value < maxStep.value) { pause(); currentStepIndex.value++ } }
function stepPrev() { if (currentStepIndex.value > 0) { pause(); currentStepIndex.value-- } }
function reset() { pause(); currentStepIndex.value = -1 }
function goToStep(i) { currentStepIndex.value = i; pause() }
function cycleSpeed() { speedIdx.value = (speedIdx.value + 1) % speedOptions.length }

function enterFullscreen() { isFullscreen.value = true }
function exitFullscreen() { isFullscreen.value = false }

function selectAnim(idx) {
  activeAnimIdx.value = idx
  reset()
  setTimeout(() => play(), 300)
}

watch(() => props.animationData, (data) => {
  if (activeAnimIdx.value >= data?.length) {
    activeAnimIdx.value = 0
  }
  reset()
  if (data?.length) {
    setTimeout(() => play(), 300)
  }
})

function handleKey(e) {
  if (e.code === 'Escape' && isFullscreen.value) { exitFullscreen(); return }
  const tag = document.activeElement?.tagName?.toLowerCase()
  if (tag === 'input' || tag === 'textarea' || tag === 'select' || document.activeElement?.isContentEditable) return
  if (steps.value.length === 0) return
  if (e.code === 'Space') { e.preventDefault(); isPlaying.value ? pause() : play() }
  if (e.code === 'ArrowRight') { e.preventDefault(); stepNext() }
  if (e.code === 'ArrowLeft') { e.preventDefault(); stepPrev() }
  if (e.code === 'KeyR') { e.preventDefault(); reset() }
}

onMounted(() => window.addEventListener('keydown', handleKey))
onUnmounted(() => { window.removeEventListener('keydown', handleKey) })
onBeforeUnmount(() => { if (animationTimer) clearTimeout(animationTimer) })
</script>

<template>
  <div class="animation-panel">
    <div class="panel-header">
      <div class="header-left">
        <div class="header-icon-box">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <polygon points="5 3 19 12 5 21 5 3"/>
          </svg>
        </div>
        <div class="header-text">
          <span class="header-title">动画演示</span>
          <span v-if="steps.length > 0" class="header-badge">{{ steps.length }} 帧</span>
        </div>
      </div>
      <div class="controls" v-if="steps.length > 0">
        <button
          :class="['ctrl-btn', { 'ctrl-fs': showSizeHint }]"
          @click="enterFullscreen"
          :title="showSizeHint ? '全屏查看 (建议)' : '全屏查看'"
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M8 3H5a2 2 0 00-2 2v3m18 0V5a2 2 0 00-2-2h-3m-8 18H5a2 2 0 01-2-2v-3m18 0v3a2 2 0 01-2 2h-3"/>
          </svg>
        </button>
        <button class="ctrl-btn ctrl-speed" @click="cycleSpeed" :title="'速度: ' + speed + 'x'">
          <span class="speed-label">{{ speed }}x</span>
        </button>
        <button class="ctrl-btn" @click="stepPrev" :disabled="currentStepIndex <= 0" title="上一步 (←)">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/></svg>
        </button>
        <button v-if="!isPlaying" class="ctrl-btn ctrl-play" @click="play" title="播放 (Space)">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        </button>
        <button v-else class="ctrl-btn ctrl-play" @click="pause" title="暂停 (Space)">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
        </button>
        <button class="ctrl-btn" @click="stepNext" :disabled="currentStepIndex >= maxStep" title="下一步 (→)">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>
        </button>
        <button class="ctrl-btn" @click="reset" title="重置 (R)">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><polyline points="1 4 1 10 7 10"/><path d="M1 10a11 11 0 0022 0"/></svg>
        </button>
      </div>
    </div>

    <!-- Animation tabs -->
    <div v-if="animList.length > 1" class="anim-tabs">
      <button
        v-for="(anim, idx) in animList"
        :key="idx"
        :class="['anim-tab', { active: idx === activeAnimIdx }]"
        @click="selectAnim(idx)"
      >
        {{ anim.label || ('动画' + (idx+1)) }}
      </button>
    </div>

    <div class="canvas-container">
      <div v-if="steps.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg width="36" height="36" viewBox="0 0 24 24" fill="none" style="stroke:var(--text-dim);opacity:0.3" stroke-width="1.2">
            <rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>
          </svg>
        </div>
        <p v-if="animList.length === 0">输入题目后<br/>此处展示算法动画</p>
        <p v-else>当前动画无帧数据</p>
        <div class="kbd-hints">
          <span><kbd>Space</kbd> 播放</span>
          <span><kbd>← →</kbd> 步进</span>
          <span><kbd>R</kbd> 重置</span>
        </div>
      </div>
      <div v-else class="renderer-area">
        <div class="renderer-frame">
          <UniversalRenderer :elements="elements" :frames="steps" :current-step="currentStepIndex" :svgW="svgW" :svgH="svgH" />
        </div>
        <div v-if="showSizeHint && !isFullscreen" class="size-hint">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <span>元素较多 ({{ elements.length }})，<a href="#" @click.prevent="enterFullscreen">全屏查看</a>效果更佳</span>
        </div>
        <div class="step-desc" v-if="currentDesc()">
          <span class="step-num">{{ currentStepIndex + 1 }}/{{ steps.length }}</span>
          <span>{{ currentDesc() }}</span>
        </div>
      </div>
    </div>

    <div v-if="steps.length > 0" class="progress-bar-wrap">
      <div class="progress-track">
        <div class="progress-bar" :style="{ width: progress + '%' }"></div>
      </div>
      <div class="step-dots">
        <span v-for="(_, i) in steps" :key="i"
          :class="['dot', { done: i <= currentStepIndex, active: i === currentStepIndex }]"
          @click="goToStep(i)"
          :title="steps[i]?.desc"
        ></span>
      </div>
    </div>
  </div>

  <Teleport to="body">
    <div v-if="isFullscreen" class="fs-overlay" @click.self="exitFullscreen">
      <button class="fs-close" @click="exitFullscreen" title="退出全屏 (Esc)">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
          <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>

      <!-- Fullscreen tabs -->
      <div v-if="animList.length > 1" class="fs-anim-tabs">
        <button
          v-for="(anim, idx) in animList"
          :key="idx"
          :class="['fs-anim-tab', { active: idx === activeAnimIdx }]"
          @click="selectAnim(idx)"
        >
          {{ anim.label || ('动画' + (idx+1)) }}
        </button>
      </div>

      <div class="fs-canvas" :style="{ width: fsWidth + 'px', height: fsHeight + 'px' }">
        <UniversalRenderer :elements="elements" :frames="steps" :current-step="currentStepIndex" :svgW="svgW" :svgH="svgH" />
      </div>

      <div class="fs-step-desc" v-if="currentDesc()">
        <span class="step-num">{{ currentStepIndex + 1 }}/{{ steps.length }}</span>
        <span>{{ currentDesc() }}</span>
      </div>

      <div class="fs-controls">
        <button class="fs-ctrl-btn" @click="stepPrev" :disabled="currentStepIndex <= 0" title="上一步 (←)">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/></svg>
        </button>
        <button v-if="!isPlaying" class="fs-ctrl-btn fs-ctrl-play" @click="play" title="播放 (Space)">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        </button>
        <button v-else class="fs-ctrl-btn fs-ctrl-play" @click="pause" title="暂停 (Space)">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
        </button>
        <button class="fs-ctrl-btn" @click="stepNext" :disabled="currentStepIndex >= maxStep" title="下一步 (→)">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>
        </button>
        <button class="fs-ctrl-btn" @click="reset" title="重置 (R)">
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><polyline points="1 4 1 10 7 10"/><path d="M1 10a11 11 0 0022 0"/></svg>
        </button>
        <button class="fs-ctrl-btn fs-ctrl-speed" @click="cycleSpeed" :title="'速度: ' + speed + 'x'">
          <span class="speed-label">{{ speed }}x</span>
        </button>
      </div>

      <div class="fs-progress">
        <div class="fs-progress-track">
          <div class="fs-progress-bar" :style="{ width: progress + '%' }"></div>
        </div>
        <div class="fs-step-dots">
          <span v-for="(_, i) in steps" :key="i"
            :class="['dot', { done: i <= currentStepIndex, active: i === currentStepIndex }]"
            @click="goToStep(i)"
            :title="steps[i]?.desc"
          ></span>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.animation-panel {
  display:flex; flex-direction:column; height:100%;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-left: 1px solid var(--glass-border);
  overflow:hidden;
}

.panel-header {
  display:flex; align-items:center; justify-content:space-between;
  padding:14px 16px;
  border-bottom:1px solid var(--glass-border); flex-shrink:0;
}
.header-left { display:flex; align-items:center; gap:12px; }
.header-icon-box {
  width:30px; height:30px; border-radius:var(--radius-xs);
  background: rgba(59,130,246,0.12);
  display:flex; align-items:center; justify-content:center; color:var(--accent);
}
.header-text { display:flex; flex-direction:column; gap:2px; }
.header-title { font-size:13px; font-weight:600; color:var(--text-primary); }
.header-badge {
  font-size:10px; color:var(--text-dim); letter-spacing:0.3px;
}

.controls { display:flex; gap:4px; align-items:center; }
.ctrl-btn {
  width:32px; height:32px;
  border:1px solid var(--glass-border); border-radius:var(--radius-sm);
  background: var(--glass-hover); color:var(--text-dim); cursor:pointer;
  display:flex; align-items:center; justify-content:center;
  transition:all var(--transition-fast);
}
.ctrl-btn:hover:not(:disabled) {
  background: var(--glass-active); color:var(--text-primary);
  border-color:rgba(255,255,255,0.12); transform:translateY(-1px);
}
.ctrl-btn:active:not(:disabled) { transform:scale(0.95); }
.ctrl-btn:disabled { opacity:0.25; cursor:not-allowed; }
.ctrl-play {
  background:rgba(59,130,246,0.12); border-color:rgba(59,130,246,0.2); color:var(--accent-light);
}
.ctrl-play:hover:not(:disabled) {
  background:rgba(59,130,246,0.2); border-color:rgba(59,130,246,0.35);
  box-shadow: 0 0 12px rgba(59,130,246,0.15);
}
.ctrl-speed { width:auto; padding:0 10px; font-size:11px; font-weight:700; letter-spacing:0.5px; }
.ctrl-speed:hover { color:var(--accent-light); border-color:rgba(59,130,246,0.25); }
.ctrl-fs { color:var(--accent-light); border-color:rgba(59,130,246,0.2); background:rgba(59,130,246,0.06); }
.ctrl-fs:hover {
  background:rgba(59,130,246,0.14); border-color:rgba(59,130,246,0.35);
  box-shadow:0 0 12px rgba(59,130,246,0.12);
}
.speed-label { font-variant-numeric:tabular-nums; }

/* Animation Tabs */
.anim-tabs {
  display: flex; gap:4px; padding:8px 16px; overflow-x:auto;
  border-bottom:1px solid var(--glass-border); flex-shrink:0;
}
.anim-tab {
  padding:6px 14px; border:1px solid var(--glass-border); border-radius:20px;
  background: transparent; color:var(--text-dim); font-size:11px; font-weight:500;
  cursor:pointer; transition:all var(--transition-fast); white-space:nowrap;
}
.anim-tab:hover {
  color:var(--text-secondary);
  background: var(--glass-hover);
  border-color: rgba(255,255,255,0.1);
}
.anim-tab:active { transform: scale(0.95); }
.anim-tab.active {
  color:var(--accent-light);
  background: rgba(59,130,246,0.1);
  border-color:rgba(59,130,246,0.3);
  box-shadow: 0 0 16px rgba(59,130,246,0.08);
}

.canvas-container {
  flex:1; min-height:0;
  display:flex; align-items:center; justify-content:center;
  overflow:hidden; padding:14px;
}

.empty-state { text-align:center; animation: fadeIn 0.5s ease; }
.empty-icon {
  margin:0 auto 20px; display:flex; align-items:center; justify-content:center;
  width:80px; height:80px; border-radius:50%;
  background: rgba(48,54,61,0.2); border: 1px solid var(--glass-border);
}
.empty-state p { font-size:12px; color:var(--text-dim); line-height:1.7; margin-bottom:16px; }

.kbd-hints { display:flex; gap:14px; justify-content:center; flex-wrap:wrap; }
.kbd-hints span { font-size:10px; color:var(--text-dim); display:flex; align-items:center; gap:4px; }
.kbd-hints kbd {
  padding:2px 7px;
  background: var(--glass-hover); border:1px solid var(--glass-border);
  border-radius:4px; font-size:10px; font-family:var(--font-mono); color:var(--text-dim);
}

.renderer-area {
  display:flex; flex-direction:column; align-items:center; gap:10px;
  width:100%; max-height:100%; overflow:hidden;
}
.renderer-frame {
  display:flex; align-items:center; justify-content:center;
  width:100%; max-height:100%; overflow:hidden;
  background: rgba(12,17,25,0.6);
  border-radius:var(--radius-md);
  padding:10px;
  border:1px solid var(--glass-border);
  box-shadow: var(--shadow-md), inset 0 1px 0 rgba(255,255,255,0.02);
}

.size-hint {
  display:flex; align-items:center; gap:8px; flex-shrink:0;
  font-size:11px; color:var(--warning);
  background: rgba(245,158,11,0.06);
  border:1px solid rgba(245,158,11,0.15); border-radius:var(--radius-sm);
  padding:8px 14px;
}
.size-hint a { color:var(--accent-light); text-decoration:none; font-weight:600; }
.size-hint a:hover { text-decoration:underline; }

.step-desc {
  text-align:center; font-size:13px; color:var(--text-secondary); line-height:1.5; flex-shrink:0;
  padding:10px 16px;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur)); -webkit-backdrop-filter: blur(var(--glass-blur));
  border-radius:var(--radius-sm);
  border:1px solid var(--glass-border);
  max-width:95%;
}
.step-num {
  font-size:11px; color:var(--accent-light); margin-right:8px; font-weight:700;
  background:rgba(59,130,246,0.1); padding:2px 10px; border-radius:12px;
}

.progress-bar-wrap {
  flex-shrink:0; padding:12px 16px 16px;
  border-top:1px solid var(--glass-border);
}
.progress-track {
  height:3px; background:var(--glass-hover); border-radius:2px;
  overflow:hidden; margin-bottom:12px;
}
.progress-bar {
  height:100%;
  background: var(--gradient-brand);
  border-radius:2px;
  transition:width 0.4s cubic-bezier(0.4,0,0.2,1);
  box-shadow:0 0 8px rgba(59,130,246,0.3);
}

.step-dots { display:flex; justify-content:center; gap:8px; }
.dot {
  width:8px; height:8px; border-radius:50%;
  background:rgba(255,255,255,0.06); cursor:pointer;
  transition:all 0.3s cubic-bezier(0.4,0,0.2,1);
}
.dot:hover { background:rgba(255,255,255,0.15); transform:scale(1.3); }
.dot.done { background:rgba(59,130,246,0.4); }
.dot.active {
  background:var(--accent);
  box-shadow:0 0 12px rgba(59,130,246,0.6); transform:scale(1.4);
}

/* Fullscreen */
.fs-overlay {
  position:fixed; inset:0; z-index:9999;
  background:rgba(3,6,12,0.96);
  backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px);
  display:flex; flex-direction:column; align-items:center; justify-content:center;
  gap:18px;
  animation: fsFadeIn 0.3s ease;
}
@keyframes fsFadeIn { from { opacity:0; } to { opacity:1; } }

.fs-close {
  position:absolute; top:24px; right:24px;
  width:44px; height:44px;
  border:1px solid rgba(255,255,255,0.08); border-radius:var(--radius-sm);
  background:rgba(255,255,255,0.04); color:var(--text-dim); cursor:pointer;
  display:flex; align-items:center; justify-content:center;
  transition:all var(--transition-fast);
}
.fs-close:hover { background:rgba(255,255,255,0.08); color:var(--text-primary); border-color:rgba(255,255,255,0.15); }

.fs-anim-tabs {
  display:flex; gap:6px; padding:6px 20px;
  background:rgba(255,255,255,0.02); border:1px solid rgba(255,255,255,0.05);
  border-radius:var(--radius-sm);
}
.fs-anim-tab {
  padding:8px 20px; border:1px solid rgba(255,255,255,0.05); border-radius:20px;
  background:transparent; color:var(--text-dim); font-size:13px; font-weight:500;
  cursor:pointer; transition:all var(--transition-fast); white-space:nowrap;
}
.fs-anim-tab:hover { color:var(--text-primary); background:rgba(255,255,255,0.04); }
.fs-anim-tab.active {
  color:var(--accent-light); background:rgba(59,130,246,0.12);
  border-color:rgba(59,130,246,0.35);
}

.fs-canvas {
  background: var(--bg-panel); border-radius: var(--radius-lg);
  padding: 20px;
  border: 1px solid var(--glass-border);
  box-shadow: 0 8px 48px rgba(0,0,0,0.5), 0 0 0 1px rgba(59,130,246,0.04);
  max-width:calc(100vw - 80px); max-height:calc(100vh - 280px);
  display:flex; align-items:center; justify-content:center;
  overflow:hidden;
}

.fs-step-desc {
  text-align:center; font-size:15px; color:var(--text-secondary); line-height:1.6;
  padding:14px 28px;
  background:rgba(255,255,255,0.03); border-radius:var(--radius-sm);
  border:1px solid rgba(255,255,255,0.05); max-width:90%;
}
.fs-step-desc .step-num {
  font-size:12px; color:var(--accent-light); margin-right:10px; font-weight:700;
  background:rgba(59,130,246,0.1); padding:3px 12px; border-radius:12px;
}

.fs-controls {
  display:flex; gap:8px; align-items:center;
  padding:12px 24px;
  background:rgba(255,255,255,0.02); border:1px solid rgba(255,255,255,0.05);
  border-radius:var(--radius-sm);
}

.fs-ctrl-btn {
  width:44px; height:44px;
  border:1px solid rgba(255,255,255,0.06); border-radius:var(--radius-sm);
  background:rgba(255,255,255,0.03); color:var(--text-dim); cursor:pointer;
  display:flex; align-items:center; justify-content:center;
  transition:all var(--transition-fast);
}
.fs-ctrl-btn:hover:not(:disabled) {
  background:rgba(255,255,255,0.08); color:var(--text-primary);
  border-color:rgba(255,255,255,0.12); transform:translateY(-1px);
}
.fs-ctrl-btn:active:not(:disabled) { transform:scale(0.95); }
.fs-ctrl-btn:disabled { opacity:0.2; cursor:not-allowed; }
.fs-ctrl-play {
  background:rgba(59,130,246,0.15); border-color:rgba(59,130,246,0.25); color:var(--accent-light);
}
.fs-ctrl-play:hover:not(:disabled) {
  background:rgba(59,130,246,0.25); border-color:rgba(59,130,246,0.4);
  box-shadow:0 0 16px rgba(59,130,246,0.2);
}
.fs-ctrl-speed { width:auto; padding:0 16px; font-size:13px; font-weight:700; letter-spacing:0.5px; color:var(--text-dim); }
.fs-ctrl-speed:hover:not(:disabled) { color:var(--accent-light); border-color:rgba(59,130,246,0.25); }

.fs-progress {
  padding:10px 24px 0;
  width:100%; max-width:640px;
}
.fs-progress-track {
  height:4px; background:rgba(255,255,255,0.05); border-radius:2px;
  overflow:hidden; margin-bottom:12px;
}
.fs-progress-bar {
  height:100%;
  background: var(--gradient-brand);
  border-radius:2px;
  transition:width 0.4s cubic-bezier(0.4,0,0.2,1);
  box-shadow:0 0 10px rgba(59,130,246,0.4);
}
.fs-step-dots { display:flex; justify-content:center; gap:10px; }

@media (max-width: 480px) {
  .canvas-container { padding: 8px; }
  .renderer-frame { padding: 6px; }
  .step-desc { font-size: 11px; padding: 8px 12px; }
  .panel-header { padding: 10px 12px; }
  .controls { gap: 2px; }
  .ctrl-btn { width: 28px; height: 28px; }
  .anim-tabs { padding: 6px 8px; }
  .anim-tab { padding: 4px 10px; font-size: 10px; }
}
</style>
