<script setup>
import Sidebar from './components/Sidebar.vue'
import ChatArea from './components/ChatArea.vue'
import AnimationPanel from './components/AnimationPanel.vue'
import TokenBar from './components/TokenBar.vue'
import { ref, provide } from 'vue'

const activeSessionId = ref('')
const sessions = ref([])
const tokenUsage = ref({ sessionTokens: 0, totalTokens: 0 })
const messages = ref([])
const isAnimating = ref(false)
const animationSteps = ref([])

provide('activeSessionId', activeSessionId)
provide('sessions', sessions)
provide('tokenUsage', tokenUsage)
provide('messages', messages)
provide('isAnimating', isAnimating)
provide('animationSteps', animationSteps)
</script>

<template>
  <div class="app-layout">
    <Sidebar
      :sessions="sessions"
      :activeId="activeSessionId"
      @select="activeSessionId = $event"
      @new="activeSessionId = ''; messages = []"
      @delete="onDeleteSession"
    />
    <ChatArea
      :messages="messages"
      :sessionId="activeSessionId"
      @newMessage="onNewMessage"
    />
    <AnimationPanel
      :steps="animationSteps"
      :isAnimating="isAnimating"
      @play="isAnimating = true"
      @pause="isAnimating = false"
      @reset="isAnimating = false"
    />
    <TokenBar :usage="tokenUsage" />
  </div>
</template>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #111827;
  color: #e5e7eb;
  overflow: hidden;
}

::-webkit-scrollbar {
  width: 6px;
}

::-webkit-scrollbar-track {
  background: #1f2937;
}

::-webkit-scrollbar-thumb {
  background: #4b5563;
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: #6b7280;
}
</style>

<style scoped>
.app-layout {
  display: grid;
  grid-template-columns: 260px 1fr 380px;
  grid-template-rows: 1fr 40px;
  height: 100vh;
  width: 100vw;
}

.app-layout > :nth-child(4) {
  grid-column: 1 / -1;
}
</style>
