<script setup>
import { ref } from 'vue'
import { GetSessions, DeleteSession } from '../../bindings/Oj-Agent/chatservice'
import { SessionInfo } from '../../bindings/Oj-Agent/models'

const props = defineProps({
  sessions: Array,
  activeId: String
})

const emit = defineEmits(['select', 'new', 'delete'])
const searchQuery = ref('')
const loading = ref(false)

const filteredSessions = () => {
  if (!searchQuery.value) return props.sessions || []
  const q = searchQuery.value.toLowerCase()
  return (props.sessions || []).filter(s =>
    (s.title || '').toLowerCase().includes(q)
  )
}

const loadSessions = async () => {
  loading.value = true
  try {
    const result = await GetSessions()
    if (result) {
      result.forEach(s => {
        const idx = (props.sessions || []).findIndex(x => x.id === s.id)
        if (idx === -1) {
          props.sessions.push(s)
        }
      })
    }
  } catch (e) {
    console.error('Failed to load sessions:', e)
  } finally {
    loading.value = false
  }
}

loadSessions()

const selectSession = (id) => {
  emit('select', id)
}

const newSession = () => {
  emit('new')
}

const deleteSession = async (id, e) => {
  e.stopPropagation()
  try {
    await DeleteSession(id)
    const idx = (props.sessions || []).findIndex(s => s.id === id)
    if (idx !== -1) props.sessions.splice(idx, 1)
    if (props.activeId === id) {
      emit('new')
    }
  } catch (err) {
    console.error('Failed to delete session:', err)
  }
}

const formatTime = (t) => {
  if (!t) return ''
  const d = new Date(t)
  const now = new Date()
  if (d.toDateString() === now.toDateString()) {
    return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  }
  return d.toLocaleDateString('zh-CN', { month: 'short', day: 'numeric' })
}
</script>

<template>
  <div class="sidebar">
    <div class="sidebar-header">
      <button class="new-chat-btn" @click="newSession">
        <span class="icon">+</span>
        新建对话
      </button>
    </div>

    <div class="search-box">
      <input
        v-model="searchQuery"
        type="text"
        placeholder="搜索历史记录..."
        class="search-input"
      />
    </div>

    <div class="session-list">
      <div
        v-for="session in filteredSessions()"
        :key="session.id"
        :class="['session-item', { active: session.id === activeId }]"
        @click="selectSession(session.id)"
      >
        <div class="session-info">
          <span class="session-title">{{ session.title || '新对话' }}</span>
          <span class="session-meta">
            {{ session.msgCount || 0 }} 条消息 · {{ formatTime(session.updatedAt) }}
          </span>
        </div>
        <button class="delete-btn" @click="(e) => deleteSession(session.id, e)" title="删除">
          &times;
        </button>
      </div>

      <div v-if="(filteredSessions()).length === 0" class="empty-hint">
        暂无历史记录
      </div>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  background: #1a1f2e;
  display: flex;
  flex-direction: column;
  height: 100%;
  border-right: 1px solid #2d3748;
  overflow: hidden;
}

.sidebar-header {
  padding: 16px;
  border-bottom: 1px solid #2d3748;
}

.new-chat-btn {
  width: 100%;
  padding: 10px 16px;
  background: #3b82f6;
  color: #fff;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  transition: background 0.2s;
}

.new-chat-btn:hover {
  background: #2563eb;
}

.icon {
  font-size: 18px;
  font-weight: bold;
}

.search-box {
  padding: 12px 16px;
}

.search-input {
  width: 100%;
  padding: 8px 12px;
  background: #111827;
  border: 1px solid #374151;
  border-radius: 6px;
  color: #e5e7eb;
  font-size: 13px;
  outline: none;
  transition: border-color 0.2s;
}

.search-input:focus {
  border-color: #3b82f6;
}

.search-input::placeholder {
  color: #6b7280;
}

.session-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 8px;
}

.session-item {
  display: flex;
  align-items: center;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s;
  margin-bottom: 2px;
}

.session-item:hover {
  background: #1f2937;
}

.session-item.active {
  background: #1e3a5f;
}

.session-info {
  flex: 1;
  min-width: 0;
}

.session-title {
  display: block;
  font-size: 14px;
  color: #e5e7eb;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  margin-bottom: 2px;
}

.session-meta {
  font-size: 12px;
  color: #6b7280;
}

.delete-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: #6b7280;
  font-size: 18px;
  cursor: pointer;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: all 0.15s;
  flex-shrink: 0;
}

.session-item:hover .delete-btn {
  opacity: 1;
}

.delete-btn:hover {
  background: #ef4444;
  color: #fff;
}

.empty-hint {
  text-align: center;
  color: #6b7280;
  font-size: 13px;
  padding: 32px 16px;
}
</style>
