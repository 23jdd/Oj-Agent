<script setup>
import { ref, onMounted, inject } from 'vue'
import { UpdateLLMConfig, GetLLMStatus } from '../../bindings/Oj-Agent/chatservice'

const emit = defineEmits(['close', 'updated'])
const modelRef = inject('selectedModel')
const llmStatusRef = inject('llmStatus')
const tokenUsage = inject('tokenUsage')

const apiKey = ref('')
const baseURL = ref('https://api.deepseek.com')
const llmModel = ref('deepseek-chat')
const showKey = ref(false)
const status = ref('')
const testing = ref(false)

onMounted(async () => {
  try {
    const raw = await GetLLMStatus()
    try {
      const info = JSON.parse(raw)
      status.value = info.status === 'connected' ? 'connected' : 'mock'
    } catch {
      status.value = raw === 'connected' ? 'connected' : 'mock'
    }
  } catch (e) { status.value = 'mock' }

  const saved = localStorage.getItem('oj-agent-config')
  if (saved) {
    try {
      const cfg = JSON.parse(saved)
      if (cfg.apiKey) apiKey.value = cfg.apiKey
      if (cfg.baseURL) baseURL.value = cfg.baseURL
      if (cfg.model) llmModel.value = cfg.model
    } catch (e) {}
  }
})

const saveConfig = async () => {
  testing.value = true
  status.value = 'testing'

  localStorage.setItem('oj-agent-config', JSON.stringify({
    apiKey: apiKey.value,
    baseURL: baseURL.value,
    model: llmModel.value
  }))

  try {
    const result = await UpdateLLMConfig(apiKey.value, baseURL.value, llmModel.value)
    status.value = result.includes('成功') || result.includes('连接成功') ? 'connected' : 'error'
    modelRef.value = llmModel.value
    if (llmStatusRef) llmStatusRef.value = status.value === 'connected' ? 'connected' : 'mock'
    emit('updated', { apiKey: apiKey.value, baseURL: baseURL.value, model: llmModel.value })
  } catch (e) {
    status.value = 'error'
    console.error('UpdateLLMConfig error:', e)
  } finally {
    testing.value = false
  }
}

const clearConfig = async () => {
  apiKey.value = ''
  localStorage.removeItem('oj-agent-config')
  await UpdateLLMConfig('', '', '')
  status.value = 'mock'
  if (llmStatusRef) llmStatusRef.value = 'mock'
  emit('updated', { apiKey: '', baseURL: '', model: 'deepseek-chat' })
}

</script>

<template>
  <div class="settings-overlay" @click.self="$emit('close')">
    <div class="settings-modal">
      <div class="modal-header">
        <div class="modal-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"/><path d="M12 1v2m0 18v2M4.22 4.22l1.42 1.42m12.72 12.72l1.42 1.42M1 12h2m18 0h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
          </svg>
          <span>模型设置</span>
        </div>
        <button class="close-btn" @click="$emit('close')">&times;</button>
      </div>

      <div class="modal-body">
        <div class="field">
          <label>API Key</label>
          <div class="input-wrap">
            <input
              :type="showKey ? 'text' : 'password'"
              v-model="apiKey"
              placeholder="sk-xxxxxxxx"
              class="field-input mono"
            />
            <button class="toggle-key" @click="showKey = !showKey" :title="showKey ? '隐藏' : '显示'">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path v-if="showKey" d="M17.94 17.94A10.07 10.07 0 0112 20c-7 0-11-8-11-8a18.45 18.45 0 015.06-5.94M9.9 4.24A9.12 9.12 0 0112 4c7 0 11 8 11 8a18.5 18.5 0 01-2.16 3.19m-6.72-1.07a3 3 0 11-4.24-4.24"/>
                <path v-else d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/>
              </svg>
            </button>
          </div>
        </div>

        <div class="field">
          <label>Base URL</label>
          <input v-model="baseURL" placeholder="https://api.deepseek.com" class="field-input mono" />
        </div>

        <div class="field">
          <label>模型名称</label>
          <input v-model="llmModel" placeholder="deepseek-chat" class="field-input" />
        </div>

        <div class="actions">
          <button class="btn btn-primary" @click="saveConfig" :disabled="testing || !apiKey">
            <svg v-if="testing" class="spin" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <path d="M21 12a9 9 0 11-6.219-8.56"/>
            </svg>
            <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            保存并测试连接
          </button>
          <button class="btn btn-ghost" @click="clearConfig">清除配置</button>
        </div>

        <div v-if="status" :class="['status-bar', status]">
          <span class="status-dot"></span>
          <span v-if="status === 'connected'">已连接 — {{ llmModel }}</span>
          <span v-else-if="status === 'testing'">正在测试连接...</span>
          <span v-else-if="status === 'error'">连接失败，使用模拟数据</span>
          <span v-else>模拟模式 — 未配置 API Key</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.settings-overlay {
  position: fixed; inset: 0; z-index: 1000;
  background: rgba(0,0,0,0.65);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
  display: flex; align-items: center; justify-content: center;
  animation: fadeIn 0.2s ease;
}
@keyframes fadeIn { from { opacity: 0 } to { opacity: 1 } }

.settings-modal {
  width: 480px; max-width: 90vw;
  background: var(--glass-bg);
  backdrop-filter: blur(24px);
  -webkit-backdrop-filter: blur(24px);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-lg);
  box-shadow: var(--glass-shadow), 0 0 0 1px rgba(255,255,255,0.04);
  animation: slideUp 0.25s cubic-bezier(0.4,0,0.2,1);
}
@keyframes slideUp { from { opacity:0; transform:translateY(16px) } to { opacity:1; transform:translateY(0) } }

.modal-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 18px 22px;
  border-bottom: 1px solid var(--glass-border);
  background: var(--glass-hover);
}
.modal-title { display: flex; align-items: center; gap: 10px; font-size: 15px; font-weight: 600; color: var(--text-primary); }
.modal-title svg { color: var(--accent); }
.close-btn {
  width: 32px; height: 32px; border: none; background: transparent;
  color: var(--text-muted); font-size: 22px; cursor: pointer;
  border-radius: var(--radius-sm); display: flex; align-items: center; justify-content: center;
  transition: all var(--transition-fast);
}
.close-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

.modal-body { padding: 22px; display: flex; flex-direction: column; gap: 18px; }

.field { display: flex; flex-direction: column; gap: 6px; }
.field label { font-size: 12px; font-weight: 600; color: var(--text-secondary); letter-spacing: 0.3px; text-transform: uppercase; }
.field-input {
  padding: 10px 14px;
  background: rgba(255,255,255,0.03);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md); color: var(--text-primary); font-size: 14px;
  font-family: var(--font-main); outline: none; transition: all var(--transition-fast);
}
.field-input:focus {
  border-color: var(--border-focus);
  background: rgba(59,130,246,0.04);
  box-shadow: 0 0 0 3px rgba(59,130,246,0.1);
}
.field-input::placeholder { color: var(--text-muted); }
.field-input.mono { font-family: var(--font-mono); font-size: 13px; }

.input-wrap { position: relative; }
.input-wrap .field-input { padding-right: 40px; width: 100%; }
.toggle-key {
  position: absolute; right: 4px; top: 50%; transform: translateY(-50%);
  width: 32px; height: 32px; border: none; background: transparent;
  color: var(--text-muted); cursor: pointer; border-radius: var(--radius-sm);
  display: flex; align-items: center; justify-content: center;
}
.toggle-key:hover { color: var(--text-secondary); }

.actions { display: flex; gap: 10px; }
.btn {
  padding: 10px 20px; border-radius: var(--radius-md); font-size: 13px; font-weight: 600;
  cursor: pointer; border: none; display: flex; align-items: center; gap: 8px;
  transition: all var(--transition-fast);
}
.btn-primary {
  background: linear-gradient(135deg, var(--accent), #2563eb);
  color: #fff; box-shadow: 0 2px 8px rgba(59,130,246,0.25);
}
.btn-primary:hover:not(:disabled) { transform: translateY(-1px); box-shadow: 0 4px 16px rgba(59,130,246,0.35); }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-ghost { background: transparent; color: var(--text-muted); border: 1px solid var(--border-subtle); }
.btn-ghost:hover { background: var(--bg-hover); color: var(--text-secondary); }

.status-bar {
  display: flex; align-items: center; gap: 8px;
  padding: 10px 14px; border-radius: var(--radius-md);
  font-size: 12px; font-weight: 500;
}
.status-bar.connected { background: rgba(16,185,129,0.1); color: #10b981; }
.status-bar.testing { background: rgba(59,130,246,0.1); color: var(--accent); }
.status-bar.error { background: rgba(239,68,68,0.1); color: #f87171; }
.status-bar.mock { background: var(--bg-main); color: var(--text-muted); }
.status-dot {
  width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0;
}
.connected .status-dot { background: #10b981; box-shadow: 0 0 6px rgba(16,185,129,0.5); }
.testing .status-dot { background: var(--accent); animation: dotPulse 1s infinite; }
.error .status-dot { background: #ef4444; }
.mock .status-dot { background: #6b7280; }

@keyframes dotPulse { 0%,100% { opacity:1 } 50% { opacity:0.3 } }
.spin { animation: spin 1s linear infinite; }
@keyframes spin { from { transform:rotate(0deg) } to { transform:rotate(360deg) } }
</style>
