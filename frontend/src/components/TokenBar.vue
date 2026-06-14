<script setup>
import { inject } from 'vue'

const props = defineProps({ usage: Object })
const llmStatus = inject('llmStatus', null)

const formatTokens = (val) => {
  if (!val || val === 0) return '0'
  if (val >= 1e6) return (val / 1e6).toFixed(1) + 'M'
  if (val >= 1e3) return (val / 1e3).toFixed(1) + 'K'
  return String(val)
}
</script>

<template>
  <div class="token-bar">
    <div class="token-item">
      <span :class="['dot', llmStatus === 'connected' ? 'online' : 'offline']"></span>
      <span class="token-label">{{ llmStatus === 'connected' ? 'LLM 在线' : '模拟模式' }}</span>
    </div>
    <div class="divider"></div>
    <div class="token-item">
      <svg class="token-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>
      </svg>
      <span class="token-value">{{ formatTokens(usage?.sessionTokens) }}</span>
      <span class="token-suffix">/ 本次</span>
    </div>
    <div class="divider"></div>
    <div class="token-item">
      <span class="token-value">{{ formatTokens(usage?.totalTokens) }}</span>
      <span class="token-suffix">/ 累计</span>
    </div>
    <div class="flex-spacer"></div>
    <span class="version">v0.1.0</span>
  </div>
</template>

<style scoped>
.token-bar {
  display: flex; align-items: center; gap: 2px; padding: 0 18px; height: 44px;
  background: rgba(22, 27, 34, 0.9); backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  font-size: 12px; border-top: 1px solid rgba(42,48,60,0.5);
}
.token-item { display: flex; align-items: center; gap: 7px; white-space: nowrap; }
.token-label { color: var(--text-secondary); font-weight: 600; font-size: 12px; }
.token-value { color: var(--text-primary); font-weight: 700; font-variant-numeric: tabular-nums; }
.token-suffix { color: var(--text-muted); font-size: 10px; font-weight: 400; }
.token-icon { color: var(--text-muted); flex-shrink: 0; }
.divider { width: 1px; height: 16px; background: #2a303c; margin: 0 16px; }
.flex-spacer { flex: 1; }
.version { color: var(--text-muted); font-size: 10px; opacity: 0.5; letter-spacing: 0.5px; }
.dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.dot.online { background: #10b981; box-shadow: 0 0 8px rgba(16,185,129,0.5); animation: dotPulse 2s ease-in-out infinite; }
.dot.offline { background: #6b7280; }
@keyframes dotPulse { 0%,100% { opacity:1; } 50% { opacity:0.6; } }
</style>
