<script setup>
import { ref, watch, onBeforeUnmount, computed } from 'vue'
import UniversalRenderer from './anim/UniversalRenderer.vue'

const props = defineProps({ animationData: Object })

const isPlaying = ref(false)
const currentStepIndex = ref(-1)
let animationTimer = null
const stepDuration = 2000

const steps = computed(() => props.animationData?.frames || [])
const elements = computed(() => props.animationData?.elements || [])
const svgW = computed(() => props.animationData?.svgW || 400)
const svgH = computed(() => props.animationData?.svgH || 250)
const maxStep = computed(() => Math.max(0, steps.value.length - 1))
const progress = computed(() => currentStepIndex.value >= 0 ? ((currentStepIndex.value + 1) / steps.value.length * 100) : 0)

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
  }, stepDuration)
}

function play() { isPlaying.value = true; start() }
function pause() { isPlaying.value = false; if (animationTimer) clearTimeout(animationTimer) }
function stepNext() { if (currentStepIndex.value < maxStep.value) currentStepIndex.value++ }
function stepPrev() { if (currentStepIndex.value > 0) currentStepIndex.value-- }
function reset() { pause(); currentStepIndex.value = -1 }

watch(() => props.animationData, () => reset())
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
        <button class="ctrl-btn" @click="stepPrev" :disabled="currentStepIndex <= 0" title="上一步">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 6h2v12H6zm3.5 6l8.5 6V6z"/></svg>
        </button>
        <button v-if="!isPlaying" class="ctrl-btn ctrl-play" @click="play" title="播放">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        </button>
        <button v-else class="ctrl-btn ctrl-play" @click="pause" title="暂停">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/></svg>
        </button>
        <button class="ctrl-btn" @click="stepNext" :disabled="currentStepIndex >= maxStep" title="下一步">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><path d="M6 18l8.5-6L6 6v12zM16 6v12h2V6h-2z"/></svg>
        </button>
        <button class="ctrl-btn" @click="reset" title="重置">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><polyline points="1 4 1 10 7 10"/><path d="M1 10a11 11 0 0022 0"/></svg>
        </button>
      </div>
    </div>

    <div class="canvas-container">
      <div v-if="steps.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="#30363d" stroke-width="1.2">
            <rect x="2" y="3" width="20" height="14" rx="2"/><path d="M8 21h8M12 17v4"/>
          </svg>
        </div>
        <p>输入题目后<br/>此处展示算法动画</p>
      </div>
      <div v-else class="renderer-area">
        <div class="renderer-frame">
          <UniversalRenderer :elements="elements" :frames="steps" :current-step="currentStepIndex" :svgW="svgW" :svgH="svgH" />
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
          @click="currentStepIndex = i; pause()"
          :title="steps[i]?.desc"
        ></span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.animation-panel { display:flex; flex-direction:column; height:100%; background:linear-gradient(180deg, #161b22 0%, #131820 100%); overflow:hidden; }

.panel-header {
  display:flex; align-items:center; justify-content:space-between;
  padding:12px 16px; background:rgba(0,0,0,0.12); border-bottom:1px solid var(--border-subtle); flex-shrink:0;
}
.header-left { display:flex; align-items:center; gap:10px; }
.header-icon-box {
  width:28px; height:28px; border-radius:var(--radius-sm); background:rgba(59,130,246,0.12);
  display:flex; align-items:center; justify-content:center; color:var(--accent);
}
.header-text { display:flex; flex-direction:column; gap:1px; }
.header-title { font-size:13px; font-weight:600; color:var(--text-primary); }
.header-badge { font-size:10px; color:var(--text-muted); letter-spacing:0.3px; }

.controls { display:flex; gap:3px; align-items:center; }
.ctrl-btn {
  width:30px; height:30px; border:1px solid var(--border-subtle); border-radius:var(--radius-sm);
  background:var(--bg-elevated); color:var(--text-secondary); cursor:pointer;
  display:flex; align-items:center; justify-content:center; transition:all 0.2s ease;
}
.ctrl-btn:hover:not(:disabled) { background:var(--bg-hover); color:var(--text-primary); border-color:#484f58; transform:translateY(-1px); }
.ctrl-btn:active:not(:disabled) { transform:scale(0.95); }
.ctrl-btn:disabled { opacity:0.3; cursor:not-allowed; }
.ctrl-play { background:rgba(59,130,246,0.15); border-color:rgba(59,130,246,0.25); color:var(--accent); }
.ctrl-play:hover:not(:disabled) { background:rgba(59,130,246,0.25); border-color:rgba(59,130,246,0.4); }

.canvas-container { flex:1; display:flex; align-items:center; justify-content:center; overflow:auto; padding:16px; min-height:0; }

.empty-state { text-align:center; }
.empty-icon { margin:0 auto 16px; display:flex; align-items:center; justify-content:center; width:72px; height:72px; border-radius:50%; background:rgba(48,54,61,0.3); }
.empty-state p { font-size:12px; color:var(--text-muted); line-height:1.7; }

.renderer-area { display:flex; flex-direction:column; align-items:center; gap:12px; width:100%; }
.renderer-frame {
  width:100%; background:var(--bg-panel); border-radius:var(--radius-lg);
  padding:8px; border:1px solid var(--border-subtle);
  box-shadow: 0 2px 12px rgba(0,0,0,0.2), 0 0 0 1px rgba(59,130,246,0.03);
}

.step-desc {
  text-align:center; font-size:13px; color:var(--text-secondary); line-height:1.5;
  padding:10px 16px; background:var(--bg-elevated); border-radius:var(--radius-md);
  border:1px solid var(--border-subtle); max-width:95%;
}
.step-num {
  font-size:11px; color:var(--accent); margin-right:8px; font-weight:700;
  background:rgba(59,130,246,0.1); padding:2px 8px; border-radius:10px;
}

.progress-bar-wrap { flex-shrink:0; padding:10px 16px 14px; border-top:1px solid var(--border-subtle); }
.progress-track { height:3px; background:var(--bg-elevated); border-radius:2px; overflow:hidden; margin-bottom:10px; }
.progress-bar { height:100%; background:linear-gradient(90deg, var(--accent), #60a5fa); border-radius:2px; transition:width 0.4s cubic-bezier(0.4,0,0.2,1); box-shadow:0 0 6px rgba(59,130,246,0.3); }

.step-dots { display:flex; justify-content:center; gap:6px; }
.dot { width:7px; height:7px; border-radius:50%; background:#30363d; cursor:pointer; transition:all 0.3s cubic-bezier(0.4,0,0.2,1); }
.dot:hover { background:#484f58; transform:scale(1.3); }
.dot.done { background:#1e40af; }
.dot.active { background:var(--accent); box-shadow:0 0 10px rgba(59,130,246,0.7); transform:scale(1.5); }
</style>
