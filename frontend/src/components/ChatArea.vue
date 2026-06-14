<script setup>
import { ref, nextTick, watch, inject } from 'vue'
import { NewSession, SendMessage } from '../../bindings/Oj-Agent/chatservice'
import { SendMessageRequest } from '../../bindings/Oj-Agent/models'

const props = defineProps({
  messages: Array,
  sessionId: String
})

const emit = defineEmits(['newMessage'])

const inputText = ref('')
const selectedModel = ref('gpt-4o')
const selectedLanguage = ref('go')
const loading = ref(false)
const chatContainer = ref(null)
const tokenUsage = inject('tokenUsage')
const sessions = inject('sessions')
const activeSessionId = inject('activeSessionId')
const animationData = inject('animationData')

const models = ['gpt-4o', 'gpt-4-turbo', 'gpt-3.5-turbo', 'deepseek-v3']
const languages = ['go', 'python', 'java', 'cpp', 'javascript', 'rust']

const scrollToBottom = () => {
  nextTick(() => {
    if (chatContainer.value) {
      chatContainer.value.scrollTop = chatContainer.value.scrollHeight
    }
  })
}

watch(() => props.messages?.length, scrollToBottom)

const sendMessage = async () => {
  const text = inputText.value.trim()
  if (!text || loading.value) return

  loading.value = true
  inputText.value = ''

  try {
    let sid = props.sessionId
    if (!sid) {
      const session = await NewSession()
      sid = session.id
      activeSessionId.value = sid
      sessions.value.unshift({
        id: sid,
        title: text.substring(0, 30),
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        msgCount: 0
      })
    }

    const req = new SendMessageRequest({
      sessionId: sid,
      content: text,
      model: selectedModel.value,
      language: selectedLanguage.value
    })

    const response = await SendMessage(req)

    if (!props.messages) return

    props.messages.push({
      role: 'user',
      content: response.userMessage.content,
      time: response.userMessage.time
    })

    const assistantMsg = {
      role: 'assistant',
      content: response.assistantMessage.content,
      time: response.assistantMessage.time
    }
    props.messages.push(assistantMsg)

    if (response.tokenUsage) {
      tokenUsage.value = response.tokenUsage
    }

    const sessionInList = sessions.value.find(s => s.id === sid)
    if (sessionInList) {
      sessionInList.updatedAt = new Date().toISOString()
      sessionInList.msgCount = (sessionInList.msgCount || 0) + 2
    }

    if (response.animation) {
      animationData.value = response.animation
    }
  } catch (e) {
    console.error('Failed to send message:', e)
    if (!props.messages) return
    props.messages.push({
      role: 'assistant',
      content: '发送失败，请重试。' + (e.message || ''),
      time: new Date().toISOString()
    })
  } finally {
    loading.value = false
    scrollToBottom()
  }
}

const handleKeydown = (e) => {
  if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
    e.preventDefault()
    sendMessage()
  }
}

const renderContent = (content) => {
  let html = content
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre class="code-block"><code>$2</code></pre>')
    .replace(/`([^`]+)`/g, '<code class="inline-code">$1</code>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/^### (.+)$/gm, '<h3>$1</h3>')
    .replace(/^## (.+)$/gm, '<h2>$1</h2>')
    .replace(/^# (.+)$/gm, '<h1>$1</h1>')
    .replace(/^> (.+)$/gm, '<blockquote>$1</blockquote>')
    .replace(/^- (.+)$/gm, '<li>$1</li>')
    .replace(/^(\d+)\. (.+)$/gm, '<li>$1. $2</li>')
    .replace(/\n\n/g, '<br><br>')
  return html
}
</script>

<template>
  <div class="chat-area">
    <div class="toolbar">
      <div class="toolbar-left">
        <select v-model="selectedModel" class="tool-select">
          <option v-for="m in models" :key="m" :value="m">{{ m }}</option>
        </select>
        <select v-model="selectedLanguage" class="tool-select">
          <option v-for="l in languages" :key="l" :value="l">{{ l }}</option>
        </select>
      </div>
    </div>

    <div class="message-list" ref="chatContainer">
      <div v-if="(props.messages || []).length === 0" class="welcome">
        <div class="welcome-icon">OJ</div>
        <h2>OJ Agent</h2>
        <p>输入一道算法题目，我会为你生成详细的题解和动画演示</p>
        <div class="hints">
          <span v-for="h in ['两数之和', '最长回文子串', '二叉树的层序遍历', '背包问题']" :key="h"
                class="hint-tag" @click="inputText = h; sendMessage()">
            {{ h }}
          </span>
        </div>
      </div>

      <div v-for="(msg, idx) in (props.messages || [])" :key="idx"
           :class="['message', msg.role]">
        <div class="message-avatar">
          {{ msg.role === 'user' ? 'U' : 'AI' }}
        </div>
        <div class="message-content">
          <div v-if="msg.role === 'user'" class="message-text">{{ msg.content }}</div>
          <div v-else class="message-text" v-html="renderContent(msg.content)"></div>
        </div>
      </div>

      <div v-if="loading" class="message assistant">
        <div class="message-avatar">AI</div>
        <div class="message-content">
          <div class="typing-indicator">
            <span></span><span></span><span></span>
          </div>
        </div>
      </div>
    </div>

    <div class="input-area">
      <div class="input-box">
        <textarea
          v-model="inputText"
          @keydown="handleKeydown"
          placeholder="输入题目描述，Ctrl+Enter 发送..."
          rows="3"
          :disabled="loading"
        ></textarea>
        <button class="send-btn" @click="sendMessage" :disabled="loading || !inputText.trim()">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 2L11 13"></path>
            <path d="M22 2L15 22L11 13L2 9L22 2Z"></path>
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-area {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #111827;
  overflow: hidden;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 16px;
  background: #1a1f2e;
  border-bottom: 1px solid #2d3748;
  gap: 12px;
}

.toolbar-left {
  display: flex;
  gap: 8px;
}

.tool-select {
  padding: 6px 12px;
  background: #111827;
  border: 1px solid #374151;
  border-radius: 6px;
  color: #e5e7eb;
  font-size: 13px;
  outline: none;
  cursor: pointer;
}

.tool-select:focus {
  border-color: #3b82f6;
}

.message-list {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.welcome {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  text-align: center;
  color: #9ca3af;
}

.welcome-icon {
  width: 64px;
  height: 64px;
  background: linear-gradient(135deg, #3b82f6, #8b5cf6);
  border-radius: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  font-weight: bold;
  color: #fff;
  margin-bottom: 16px;
}

.welcome h2 {
  color: #e5e7eb;
  margin-bottom: 8px;
}

.welcome p {
  margin-bottom: 24px;
  max-width: 400px;
}

.hints {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: center;
}

.hint-tag {
  padding: 8px 16px;
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 20px;
  font-size: 13px;
  color: #9ca3af;
  cursor: pointer;
  transition: all 0.2s;
}

.hint-tag:hover {
  background: #374151;
  color: #e5e7eb;
  border-color: #3b82f6;
}

.message {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
}

.message.user {
  flex-direction: row-reverse;
}

.message-avatar {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: bold;
  flex-shrink: 0;
}

.message.user .message-avatar {
  background: #3b82f6;
  color: #fff;
}

.message.assistant .message-avatar {
  background: #8b5cf6;
  color: #fff;
}

.message-content {
  max-width: 80%;
}

.message.user .message-content {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
}

.message-text {
  padding: 10px 14px;
  border-radius: 12px;
  font-size: 14px;
  line-height: 1.6;
  word-break: break-word;
}

.message.user .message-text {
  background: #1d4ed8;
  color: #fff;
  border-bottom-right-radius: 4px;
}

.message.assistant .message-text {
  background: #1f2937;
  color: #e5e7eb;
  border-bottom-left-radius: 4px;
}

.message-text :deep(h1),
.message-text :deep(h2),
.message-text :deep(h3) {
  margin: 12px 0 8px;
  color: #f3f4f6;
}

.message-text :deep(code) {
  padding: 2px 6px;
  background: #374151;
  border-radius: 4px;
  font-family: 'Fira Code', 'Cascadia Code', 'Consolas', monospace;
  font-size: 13px;
}

.message-text :deep(pre.code-block) {
  margin: 12px 0;
  padding: 16px;
  background: #0d1117;
  border-radius: 8px;
  overflow-x: auto;
  border: 1px solid #30363d;
}

.message-text :deep(pre.code-block code) {
  padding: 0;
  background: none;
  font-size: 13px;
  line-height: 1.5;
  color: #c9d1d9;
}

.message-text :deep(blockquote) {
  border-left: 3px solid #3b82f6;
  padding: 4px 12px;
  margin: 8px 0;
  color: #9ca3af;
  background: #1a1f2e;
  border-radius: 0 4px 4px 0;
}

.message-text :deep(li) {
  margin-left: 20px;
  padding: 2px 0;
}

.typing-indicator {
  display: flex;
  gap: 4px;
  padding: 10px 14px;
  background: #1f2937;
  border-radius: 12px;
  border-bottom-left-radius: 4px;
}

.typing-indicator span {
  width: 8px;
  height: 8px;
  background: #6b7280;
  border-radius: 50%;
  animation: typing 1.4s infinite both;
}

.typing-indicator span:nth-child(2) { animation-delay: 0.2s; }
.typing-indicator span:nth-child(3) { animation-delay: 0.4s; }

@keyframes typing {
  0%, 60%, 100% { transform: translateY(0); opacity: 0.4; }
  30% { transform: translateY(-6px); opacity: 1; }
}

.input-area {
  padding: 12px 16px;
  background: #1a1f2e;
  border-top: 1px solid #2d3748;
}

.input-box {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}

.input-box textarea {
  flex: 1;
  padding: 10px 14px;
  background: #111827;
  border: 1px solid #374151;
  border-radius: 8px;
  color: #e5e7eb;
  font-size: 14px;
  font-family: inherit;
  resize: none;
  outline: none;
  min-height: 44px;
  max-height: 120px;
}

.input-box textarea:focus {
  border-color: #3b82f6;
}

.input-box textarea::placeholder {
  color: #6b7280;
}

.send-btn {
  width: 44px;
  height: 44px;
  background: #3b82f6;
  border: none;
  border-radius: 8px;
  color: #fff;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s;
  flex-shrink: 0;
}

.send-btn:hover:not(:disabled) {
  background: #2563eb;
}

.send-btn:disabled {
  background: #374151;
  color: #6b7280;
  cursor: not-allowed;
}
</style>
