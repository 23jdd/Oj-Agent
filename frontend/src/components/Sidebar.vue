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
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2.5">
          <polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/>
        </svg>
      </div>
      <div class="brand-info">
        <span class="brand-text">OJ Agent</span>
        <span class="brand-sub">Algorithm Visualizer</span>
      </div>
    </div>

    <div class="sidebar-header">
      <button class="new-chat-btn" @click="newSession">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
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
      <div v-if="(filteredSessions()).length === 0 && !loading" class="empty-hint">
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.2" opacity="0.3">
          <path d="M21 15a2 2 0 01-2 2H7l-4 4V5a2 2 0 012-2h14a2 2 0 012 2z"/>
        </svg>
        <span>暂无对话</span>
      </div>
    </div>

    <div class="sidebar-footer">
      <div class="footer-item">
        <div class="footer-dot"></div>
        <span>暗色模式</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  display:flex; flex-direction:column; height:100%;
  background: var(--glass-bg);
  backdrop-filter: blur(var(--glass-blur));
  -webkit-backdrop-filter: blur(var(--glass-blur));
  border-right: 1px solid var(--glass-border);
  box-shadow: 1px 0 0 rgba(6,182,212,0.06), 2px 0 12px rgba(6,182,212,0.04);
  overflow: hidden;
}

.sidebar-brand {
  display:flex; align-items:center; gap:12px;
  padding: 20px 18px 16px;
  border-bottom: 1px solid var(--glass-border);
  position: relative;
}
.sidebar-brand::after {
  content: '';
  position: absolute; bottom: -1px; left: 12px; right: 12px; height: 1px;
  background: linear-gradient(90deg, transparent, rgba(6,182,212,0.30) 30%, rgba(34,211,238,0.50) 50%, rgba(6,182,212,0.30) 70%, transparent);
}
.brand-icon {
  width: 38px; height: 38px;
  background: var(--gradient-brand);
  border-radius: var(--radius-sm);
  display:flex; align-items:center; justify-content:center;
  box-shadow: 0 0 18px rgba(6,182,212,0.45), 0 0 4px rgba(34,211,238,0.3);
  position: relative;
  animation: fireflyPulse 3s ease-in-out infinite;
}
.brand-icon::after {
  content: '';
  position: absolute; inset: 0; border-radius: inherit;
  background: linear-gradient(135deg, rgba(255,255,255,0.25) 20%, transparent 50%, rgba(255,255,255,0.10) 80%);
  pointer-events: none;
}
.brand-info { display:flex; flex-direction:column; gap:1px; }
.brand-text { font-size:16px; font-weight:700; color:var(--text-primary); letter-spacing:-0.4px; }
.brand-sub { font-size:10px; color:var(--text-dim); letter-spacing:0.5px; text-transform:uppercase; }

.sidebar-header { padding: 14px 16px 8px; }
.new-chat-btn {
  width:100%; padding: 10px 16px;
  background: var(--gradient-brand);
  color:#fff; border:none; border-radius: var(--radius-sm);
  font-size:13px; font-weight:600; cursor:pointer;
  display:flex; align-items:center; justify-content:center; gap:8px;
  transition: all var(--transition-smooth);
  box-shadow: 0 2px 8px rgba(6,182,212,0.25);
}
.new-chat-btn:hover {
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(6,182,212,0.35);
}
.new-chat-btn:active { transform: scale(0.97); }

.search-box { position:relative; padding: 0 16px 12px; }
.search-icon {
  position:absolute; left:24px; top:50%; transform:translateY(-50%);
  color:var(--text-dim); pointer-events:none; z-index:1;
}
.search-input {
  width:100%; padding: 9px 12px 9px 34px;
  background: var(--glass-hover);
  border: 1px solid var(--glass-border);
  border-radius: var(--radius-sm);
  color: var(--text-primary); font-size:12px; outline:none;
  transition: all var(--transition-fast);
}
.search-input:focus {
  border-color: rgba(6,182,212,0.3);
  background: rgba(6,182,212,0.04);
  box-shadow: 0 0 0 3px rgba(6,182,212,0.08);
}
.search-input::placeholder { color: var(--text-dim); }

.session-list { flex:1; overflow-y:auto; padding: 6px 16px; }

.session-item {
  display:flex; align-items:center; gap:10px;
  padding: 10px 16px; margin-bottom: 2px;
  border-radius: var(--radius-sm); cursor:pointer;
  transition: all var(--transition-fast);
  border: 1px solid transparent;
}
.session-item:hover {
  background: var(--glass-hover);
  border-color: var(--glass-border);
}
.session-item.active {
  background: rgba(6,182,212,0.08);
  border-color: rgba(6,182,212,0.2);
  box-shadow: 0 0 20px rgba(6,182,212,0.06);
}

.session-dot {
  width:7px; height:7px; border-radius:50%;
  background: var(--text-dim); flex-shrink:0;
  transition: all var(--transition-fast);
}
.session-item.active .session-dot {
  background: var(--accent);
  box-shadow: 0 0 8px rgba(6,182,212,0.5);
}

.session-info { flex:1; min-width:0; }
.session-title {
  display:block; font-size:13px; font-weight:500; color:var(--text-secondary);
  white-space:nowrap; overflow:hidden; text-overflow:ellipsis; margin-bottom:2px;
  transition: color var(--transition-fast);
}
.session-item.active .session-title { color: var(--text-primary); font-weight:600; }
.session-meta { font-size:11px; color:var(--text-dim); }

.delete-btn {
  width:28px; height:28px; border:none; background:transparent;
  color:var(--text-dim); cursor:pointer; border-radius:var(--radius-sm);
  display:flex; align-items:center; justify-content:center;
  opacity:0; transition: all 0.2s ease; flex-shrink:0;
}
.session-item:hover .delete-btn { opacity:1; }
.delete-btn:hover { background: rgba(239,68,68,0.12); color: var(--danger); }
.delete-btn:active { transform: scale(0.9); }

.empty-hint {
  display:flex; flex-direction:column; align-items:center; gap:12px;
  text-align:center; color:var(--text-dim); font-size:12px;
  padding: 56px 16px;
}

.sidebar-footer {
  padding: 12px 18px;
  border-top: 1px solid var(--glass-border);
}
.footer-item {
  display:flex; align-items:center; gap:10px;
  color:var(--text-dim); font-size:12px; opacity:0.7;
  transition: opacity var(--transition-fast);
}
.footer-item:hover { opacity:1; }
.footer-dot { width:7px; height:7px; border-radius:50%; background: var(--text-dim); }
</style>
