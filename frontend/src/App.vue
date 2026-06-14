<script setup>
import Sidebar from './components/Sidebar.vue'
import ChatArea from './components/ChatArea.vue'
import AnimationPanel from './components/AnimationPanel.vue'
import TokenBar from './components/TokenBar.vue'
import SettingsModal from './components/SettingsModal.vue'
import { ref, provide, onMounted } from 'vue'
import { GetLLMStatus } from '../bindings/Oj-Agent/chatservice'

const activeSessionId = ref('')
const sessions = ref([])
const tokenUsage = ref({ sessionTokens: 0, totalTokens: 0 })
const messages = ref([])
const animationData = ref(null)
const showSettings = ref(false)
const selectedModel = ref('deepseek-chat')
const llmStatus = ref('checking')

provide('activeSessionId', activeSessionId)
provide('sessions', sessions)
provide('tokenUsage', tokenUsage)
provide('messages', messages)
provide('animationData', animationData)
provide('selectedModel', selectedModel)
provide('llmStatus', llmStatus)

onMounted(async () => {
  try {
    llmStatus.value = await GetLLMStatus()
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
        @new="activeSessionId = ''; messages = []; animationData = null"
      />
      <ChatArea
        :messages="messages"
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
  background-image:
    radial-gradient(circle at 20% 30%, rgba(59,130,246,0.03) 0%, transparent 50%),
    radial-gradient(circle at 80% 70%, rgba(139,92,246,0.03) 0%, transparent 50%),
    radial-gradient(circle at 50% 50%, rgba(59,130,246,0.02) 0%, transparent 70%);
}

.app-layout {
  display: grid;
  grid-template-columns: 260px 1fr 420px;
  grid-template-rows: 1fr 44px;
  height: 100vh; width: 100vw;
}

.app-layout > :nth-child(1) {
  border-right: 1px solid rgba(42,48,60,0.6);
  box-shadow: 3px 0 20px rgba(0,0,0,0.2);
  z-index: 2;
}

.app-layout > :nth-child(3) {
  border-left: 1px solid rgba(42,48,60,0.6);
  box-shadow: -3px 0 20px rgba(0,0,0,0.2);
  z-index: 2;
}

.app-layout > :nth-child(4) {
  grid-column: 1 / -1;
  z-index: 3;
}
</style>
