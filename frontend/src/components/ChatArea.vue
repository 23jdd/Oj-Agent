<script setup>
import { ref, nextTick, watch, inject, onUpdated, onMounted, onUnmounted } from 'vue'
import hljs from 'highlight.js'
import 'highlight.js/styles/github-dark.css'
import { NewSession, SendMessage } from '../../bindings/Oj-Agent/chatservice'
import { SendMessageRequest } from '../../bindings/Oj-Agent/models'
import { Events } from '@wailsio/runtime'

const props = defineProps({ messages: Array, sessionId: String })
const emit = defineEmits(['openSettings'])

const inputText = ref('')
const selectedModel = inject('selectedModel', ref('deepseek-chat'))
const selectedLanguage = ref('go')
const loading = ref(false)
const chatContainer = ref(null)
const tokenUsage = inject('tokenUsage')
const sessions = inject('sessions')
const activeSessionId = inject('activeSessionId')
const animationData = inject('animationData')
const llmStatus = inject('llmStatus', null)

const streamingContent = ref('')
const currentStreamingId = ref('')

const models = ['deepseek-chat', 'deepseek-reasoner', 'gpt-4o', 'qwen-max', 'claude-3.5-sonnet']
const languages = ['go', 'python', 'java', 'cpp', 'javascript', 'rust']
const hints = [
  { label: '两数之和', text: '两数之和，用哈希表实现 O(n)' },
  { label: '反转链表', text: '反转一个单链表' },
  { label: '中序遍历', text: '二叉树的中序遍历' },
  { label: '背包问题', text: '0/1背包问题' },
  { label: '快速排序', text: '快速排序的partition过程' },
  { label: '滑动窗口', text: '长度最小的子数组，滑动窗口解法' },
  { label: '盛水容器', text: '盛最多水的容器，双指针解法' },
  { label: '最长回文', text: '最长回文子串' },
]

let unsubChunk = null
let unsubComplete = null
let unsubError = null

onMounted(() => {
  unsubChunk = Events.On('chat-chunk', (event) => {
    streamingContent.value = event.data.content || ''
    currentStreamingId.value = event.data.sessionId || ''
    scrollToBottom()
    handleHighlight()
  })
  unsubComplete = Events.On('chat-complete', (event) => {
    const data = event.data || {}
    streamingContent.value = ''
    loading.value = false
    if (!props.messages) return
    props.messages.push({ role: 'assistant', content: data.content || '', time: data.time || new Date().toISOString() })
    if (data.tokenUsage) tokenUsage.value = data.tokenUsage
    if (data.animation && data.animation.elements?.length && data.animation.frames?.length) animationData.value = data.animation
    const sid = currentStreamingId.value || data.sessionId
    currentStreamingId.value = ''
    if (sid && sessions.value) {
      const sil = sessions.value.find(s => s.id === sid)
      if (sil) { sil.updatedAt = new Date().toISOString(); sil.msgCount = (sil.msgCount || 0) + 2 }
    }
    scrollToBottom()
    handleHighlight()
  })
  unsubError = Events.On('chat-error', (event) => {
    streamingContent.value = ''
    loading.value = false
    if (!props.messages) return
    props.messages.push({ role: 'assistant', content: event.data.content || '发生未知错误。', time: new Date().toISOString() })
    currentStreamingId.value = ''
    scrollToBottom()
  })
})

onUnmounted(() => {
  if (unsubChunk) unsubChunk()
  if (unsubComplete) unsubComplete()
  if (unsubError) unsubError()
})

const scrollToBottom = () => {
  nextTick(() => { if (chatContainer.value) chatContainer.value.scrollTop = chatContainer.value.scrollHeight })
}

const handleHighlight = () => {
  nextTick(() => {
    if (!chatContainer.value) return
    chatContainer.value.querySelectorAll('pre.code-block code').forEach(b => hljs.highlightElement(b))
  })
}

onUpdated(handleHighlight)
watch(() => props.messages?.length, () => { scrollToBottom(); handleHighlight() })

const sendMessage = async () => {
  const text = inputText.value.trim()
  if (!text || loading.value) return
  loading.value = true; inputText.value = ''; streamingContent.value = ''

  try {
    let sid = props.sessionId
    if (!sid) {
      const session = await NewSession()
      sid = session.id; activeSessionId.value = sid
      sessions.value.unshift({ id: sid, title: text.substring(0, 30), createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), msgCount: 0 })
    }
    const req = new SendMessageRequest({ sessionId: sid, content: text, model: selectedModel.value, language: selectedLanguage.value })
    const response = await SendMessage(req)
    if (!props.messages) return
    props.messages.push({ role: 'user', content: response.userMessage.content, time: response.userMessage.time })
    if (response.assistantMessage.content) {
      props.messages.push({ role: 'assistant', content: response.assistantMessage.content, time: response.assistantMessage.time })
      if (response.tokenUsage) tokenUsage.value = response.tokenUsage
      if (response.animation) animationData.value = response.animation
      loading.value = false
      const sil = sessions.value.find(s => s.id === sid)
      if (sil) { sil.updatedAt = new Date().toISOString(); sil.msgCount = (sil.msgCount || 0) + 2 }
    }
    currentStreamingId.value = sid
  } catch (e) {
    console.error(e); if (!props.messages) return
    props.messages.push({ role: 'assistant', content: '发送失败，请重试。', time: new Date().toISOString() })
    loading.value = false; streamingContent.value = ''
  } finally { scrollToBottom() }
}

const handleKeydown = (e) => { if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); sendMessage() } }

const splitCodeLang = (match, lang, code) => {
  const langLabel = lang || 'code'
  return `<div class="code-wrapper"><div class="code-header"><span class="code-lang">${langLabel}</span></div><pre class="code-block"><code class="language-${lang}">${code}</code></pre></div>`
}

const renderContent = (content) => {
  let html = content
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
    .replace(/```(\w*)\n([\s\S]*?)```/g, splitCodeLang)
    .replace(/`([^`]+)`/g, '<code class="inline-code">$1</code>')
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/^### (.+)$/gm, '<h3>$1</h3>')
    .replace(/^## (.+)$/gm, '<h2>$1</h2>')
    .replace(/^# (.+)$/gm, '<h1>$1</h1>')
    .replace(/^> (.+)$/gm, '<blockquote><span class="blockquote-icon">|</span><span>$1</span></blockquote>')
    .replace(/^- (.+)$/gm, '<li>$1</li>')
    .replace(/^(\d+)\.\s(.+)$/gm, '<li><span class="li-num">$1</span>$2</li>')
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
        <span class="tool-hint">Ctrl + Enter</span>
      </div>
      <button class="settings-btn" @click="emit('openSettings')" title="模型设置">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"/><path d="M12 1v2m0 18v2M4.22 4.22l1.42 1.42m12.72 12.72l1.42 1.42M1 12h2m18 0h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
        </svg>
      </button>
    </div>

    <div class="message-list" ref="chatContainer">
      <div v-if="(props.messages || []).length === 0" class="welcome">
        <div class="welcome-bg"></div>
        <div class="welcome-icon">
          <svg width="34" height="34" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="1.6">
            <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
          </svg>
        </div>
        <h2>OJ Agent</h2>
        <p>输入算法题目，生成可视化题解动画<br/>支持数组、链表、树、DP、排序等多种题型</p>
        <div v-if="llmStatus === 'mock'" class="mock-notice" @click="emit('openSettings')">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
          </svg>
          <span>未配置 API Key，当前使用模拟数据。点击设置 →</span>
        </div>
        <div class="hints">
          <span v-for="h in hints" :key="h.label" class="hint-tag" @click="inputText = h.text; sendMessage()">{{ h.label }}</span>
        </div>
      </div>

      <div v-for="(msg, idx) in (props.messages || [])" :key="idx" :class="['message', msg.role]" :style="{ animationDelay: '0s' }">
        <div class="message-avatar">
          <span v-if="msg.role === 'user'">U</span>
          <svg v-else width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2 4-2 4-2"/><line x1="9" y1="9" x2="9.01" y2="9"/><line x1="15" y1="9" x2="15.01" y2="9"/></svg>
        </div>
        <div class="message-body">
          <div class="message-role">{{ msg.role === 'user' ? '你' : 'OJ Agent' }}</div>
          <div v-if="msg.role === 'user'" class="message-text">{{ msg.content }}</div>
          <div v-else class="message-text" v-html="renderContent(msg.content)"></div>
        </div>
      </div>

      <div v-if="loading" class="message assistant">
        <div class="message-avatar">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2 4-2 4-2"/><line x1="9" y1="9" x2="9.01" y2="9"/><line x1="15" y1="9" x2="15.01" y2="9"/></svg>
        </div>
        <div class="message-body">
          <div class="message-role">OJ Agent</div>
          <div v-if="streamingContent" class="message-text" v-html="renderContent(streamingContent)"></div>
          <div v-else class="typing-indicator"><span></span><span></span><span></span></div>
        </div>
      </div>

      <div class="message-list-end"></div>
    </div>

    <div class="input-area">
      <div class="input-box">
        <textarea v-model="inputText" @keydown="handleKeydown" placeholder="输入题目描述，Ctrl+Enter 发送..." rows="2" :disabled="loading"></textarea>
        <button class="send-btn" @click="sendMessage" :disabled="loading || !inputText.trim()">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/>
          </svg>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-area { display: flex; flex-direction: column; height: 100%; background: var(--bg-main); overflow: hidden; position: relative; }

/* ---- Glass-morphism Toolbar ---- */
.toolbar {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 18px;
  background: rgba(22, 27, 34, 0.85);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  border-bottom: 1px solid rgba(42, 48, 60, 0.6);
  gap: 12px; z-index: 10;
}
.toolbar-left { display: flex; align-items: center; gap: 8px; }
.tool-select {
  padding: 6px 10px; background: var(--bg-main); border: 1px solid var(--border-subtle);
  border-radius: var(--radius-sm); color: var(--text-primary); font-size: 12px;
  font-family: inherit; outline: none; cursor: pointer; transition: all var(--transition-fast);
}
.tool-select:hover { border-color: #484f58; }
.tool-select:focus { border-color: var(--border-focus); box-shadow: 0 0 0 3px rgba(59,130,246,0.1); }
.tool-hint { font-size: 10px; color: var(--text-muted); background: var(--bg-main); padding: 3px 8px; border-radius: 4px; border: 1px solid var(--border-subtle); letter-spacing: 0.5px; }
.settings-btn {
  width: 32px; height: 32px; border: 1px solid var(--border-subtle); border-radius: var(--radius-sm);
  background: var(--bg-main); color: var(--text-muted); cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: all var(--transition-fast);
}
.settings-btn:hover { background: var(--bg-hover); color: var(--text-primary); border-color: #484f58; }

/* ---- Message List ---- */
.message-list { flex: 1; overflow-y: auto; padding: 24px 28px 8px; }
.message-list-end { height: 1px; }

/* ---- Welcome ---- */
.welcome { display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; text-align: center; position: relative; overflow: hidden; }
.welcome-bg {
  position: absolute; inset: 0;
  background-image: radial-gradient(circle at 30% 40%, rgba(59,130,246,0.04) 0%, transparent 50%),
                    radial-gradient(circle at 70% 60%, rgba(139,92,246,0.04) 0%, transparent 50%);
  pointer-events: none;
}
.welcome-icon {
  width: 72px; height: 72px;
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-violet) 100%);
  border-radius: 18px; display: flex; align-items: center; justify-content: center;
  margin-bottom: 24px; position: relative;
  box-shadow: 0 0 40px rgba(59,130,246,0.2), 0 8px 24px rgba(0,0,0,0.3);
  animation: welcomePulse 3s ease-in-out infinite;
}
@keyframes welcomePulse {
  0%, 100% { box-shadow: 0 0 40px rgba(59,130,246,0.2), 0 8px 24px rgba(0,0,0,0.3); }
  50% { box-shadow: 0 0 60px rgba(59,130,246,0.35), 0 8px 32px rgba(0,0,0,0.4); }
}
.welcome h2 { font-size: 24px; font-weight: 700; color: var(--text-primary); margin-bottom: 8px; letter-spacing: -0.5px; position: relative; }
.welcome p { font-size: 13px; color: var(--text-muted); margin-bottom: 16px; line-height: 1.7; position: relative; }
.mock-notice {
  display: flex; align-items: center; gap: 8px; margin-bottom: 24px; position: relative;
  padding: 10px 18px; background: rgba(245,158,11,0.08); border: 1px solid rgba(245,158,11,0.25);
  border-radius: var(--radius-md); font-size: 12px; color: #f59e0b; cursor: pointer;
  transition: all var(--transition-fast);
}
.mock-notice:hover { background: rgba(245,158,11,0.15); border-color: rgba(245,158,11,0.4); }
.hints { display: flex; flex-wrap: wrap; gap: 10px; justify-content: center; max-width: 480px; position: relative; }
.hint-tag {
  padding: 9px 18px; background: rgba(28,33,41,0.8); border: 1px solid var(--border-subtle);
  border-radius: 24px; font-size: 13px; font-weight: 500; color: var(--text-secondary);
  cursor: pointer; transition: all var(--transition-smooth);
  backdrop-filter: blur(4px);
}
.hint-tag:hover {
  background: rgba(59,130,246,0.12); color: var(--accent);
  border-color: rgba(59,130,246,0.4); transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(59,130,246,0.15);
}

/* ---- Message Bubbles ---- */
.message {
  display: flex; gap: 14px; margin-bottom: 24px;
  animation: msgSlideIn 0.35s cubic-bezier(0.4, 0, 0.2, 1) both;
}
@keyframes msgSlideIn {
  from { opacity: 0; transform: translateY(12px); }
  to { opacity: 1; transform: translateY(0); }
}
.message.user { flex-direction: row-reverse; }
.message-avatar {
  width: 30px; height: 30px; border-radius: var(--radius-sm); display: flex;
  align-items: center; justify-content: center; flex-shrink: 0;
  font-size: 11px; font-weight: 700; letter-spacing: 0.5px;
}
.message.user .message-avatar { background: linear-gradient(135deg, #3b82f6, #2563eb); color: #fff; box-shadow: 0 2px 8px rgba(59,130,246,0.3); }
.message.assistant .message-avatar { background: linear-gradient(135deg, #8b5cf6, #7c3aed); color: #fff; box-shadow: 0 2px 8px rgba(139,92,246,0.3); }
.message-body { min-width: 0; max-width: 82%; }
.message.user .message-body { display: flex; flex-direction: column; align-items: flex-end; }
.message-role { font-size: 11px; font-weight: 600; color: var(--text-muted); margin-bottom: 4px; letter-spacing: 0.5px; }
.message-text { padding: 12px 16px; border-radius: var(--radius-md); font-size: 14px; line-height: 1.7; word-break: break-word; }
.message.user .message-text {
  background: linear-gradient(135deg, #1d4ed8 0%, #1e40af 100%);
  color: #e8eeff; border-bottom-right-radius: 4px;
  box-shadow: 0 2px 8px rgba(59,130,246,0.2);
}
.message.assistant .message-text {
  background: var(--bg-elevated); color: var(--text-primary);
  border: 1px solid var(--border-subtle); border-bottom-left-radius: 4px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.15);
}

/* ---- Markdown Content ---- */
.message-text :deep(h1), .message-text :deep(h2), .message-text :deep(h3) {
  margin: 16px 0 8px; color: #fff; font-weight: 600; letter-spacing: -0.2px;
}
.message-text :deep(h2) { font-size: 17px; padding-bottom: 6px; border-bottom: 1px solid var(--border-subtle); }
.message-text :deep(h3) { font-size: 15px; }
.message-text :deep(code.inline-code) {
  padding: 2px 7px; background: #2a303c; border-radius: 4px;
  font-family: var(--font-mono); font-size: 12px; color: #f59e0b; font-weight: 500;
}
.message-text :deep(.code-wrapper) { margin: 16px 0; border-radius: var(--radius-md); overflow: hidden; border: 1px solid #21262d; }
.message-text :deep(.code-header) {
  display: flex; align-items: center; justify-content: space-between;
  padding: 6px 14px; background: #161b22; border-bottom: 1px solid #21262d;
}
.message-text :deep(.code-lang) {
  font-size: 11px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px;
}
.message-text :deep(pre.code-block) {
  margin: 0; padding: 14px 16px; background: #0d1117; overflow-x: auto;
}
.message-text :deep(pre.code-block code) {
  padding: 0; background: none; font-family: var(--font-mono); font-size: 13px; line-height: 1.6; color: #c9d1d9;
}
.message-text :deep(blockquote) {
  display: flex; gap: 10px; border: none; padding: 10px 14px; margin: 10px 0;
  color: var(--text-secondary); background: rgba(59,130,246,0.05);
  border-radius: var(--radius-md); border: 1px solid rgba(59,130,246,0.1);
}
.message-text :deep(.blockquote-icon) { color: var(--accent); font-weight: 700; font-size: 16px; line-height: 1; flex-shrink: 0; }
.message-text :deep(li) { margin-left: 22px; padding: 2px 0; line-height: 1.7; }
.message-text :deep(.li-num) { color: var(--accent); font-weight: 600; margin-right: 4px; }

/* ---- Typing Indicator ---- */
.typing-indicator { display: flex; gap: 5px; padding: 14px 16px; background: var(--bg-elevated); border-radius: var(--radius-md); border-bottom-left-radius: 4px; border: 1px solid var(--border-subtle); box-shadow: 0 1px 3px rgba(0,0,0,0.15); }
.typing-indicator span { width: 7px; height: 7px; background: var(--text-muted); border-radius: 50%; animation: typing 1.4s infinite both; }
.typing-indicator span:nth-child(2) { animation-delay: 0.2s; }
.typing-indicator span:nth-child(3) { animation-delay: 0.4s; }
@keyframes typing { 0%,60%,100% { transform:translateY(0); opacity:0.4; } 30% { transform:translateY(-8px); opacity:1; } }

/* ---- Input Area ---- */
.input-area { padding: 12px 18px 14px; background: rgba(22,27,34,0.85); backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px); border-top: 1px solid rgba(42,48,60,0.6); }
.input-box { display: flex; gap: 10px; align-items: flex-end; }
.input-box textarea {
  flex: 1; padding: 11px 14px; background: var(--bg-main); border: 1px solid var(--border-subtle);
  border-radius: var(--radius-md); color: var(--text-primary); font-size: 14px; font-family: inherit;
  resize: none; outline: none; min-height: 48px; max-height: 120px;
  transition: all var(--transition-fast);
}
.input-box textarea:focus { border-color: var(--border-focus); box-shadow: 0 0 0 3px rgba(59,130,246,0.12); }
.input-box textarea::placeholder { color: var(--text-muted); }
.send-btn {
  width: 48px; height: 48px;
  background: linear-gradient(135deg, var(--accent) 0%, #2563eb 100%);
  border: none; border-radius: var(--radius-md); color: #fff; cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: all var(--transition-smooth); flex-shrink: 0;
  box-shadow: 0 2px 8px rgba(59,130,246,0.25);
}
.send-btn:hover:not(:disabled) { transform: translateY(-2px); box-shadow: 0 6px 20px rgba(59,130,246,0.4); }
.send-btn:disabled { background: var(--bg-elevated); color: var(--text-muted); box-shadow: none; cursor: not-allowed; }
.send-btn:active:not(:disabled) { transform: scale(0.95); }
</style>
