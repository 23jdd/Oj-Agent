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
}

.app-shell::before {
  content: '';
  position: absolute; inset: 0;
  background:
    radial-gradient(ellipse 80% 60% at 15% 30%, rgba(59,130,246,0.04) 0%, transparent 60%),
    radial-gradient(ellipse 60% 70% at 85% 70%, rgba(139,92,246,0.04) 0%, transparent 60%),
    radial-gradient(ellipse 50% 50% at 50% 50%, rgba(59,130,246,0.02) 0%, transparent 70%);
  pointer-events: none; z-index: 0;
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
