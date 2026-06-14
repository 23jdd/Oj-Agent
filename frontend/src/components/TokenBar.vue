<script setup>
import { computed } from 'vue'

const props = defineProps({
  usage: Object
})

const formatTokens = (val) => {
  if (!val || val === 0) return '0'
  if (val >= 1000000) return (val / 1000000).toFixed(1) + 'M'
  if (val >= 1000) return (val / 1000).toFixed(1) + 'K'
  return String(val)
}
</script>

<template>
  <div class="token-bar">
    <div class="token-item">
      <span class="token-label">模型</span>
      <span class="token-value">GPT-4o</span>
    </div>
    <div class="divider"></div>
    <div class="token-item">
      <span class="token-label">本次会话</span>
      <span class="token-value">{{ formatTokens(usage?.sessionTokens) }} tokens</span>
    </div>
    <div class="divider"></div>
    <div class="token-item">
      <span class="token-label">累计消耗</span>
      <span class="token-value">{{ formatTokens(usage?.totalTokens) }} tokens</span>
    </div>
    <div class="spacer"></div>
    <div class="token-item">
      <span class="token-label">OJ Agent v0.1</span>
    </div>
  </div>
</template>

<style scoped>
.token-bar {
  display: flex;
  align-items: center;
  padding: 0 16px;
  height: 40px;
  background: #0f172a;
  border-top: 1px solid #1e293b;
  font-size: 12px;
}

.token-item {
  display: flex;
  align-items: center;
  gap: 6px;
}

.token-label {
  color: #64748b;
}

.token-value {
  color: #94a3b8;
  font-weight: 500;
}

.divider {
  width: 1px;
  height: 16px;
  background: #334155;
  margin: 0 14px;
}

.spacer {
  flex: 1;
}
</style>
