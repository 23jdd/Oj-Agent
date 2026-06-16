<script setup>
import { ref } from 'vue'
import { GetSessions, NewSession, DeleteSession as DelSession } from '../../bindings/Oj-Agent/chatservice'

const props = defineProps({ sessions: Array, activeId: String })
const emit = defineEmits(['select', 'new'])
const searchQuery = ref('')
const loading = ref(false)

const filteredSessions = () => {
  if (!searchQuery.value) return props.sessions || []
  const q = searchQuery.value.toLowerCase()
  return (props.sessions || []).filter(s => (s.title || '').toLowerCase().includes(q))
}

const loadSessions = async () => {
  loading.value = true
  try {
    const result = await GetSessions()
    if (result) {
      result.forEach(s => {
        const idx = (props.sessions || []).findIndex(x => x.id === s.id)
        if (idx === -1) props.sessions.push(s)
      })
    }
  } catch (e) { console.error(e) }
  finally { loading.value = false }
}
loadSessions()

const selectSession = (id) => emit('select', id)
const newSession = () => emit('new')
const deleteSession = async (id, e) => {
  e.stopPropagation()
  try {
    await DelSession(id)
    const idx = (props.sessions || []).findIndex(s => s.id === id)
    if (idx !== -1) props.sessions.splice(idx, 1)
    if (props.activeId === id) emit('new')
  } catch (err) { console.error(err) }
}

const formatTime = (t) => {
  if (!t) return ''
  const d = new Date(t)
  const now = new Date()
  if (d.toDateString() === now.toDateString()) return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  return d.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}
</script>

<template>
  <div class="sidebar">
    <div class="sidebar-brand">
      <div class="brand-icon">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.5">
          <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
        </svg>
      </div>
      <div class="brand-info">
        <span class="brand-text">OJ Agent</span>
        <span class="brand-sub">算法可视化</span>
      </div>
    </div>

    <div class="sidebar-header">
      <button class="new-chat-btn" @click="newSession">
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
          <line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
        </svg>
        新建对话
      </button>
    </div>

    <div class="search-box">
      <svg class="search-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
      <input v-model="searchQuery" type="text" placeholder="搜索历史记录..." class="search-input" />
    </div>

    <div class="session-list">
      <div v-for="session in filteredSessions()" :key="session.id"
           :class="['session-item', { active: session.id === activeId }]"
           @click="selectSession(session.id)">
        <div class="session-dot"></div>
        <div class="session-info">
          <span class="session-title">{{ session.title || '新对话' }}</span>
          <span class="session-meta">{{ Math.floor((session.msgCount || 0) / 2) || 0 }} 轮对话 · {{ formatTime(session.updatedAt) }}</span>
        </div>
        <button class="delete-btn" @click="(e) => deleteSession(session.id, e)" title="删除会话">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/>
          </svg>
        </button>
      </div>
      <div v-if="(filteredSessions()).length === 0 && !loading" class="empty-hint">暂无对话，点击上方开始</div>
    </div>

    <div class="sidebar-footer">
      <div class="footer-item">
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3"/><path d="M12 1v2m0 18v2M4.22 4.22l1.42 1.42m12.72 12.72l1.42 1.42M1 12h2m18 0h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
        </svg>
        <span>暗色模式</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  display: flex; flex-direction: column; height: 100%;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-right: 1px solid var(--glass-border);
  overflow: hidden;
}

.sidebar-brand {
  display: flex; align-items: center; gap: 10px; padding: 18px 16px 14px;
  border-bottom: 1px solid var(--glass-border);
  background: var(--glass-hover);
}
.brand-icon {
  width: 36px; height: 36px;
  background: linear-gradient(135deg, var(--accent) 0%, var(--accent-violet) 100%);
  border-radius: 10px; display: flex; align-items: center; justify-content: center;
  box-shadow: 0 2px 10px rgba(59,130,246,0.4), 0 0 20px rgba(59,130,246,0.15);
}
.brand-info { display: flex; flex-direction: column; gap: 1px; }
.brand-text { font-size: 15px; font-weight: 700; color: var(--text-primary); letter-spacing: -0.3px; }
.brand-sub { font-size: 10px; color: var(--text-muted); letter-spacing: 0.3px; }

.sidebar-header { padding: 12px 14px; }
.new-chat-btn {
  width: 100%; padding: 10px 16px;
  background: linear-gradient(135deg, var(--accent) 0%, #2563eb 100%);
  color: #fff; border: none; border-radius: var(--radius-md);
  font-size: 13px; font-weight: 600; cursor: pointer;
  display: flex; align-items: center; justify-content: center; gap: 8px;
  transition: all 0.25s cubic-bezier(0.4,0,0.2,1);
  box-shadow: 0 2px 6px rgba(0,0,0,0.2), 0 0 0 0 rgba(59,130,246,0.4);
}
.new-chat-btn:hover { transform: translateY(-1px); box-shadow: 0 4px 14px rgba(59,130,246,0.35), 0 2px 6px rgba(0,0,0,0.2); }
.new-chat-btn:active { transform: translateY(0) scale(0.98); }

.search-box { position: relative; padding: 0 14px 10px; }
.search-icon { position: absolute; left: 24px; top: 50%; transform: translateY(-50%); color: var(--text-muted); pointer-events: none; }
.search-input {
  width: 100%; padding: 8px 12px 8px 32px;
  background: rgba(255,255,255,0.03);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-md);
  color: var(--text-primary); font-size: 12px; outline: none; font-family: inherit;
  transition: all var(--transition-fast);
}
.search-input:focus {
  border-color: var(--border-focus);
  background: rgba(59,130,246,0.06);
  box-shadow: 0 0 0 3px rgba(59,130,246,0.1);
}
.search-input::placeholder { color: var(--text-muted); }

.session-list { flex: 1; overflow-y: auto; padding: 4px 10px; }

.session-item {
  display: flex; align-items: center; gap: 10px; padding: 10px 12px;
  border-radius: var(--radius-md); cursor: pointer;
  transition: all var(--transition-fast); margin-bottom: 2px; position: relative;
  background: transparent;
  border: 1px solid transparent;
}
.session-item:hover {
  background: var(--glass-hover);
  border-color: var(--glass-border);
}
.session-item.active {
  background: rgba(59,130,246,0.1);
  border: 1px solid rgba(59,130,246,0.25);
  box-shadow: 0 0 16px rgba(59,130,246,0.08), inset 0 1px 0 rgba(255,255,255,0.03);
}

.session-dot {
  width: 7px; height: 7px; border-radius: 50%; background: var(--text-muted); flex-shrink: 0;
  transition: all var(--transition-fast);
}
.session-item.active .session-dot { background: var(--accent); box-shadow: 0 0 6px rgba(59,130,246,0.6); }

.session-info { flex: 1; min-width: 0; }
.session-title {
  display: block; font-size: 13px; font-weight: 500; color: var(--text-primary);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; margin-bottom: 2px;
}
.session-item.active .session-title { color: #fff; font-weight: 600; }
.session-meta { font-size: 11px; color: var(--text-muted); }

.delete-btn {
  width: 28px; height: 28px; border: none; background: transparent;
  color: var(--text-muted); cursor: pointer; border-radius: var(--radius-sm);
  display: flex; align-items: center; justify-content: center;
  opacity: 0; transition: all 0.2s ease; flex-shrink: 0;
}
.session-item:hover .delete-btn { opacity: 1; }
.delete-btn:hover { background: rgba(239,68,68,0.15); color: #f87171; }

.empty-hint { text-align: center; color: var(--text-muted); font-size: 12px; padding: 48px 16px; line-height: 1.5; }

.sidebar-footer {
  padding: 12px 16px;
  border-top: 1px solid var(--glass-border);
  background: var(--glass-hover);
}
.footer-item { display: flex; align-items: center; gap: 8px; color: var(--text-muted); font-size: 12px; opacity: 0.6; transition: opacity var(--transition-fast); }
.footer-item:hover { opacity: 1; }
</style>
