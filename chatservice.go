package main

import (
	"fmt"
	"math/rand"
	"strings"
	"sync"
	"time"
)

type Role string

const (
	RoleUser      Role = "user"
	RoleAssistant Role = "assistant"
)

type Message struct {
	Role    Role      `json:"role"`
	Content string    `json:"content"`
	Time    time.Time `json:"time"`
}

type ChatSession struct {
	ID        string    `json:"id"`
	Title     string    `json:"title"`
	Messages  []Message `json:"messages"`
	CreatedAt time.Time `json:"createdAt"`
	UpdatedAt time.Time `json:"updatedAt"`
}

type SendMessageRequest struct {
	SessionID string `json:"sessionId"`
	Content   string `json:"content"`
	Model     string `json:"model"`
	Language  string `json:"language"`
}

type SendMessageResponse struct {
	UserMessage      Message    `json:"userMessage"`
	AssistantMessage Message    `json:"assistantMessage"`
	SessionID        string     `json:"sessionId"`
	TokenUsage       TokenUsage `json:"tokenUsage"`
}

type TokenUsage struct {
	SessionTokens int `json:"sessionTokens"`
	TotalTokens   int `json:"totalTokens"`
}

type SessionInfo struct {
	ID        string    `json:"id"`
	Title     string    `json:"title"`
	CreatedAt time.Time `json:"createdAt"`
	UpdatedAt time.Time `json:"updatedAt"`
	MsgCount  int       `json:"msgCount"`
}

type ChatService struct {
	mu          sync.Mutex
	sessions    map[string]*ChatSession
	totalTokens int
}

func NewChatService() *ChatService {
	return &ChatService{
		sessions: make(map[string]*ChatSession),
	}
}

func (c *ChatService) NewSession() *ChatSession {
	c.mu.Lock()
	defer c.mu.Unlock()

	id := fmt.Sprintf("session_%d", time.Now().UnixNano())
	session := &ChatSession{
		ID:        id,
		Title:     "新对话",
		Messages:  []Message{},
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}
	c.sessions[id] = session
	return session
}

func (c *ChatService) GetSessions() []SessionInfo {
	c.mu.Lock()
	defer c.mu.Unlock()

	result := make([]SessionInfo, 0, len(c.sessions))
	for _, s := range c.sessions {
		result = append(result, SessionInfo{
			ID:        s.ID,
			Title:     s.Title,
			CreatedAt: s.CreatedAt,
			UpdatedAt: s.UpdatedAt,
			MsgCount:  len(s.Messages),
		})
	}
	return result
}

func (c *ChatService) GetSession(sessionID string) *ChatSession {
	c.mu.Lock()
	defer c.mu.Unlock()
	return c.sessions[sessionID]
}

func (c *ChatService) DeleteSession(sessionID string) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	delete(c.sessions, sessionID)
	return nil
}

func (c *ChatService) SendMessage(req SendMessageRequest) SendMessageResponse {
	c.mu.Lock()
	defer c.mu.Unlock()

	session, ok := c.sessions[req.SessionID]
	if !ok {
		session = &ChatSession{
			ID:        req.SessionID,
			Title:     req.Content,
			Messages:  []Message{},
			CreatedAt: time.Now(),
			UpdatedAt: time.Now(),
		}
		c.sessions[req.SessionID] = session
	}

	userMsg := Message{
		Role:    RoleUser,
		Content: req.Content,
		Time:    time.Now(),
	}
	session.Messages = append(session.Messages, userMsg)

	if len(session.Messages) == 2 {
		session.Title = truncateString(req.Content, 30)
	}

	assistantMsg := c.generateMockResponse(req)
	session.Messages = append(session.Messages, assistantMsg)
	session.UpdatedAt = time.Now()

	inputTokens := estimateTokens(req.Content)
	outputTokens := estimateTokens(assistantMsg.Content)
	sessionTokens := inputTokens + outputTokens
	c.totalTokens += sessionTokens

	return SendMessageResponse{
		UserMessage:      userMsg,
		AssistantMessage: assistantMsg,
		SessionID:        session.ID,
		TokenUsage: TokenUsage{
			SessionTokens: sessionTokens,
			TotalTokens:   c.totalTokens,
		},
	}
}

func (c *ChatService) GetTokenUsage() TokenUsage {
	c.mu.Lock()
	defer c.mu.Unlock()
	return TokenUsage{
		SessionTokens: 0,
		TotalTokens:   c.totalTokens,
	}
}

func (c *ChatService) generateMockResponse(req SendMessageRequest) Message {
	algorithms := []string{
		"双指针",
		"哈希表",
		"动态规划",
		"贪心算法",
		"广度优先搜索",
		"深度优先搜索",
		"二分查找",
		"滑动窗口",
		"前缀和",
		"单调栈",
	}

	alg := algorithms[rand.Intn(len(algorithms))]

	code := fmt.Sprintf("```%s\nfunc solve(nums []int) int {\n    // %s 解法\n    result := 0\n    return result\n}\n```", req.Language, alg)

	response := fmt.Sprintf(`## 解题思路

这道题可以使用 **%s** 来解决。

### 核心思想
1. 分析问题的性质和约束条件
2. 选择合适的数据结构
3. 设计高效的算法流程

### 时间复杂度
- 时间复杂度：O(n)
- 空间复杂度：O(1)

### 代码实现

%s

### 动画演示
算法的执行过程如下：
- 步骤1：初始化指针/变量
- 步骤2：遍历数据，根据条件移动指针
- 步骤3：更新结果
- 步骤4：返回最终答案

> 提示：右侧动画区将展示完整的算法执行过程。`, alg, code)

	return Message{
		Role:    RoleAssistant,
		Content: response,
		Time:    time.Now(),
	}
}

func estimateTokens(text string) int {
	return len([]rune(text)) * 2 / 3
}

func truncateString(s string, maxLen int) string {
	runes := []rune(s)
	if len(runes) <= maxLen {
		return s
	}
	return string(runes[:maxLen]) + "..."
}

var _ = strings.TrimSpace
