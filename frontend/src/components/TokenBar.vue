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
  display: flex; align-items: center; gap: 4px; padding: 0 20px; height: 44px;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  font-size: 12px;
  border-top: 1px solid var(--glass-border);
}
.token-item { display: flex; align-items: center; gap: 8px; white-space: nowrap; }
.token-label { color: var(--text-dim); font-weight: 500; font-size: 11px; }
.token-value { color: var(--text-secondary); font-weight: 600; font-variant-numeric: tabular-nums; }
.token-suffix { color: var(--text-dim); font-size: 10px; font-weight: 400; }
.token-icon { color: var(--text-dim); flex-shrink: 0; }
.divider { width: 1px; height: 16px; background: var(--glass-border); margin: 0 10px; }
.flex-spacer { flex: 1; }
.version { color: var(--text-dim); font-size: 10px; opacity: 0.6; letter-spacing: 0.5px; }
.dot { width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; }
.dot.online { background: var(--success); box-shadow: 0 0 6px var(--success-glow); }
.dot.offline { background: var(--text-dim); }
@keyframes dotPulse { 0%,100% { opacity:1; } 50% { opacity:0.5; } }
</style>
