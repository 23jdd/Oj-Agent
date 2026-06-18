<script setup>
import Sidebar from './components/Sidebar.vue'
import ChatArea from './components/ChatArea.vue'
import AnimationPanel from './components/AnimationPanel.vue'
import TokenBar from './components/TokenBar.vue'
import SettingsModal from './components/SettingsModal.vue'
import { ref, reactive, computed, watch, provide, onMounted } from 'vue'
import { GetLLMStatus, GetSession } from '../bindings/Oj-Agent/chatservice'

const activeSessionId = ref('')
const sessions = ref([])
const tokenUsage = ref({ sessionTokens: 0, totalTokens: 0 })
const allMessages = reactive({})
const animationData = ref(null)
const activeAnimIdx = ref(0)
const showSettings = ref(false)
const selectedModel = ref('deepseek-chat')
const llmStatus = ref('checking')

function loadConfig() {
  try {
    const saved = localStorage.getItem('oj-agent-config')
    if (saved) {
      const cfg = JSON.parse(saved)
      if (cfg.model) selectedModel.value = cfg.model
    }
  } catch (e) {}
}
loadConfig()
const streamStates = reactive({})

function ensureStreamState(sid) {
  if (!streamStates[sid]) streamStates[sid] = { loading: false, content: '' }
  return streamStates[sid]
}

const currentMessages = computed(() => {
  const sid = activeSessionId.value
  if (!sid) return []
  if (!allMessages[sid]) allMessages[sid] = []
  return allMessages[sid]
})

function addMessage(sessionId, message) {
  if (!allMessages[sessionId]) allMessages[sessionId] = []
  allMessages[sessionId].push(message)
}

async function loadSessionMessages(sid) {
  if (!sid) return
  try {
    const session = await GetSession(sid)
    if (session && session.messages && session.messages.length > 0) {
      if (!allMessages[sid]) allMessages[sid] = []
      const existing = allMessages[sid]
      if (existing.length < session.messages.length) {
        allMessages[sid] = session.messages.map(m => ({ ...m }))
      }
    }
  } catch (e) {
    console.error('Failed to load session messages:', e)
  }
}

watch(activeSessionId, (newId) => {
  if (newId) {
    loadSessionMessages(newId)
    animationData.value = null
  }
})

provide('activeSessionId', activeSessionId)
provide('sessions', sessions)
provide('tokenUsage', tokenUsage)
provide('messages', currentMessages)
provide('animationData', animationData)
provide('activeAnimIdx', activeAnimIdx)
provide('selectedModel', selectedModel)
provide('llmStatus', llmStatus)
provide('addMessage', addMessage)
provide('streamStates', streamStates)
provide('ensureStreamState', ensureStreamState)

onMounted(async () => {
  try {
    const raw = await GetLLMStatus()
    try {
      const info = JSON.parse(raw)
      llmStatus.value = info.status || raw
      const savedCfg = localStorage.getItem('oj-agent-config')
      const savedModel = savedCfg ? (() => { try { return JSON.parse(savedCfg).model } catch { return null } })() : null
      if (info.model && !savedModel) {
        selectedModel.value = info.model
      }
    } catch {
      llmStatus.value = raw
    }
  } catch (e) {
    llmStatus.value = 'mock'
  }
})
</script>

<template>
  <div class="app-shell">
    <div class="app-layout">
      <Sidebar
        :sessions="sessions"
        :activeId="activeSessionId"
        @select="activeSessionId = $event"
        @new="activeSessionId = ''; animationData = null"
      />
      <ChatArea
        :messages="currentMessages"
        :sessionId="activeSessionId"
        @open-settings="showSettings = true"
      />
      <AnimationPanel :animationData="animationData" />
      <TokenBar :usage="tokenUsage" />
    </div>
    <SettingsModal
      v-if="showSettings"
      @close="showSettings = false"
      @updated="showSettings = false"
    />
  </div>
</template>

<style scoped>
.app-shell {
  width: 100vw; height: 100vh;
  background: var(--bg-deepest);
  position: relative;
  overflow: hidden;
}

.app-shell::before {
  content: '';
  position: absolute; inset: 0;
  background:
    radial-gradient(ellipse 80% 60% at 15% 30%, rgba(6,182,212,0.10) 0%, transparent 60%),
    radial-gradient(ellipse 60% 70% at 85% 70%, rgba(168,85,247,0.08) 0%, transparent 60%),
    radial-gradient(ellipse 50% 60% at 50% 40%, rgba(6,182,212,0.06) 0%, transparent 70%);
  pointer-events: none; z-index: 0;
}

/* SAM 装甲能量线 — 明显斜向光迹 */
.app-shell::after {
  content: '';
  position: absolute; inset: 0; z-index: 0; pointer-events: none;
  background:
    linear-gradient(135deg, transparent 0%, rgba(6,182,212,0.10) 45%, rgba(34,211,238,0.18) 50%, rgba(6,182,212,0.10) 55%, transparent 100%),
    linear-gradient(180deg, transparent 0%, rgba(6,182,212,0.08) 48%, rgba(34,211,238,0.12) 50%, rgba(6,182,212,0.08) 52%, transparent 100%),
    linear-gradient(90deg, transparent 0%, rgba(168,85,247,0.06) 95%, rgba(168,85,247,0.12) 98%, transparent 100%);
}

/* SAM HUD 扫描线 — 更亮 */
.app-layout::before {
  content: '';
  position: absolute; left: 0; right: 0; height: 2px;
  background: linear-gradient(90deg, transparent, rgba(6,182,212,0.30) 15%, rgba(34,211,238,0.55) 50%, rgba(6,182,212,0.30) 85%, transparent);
  z-index: 10; pointer-events: none;
  box-shadow: 0 0 10px rgba(6,182,212,0.25), 0 0 3px rgba(34,211,238,0.4);
  animation: samScanline 8s linear infinite;
}

.app-layout {
  display: grid;
  grid-template-columns: 260px 1fr 420px;
  grid-template-rows: 1fr 44px;
  height: 100vh; width: 100vw;
  position: relative; z-index: 1;
}

.app-layout > :nth-child(2) {
  background: var(--bg-main);
}
</style>
