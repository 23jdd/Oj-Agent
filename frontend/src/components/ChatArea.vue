<script setup>
import { ref, computed, nextTick, watch, inject, onUpdated, onMounted, onUnmounted } from 'vue'
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
const chatContainer = ref(null)
const tokenUsage = inject('tokenUsage')
const sessions = inject('sessions')
const activeSessionId = inject('activeSessionId')
const animationData = inject('animationData')
const activeAnimIdx = inject('activeAnimIdx')
const llmStatus = inject('llmStatus', null)
const addMessage = inject('addMessage')
const streamStates = inject('streamStates')
const ensureStreamState = inject('ensureStreamState')

const sessionLoading = computed(() => {
  if (!props.sessionId) return false
  const st = streamStates[props.sessionId]
  return st ? st.loading : false
})
const sessionStreaming = computed(() => {
  if (!props.sessionId) return ''
  const st = streamStates[props.sessionId]
  return st ? st.content : ''
})

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
    const sid = event.data.sessionId || ''
    if (sid) ensureStreamState(sid).content = event.data.content || ''
    if (sid === props.sessionId) {
      scrollToBottom()
      handleHighlight()
    }
  })
  unsubComplete = Events.On('chat-complete', (event) => {
    const data = event.data || {}
    const sid = data.sessionId || ''
    const isCurrent = sid === props.sessionId
    const anims = data.animations || []
    const hasAnim = anims.length > 0
    if (sid) {
      const st = streamStates[sid]
      if (st) {
        st.content = ''
        st.loading = false
      }
      if (addMessage) {
        addMessage(sid, { role: 'assistant', content: data.content || '', time: data.time || new Date().toISOString(), animations: hasAnim ? anims : undefined })
      }
    }
    if (data.tokenUsage) tokenUsage.value = data.tokenUsage
    if (hasAnim && isCurrent) { animationData.value = anims; activeAnimIdx.value = 0 }
    if (sid && sessions.value) {
      const sil = sessions.value.find(s => s.id === sid)
      if (sil) { sil.updatedAt = new Date().toISOString(); sil.msgCount = (sil.msgCount || 0) + 2 }
    }
    if (isCurrent) {
      scrollToBottom()
      handleHighlight()
    }
  })
  unsubError = Events.On('chat-error', (event) => {
    const sid = event.data.sessionId || ''
    const isCurrent = sid === props.sessionId
    if (sid) {
      const st = streamStates[sid]
      if (st) {
        st.content = ''
        st.loading = false
      }
      if (addMessage) {
        addMessage(sid, { role: 'assistant', content: event.data.content || '发生未知错误。', time: new Date().toISOString() })
      }
    }
    if (isCurrent) scrollToBottom()
  })
})

onUnmounted(() => {
  if (unsubChunk) unsubChunk()
  if (unsubComplete) unsubComplete()
  if (unsubError) unsubError()
})

const isNearBottom = () => {
  if (!chatContainer.value) return true
  const el = chatContainer.value
  return el.scrollHeight - el.scrollTop - el.clientHeight < 120
}

const scrollToBottom = () => {
  nextTick(() => {
    if (!chatContainer.value) return
    if (isNearBottom()) {
      chatContainer.value.scrollTop = chatContainer.value.scrollHeight
    }
  })
}

const forceScrollToBottom = () => {
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
  if (!text || sessionLoading.value) return
  const sid = props.sessionId || ''
  if (sid) ensureStreamState(sid)
  if (sid) streamStates[sid].loading = true
  if (sid) streamStates[sid].content = ''
  inputText.value = ''

  try {
    let currentSid = props.sessionId
    if (!currentSid) {
      const session = await NewSession()
      currentSid = session.id; activeSessionId.value = currentSid
      sessions.value.unshift({ id: currentSid, title: text.substring(0, 30), createdAt: new Date().toISOString(), updatedAt: new Date().toISOString(), msgCount: 0 })
      ensureStreamState(currentSid)
      streamStates[currentSid].loading = true
      streamStates[currentSid].content = ''
    }
    const req = new SendMessageRequest({ sessionId: currentSid, content: text, model: selectedModel.value, language: selectedLanguage.value })
    const response = await SendMessage(req)
    if (!addMessage) return
    addMessage(currentSid, { role: 'user', content: response.userMessage.content, time: response.userMessage.time })
    if (response.assistantMessage.content) {
      const anims = response.animations || []
      const hasAnim = anims.length > 0
      addMessage(currentSid, { role: 'assistant', content: response.assistantMessage.content, time: response.assistantMessage.time, animations: hasAnim ? anims : undefined })
      if (response.tokenUsage) tokenUsage.value = response.tokenUsage
      if (hasAnim) { animationData.value = anims; activeAnimIdx.value = 0 }
      if (streamStates[currentSid]) streamStates[currentSid].loading = false
      const sil = sessions.value.find(s => s.id === currentSid)
      if (sil) { sil.updatedAt = new Date().toISOString(); sil.msgCount = (sil.msgCount || 0) + 2 }
    }
  } catch (e) {
    console.error(e); if (!addMessage) return
    const failSid = props.sessionId || ''
    addMessage(failSid, { role: 'assistant', content: '发送失败，请重试。', time: new Date().toISOString() })
    if (streamStates[failSid]) {
      streamStates[failSid].loading = false
      streamStates[failSid].content = ''
    }
  } finally { forceScrollToBottom() }
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
        <span class="model-badge">{{ selectedModel }}</span>
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
        <div v-if="msg.animations && msg.animations.length > 0" class="anim-badges">
          <div v-for="(anim, ai) in msg.animations" :key="ai" class="anim-badge" @click="animationData = msg.animations; activeAnimIdx = ai" :title="anim.label || ('动画' + (ai+1))">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
            <span class="anim-badge-label">{{ anim.label || ('动画' + (ai+1)) }}</span>
          </div>
        </div>
      </div>

      <div v-if="sessionLoading" class="message assistant">
        <div class="message-avatar">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2 4-2 4-2"/><line x1="9" y1="9" x2="9.01" y2="9"/><line x1="15" y1="9" x2="15.01" y2="9"/></svg>
        </div>
        <div class="message-body">
          <div class="message-role">OJ Agent</div>
          <div v-if="sessionStreaming" class="message-text" v-html="renderContent(sessionStreaming)"></div>
          <div v-else class="typing-indicator"><span></span><span></span><span></span></div>
        </div>
      </div>

      <div class="message-list-end"></div>
    </div>

    <div class="input-area">
      <div class="input-box">
        <textarea v-model="inputText" @keydown="handleKeydown" placeholder="输入题目描述，Ctrl+Enter 发送..." rows="2" :disabled="sessionLoading"></textarea>
        <button class="send-btn" @click="sendMessage" :disabled="sessionLoading || !inputText.trim()">
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

/* ---- Toolbar ---- */
.toolbar {
  display: flex; align-items: center; justify-content: space-between;
  padding: 10px 20px;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-bottom: 1px solid var(--glass-border);
  gap: 12px; z-index: 10;
}
.toolbar-left { display: flex; align-items: center; gap: 10px; }
.model-badge {
  padding: 5px 12px;
  background: rgba(6,182,212,0.08);
  border: 1px solid rgba(6,182,212,0.15);
  border-radius: 20px; color: var(--accent-light);
  font-size: 11px; font-weight: 600; font-family: var(--font-mono);
  white-space: nowrap; letter-spacing: 0.3px;
}
.tool-select {
  padding: 6px 12px;
  background: var(--glass-hover);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm); color: var(--text-secondary); font-size: 12px;
  outline: none; cursor: pointer; transition: all var(--transition-fast);
}
.tool-select:hover { border-color: rgba(255,255,255,0.12); background: var(--glass-active); }
.tool-select:focus { border-color: var(--border-focus); box-shadow: 0 0 0 3px rgba(6,182,212,0.1); }
.tool-hint {
  font-size: 10px; color: var(--text-dim);
  background: var(--glass-hover); padding: 4px 10px;
  border-radius: 6px; border: 1px solid var(--glass-border); letter-spacing: 0.5px;
}
.settings-btn {
  width: 34px; height: 34px;
  border: 1px solid var(--glass-border); border-radius: var(--radius-sm);
  background: var(--glass-hover); color: var(--text-dim); cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: all var(--transition-fast);
}
.settings-btn:hover {
  background: var(--glass-active); color: var(--text-secondary);
  border-color: rgba(255,255,255,0.12);
}

/* ---- Message List ---- */
.message-list { flex: 1; overflow-y: auto; padding: 28px 32px 8px; }
.message-list-end { height: 1px; }

/* ---- Welcome ---- */
.welcome {
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  height: 100%; text-align: center; position: relative;
}
.welcome-icon {
  width: 72px; height: 72px;
  background: var(--gradient-brand);
  border-radius: 18px; display: flex; align-items: center; justify-content: center;
  margin-bottom: 24px;
  box-shadow: 0 0 48px rgba(6,182,212,0.25), 0 8px 24px rgba(0,0,0,0.4);
}
.welcome h2 {
  font-size: 26px; font-weight: 700; color: var(--text-primary);
  margin-bottom: 8px; letter-spacing: -0.5px;
  background: var(--gradient-brand);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}
.welcome p { font-size: 13px; color: var(--text-dim); margin-bottom: 20px; line-height: 1.7; }
.mock-notice {
  display: flex; align-items: center; gap: 8px; margin-bottom: 24px;
  padding: 10px 18px;
  background: rgba(245,158,11,0.06);
  backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
  border: 1px solid rgba(245,158,11,0.15);
  border-radius: var(--radius-md); font-size: 12px; color: #f59e0b; cursor: pointer;
  transition: all var(--transition-fast);
}
.mock-notice:hover { background: rgba(245,158,11,0.1); border-color: rgba(245,158,11,0.28); }
.hints { display: flex; flex-wrap: wrap; gap: 10px; justify-content: center; max-width: 480px; }
.hint-tag {
  padding: 9px 18px;
  background: var(--glass-bg);
  backdrop-filter: blur(10px); -webkit-backdrop-filter: blur(10px);
  border: 1px solid var(--glass-border);
  border-radius: 24px; font-size: 13px; font-weight: 500; color: var(--text-secondary);
  cursor: pointer; transition: all var(--transition-smooth);
}
.hint-tag:hover {
  background: rgba(6,182,212,0.1); color: var(--accent-light);
  border-color: rgba(6,182,212,0.3); transform: translateY(-2px);
  box-shadow: 0 4px 16px rgba(6,182,212,0.12);
}

/* ---- Messages ---- */
.message {
  display: flex; gap: 14px; margin-bottom: 28px;
  animation: fadeIn 0.4s cubic-bezier(0.4,0,0.2,1) both;
}
.message.user { flex-direction: row-reverse; }
.message-avatar {
  width: 32px; height: 32px; border-radius: var(--radius-sm); display: flex;
  align-items: center; justify-content: center; flex-shrink: 0;
  font-size: 11px; font-weight: 700;
}
.message.user .message-avatar {
  background: linear-gradient(135deg, #06b6d4, #0891b2); color:#fff;
  box-shadow: 0 2px 12px rgba(6,182,212,0.3);
}
.message.assistant .message-avatar {
  background: linear-gradient(135deg, #a855f7, #9333ea); color:#fff;
  box-shadow: 0 2px 12px rgba(168,85,247,0.3);
}
.message-body { min-width: 0; max-width: 82%; }
.message.user .message-body { display: flex; flex-direction: column; align-items: flex-end; }
.message-role {
  font-size: 11px; font-weight: 600; color: var(--text-dim);
  margin-bottom: 6px; letter-spacing: 0.3px;
}
.message-text {
  padding: 14px 18px; border-radius: var(--radius-md);
  font-size: 14px; line-height: 1.7; word-break: break-word;
}
.message.user .message-text {
  background: linear-gradient(135deg, #1d4ed8 0%, #1e40af 100%);
  color: #e4edff; border-bottom-right-radius: var(--radius-xs);
  box-shadow: 0 4px 16px rgba(6,182,212,0.2);
}
.message.assistant .message-text {
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  color: var(--text-primary);
  border: 1px solid var(--glass-border);
  border-bottom-left-radius: var(--radius-xs);
  box-shadow: var(--shadow-md);
}

/* ---- Animation Badges ---- */
.anim-badges {
  align-self: center; flex-shrink: 0;
  display: flex; flex-direction: column; gap: 4px;
}
.anim-badge {
  display: flex; align-items: center; gap: 4px;
  padding: 5px 10px; border-radius: var(--radius-xl);
  background: rgba(6,182,212,0.08);
  border: 1px solid rgba(6,182,212,0.15);
  color: var(--accent-light);
  cursor: pointer; opacity: 0.6; transition: all 0.25s ease; white-space: nowrap;
}
.anim-badge:hover {
  opacity: 1;
  background: rgba(6,182,212,0.16);
  border-color: rgba(6,182,212,0.3);
  transform: scale(1.04);
  box-shadow: 0 0 16px rgba(6,182,212,0.15);
}
.anim-badge-label { font-size: 10px; font-weight: 500; }

/* ---- Markdown Content ---- */
.message-text :deep(h1), .message-text :deep(h2), .message-text :deep(h3) {
  margin: 18px 0 10px; color: var(--text-primary); font-weight: 600; letter-spacing: -0.2px;
}
.message-text :deep(h1) { font-size: 20px; }
.message-text :deep(h2) { font-size: 17px; padding-bottom: 8px; border-bottom: 1px solid var(--border-subtle); }
.message-text :deep(h3) { font-size: 15px; }
.message-text :deep(code.inline-code) {
  padding: 2px 7px; background: rgba(255,255,255,0.08); border-radius: var(--radius-xs);
  font-family: var(--font-mono); font-size: 12px; color: #f59e0b; font-weight: 500;
}
.message-text :deep(.code-wrapper) {
  margin: 18px 0; border-radius: var(--radius-md); overflow: hidden;
  border: 1px solid var(--glass-border);
}
.message-text :deep(.code-header) {
  display: flex; align-items: center; justify-content: space-between;
  padding: 8px 16px; background: rgba(255,255,255,0.02);
  border-bottom: 1px solid var(--glass-border);
}
.message-text :deep(.code-lang) {
  font-size: 11px; font-weight: 600; color: var(--text-dim);
  text-transform: uppercase; letter-spacing: 0.5px;
}
.message-text :deep(pre.code-block) {
  margin: 0; padding: 16px 18px; background: rgba(0,0,0,0.3); overflow-x: auto;
}
.message-text :deep(pre.code-block code) {
  padding: 0; background: none; font-family: var(--font-mono); font-size: 13px; line-height: 1.65; color: #c9d1d9;
}
.message-text :deep(blockquote) {
  display: flex; gap: 10px; border: none; padding: 12px 16px; margin: 12px 0;
  color: var(--text-secondary);
  background: rgba(6,182,212,0.04);
  border-radius: var(--radius-md);
  border: 1px solid rgba(6,182,212,0.1);
}
.message-text :deep(.blockquote-icon) { color: var(--accent); font-weight: 700; font-size: 16px; line-height: 1; flex-shrink: 0; }
.message-text :deep(li) { margin-left: 24px; padding: 3px 0; line-height: 1.7; }
.message-text :deep(.li-num) { color: var(--accent); font-weight: 600; margin-right: 6px; }

/* ---- Typing Indicator ---- */
.typing-indicator {
  display: flex; gap: 5px; padding: 16px 18px;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-radius: var(--radius-md); border-bottom-left-radius: var(--radius-xs);
  border: 1px solid var(--glass-border);
  box-shadow: var(--shadow-md);
}
.typing-indicator span {
  width: 7px; height: 7px; background: var(--text-dim); border-radius: 50%;
  animation: typing 1.4s infinite both;
}
.typing-indicator span:nth-child(2) { animation-delay: 0.2s; }
.typing-indicator span:nth-child(3) { animation-delay: 0.4s; }
@keyframes typing { 0%,60%,100% { transform:translateY(0); opacity:0.3; } 30% { transform:translateY(-8px); opacity:1; } }

/* ---- Input Area ---- */
.input-area {
  padding: 12px 20px 16px;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-top: 1px solid var(--glass-border);
}
.input-box { display: flex; gap: 10px; align-items: flex-end; }
.input-box textarea {
  flex: 1; padding: 12px 16px;
  background: var(--glass-hover);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md); color: var(--text-primary); font-size: 14px;
  resize: none; outline: none; min-height: 50px; max-height: 130px;
  transition: all var(--transition-fast);
}
.input-box textarea:focus {
  border-color: rgba(6,182,212,0.35);
  background: rgba(6,182,212,0.03);
  box-shadow: 0 0 0 3px rgba(6,182,212,0.08), 0 0 24px rgba(6,182,212,0.05);
}
.input-box textarea::placeholder { color: var(--text-dim); }
.send-btn {
  width: 50px; height: 50px;
  background: var(--gradient-brand);
  border: none; border-radius: var(--radius-md); color: #fff; cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: all var(--transition-smooth); flex-shrink: 0;
  box-shadow: 0 4px 16px rgba(6,182,212,0.25);
}
.send-btn:hover:not(:disabled) {
  transform: translateY(-2px);
  box-shadow: 0 8px 28px rgba(6,182,212,0.4);
}
.send-btn:disabled {
  background: var(--bg-elevated); color: var(--text-dim);
  box-shadow: none; cursor: not-allowed;
}
.send-btn:active:not(:disabled) { transform: scale(0.95); }
</style>
