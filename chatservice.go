package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"strings"
	"sync"
	"time"

	"Oj-Agent/llm"
	"Oj-Agent/storage"

	"github.com/wailsapp/wails/v3/pkg/application"
)

type Role string

const (
	RoleUser      Role = "user"
	RoleAssistant Role = "assistant"
)

type Style string

const (
	StyleNormal    Style = "normal"
	StyleHighlight Style = "highlight"
	StyleCompare   Style = "compare"
	StyleSwap      Style = "swap"
	StyleResult    Style = "result"
	StylePivot     Style = "pivot"
	StyleDim       Style = "dim"
)

type Message struct {
	Role      Role        `json:"role"`
	Content   string      `json:"content"`
	Time      time.Time   `json:"time"`
	Animation UnifiedAnim `json:"animation,omitempty"`
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
	UserMessage      Message     `json:"userMessage"`
	AssistantMessage Message     `json:"assistantMessage"`
	SessionID        string      `json:"sessionId"`
	TokenUsage       TokenUsage  `json:"tokenUsage"`
	Animation        UnifiedAnim `json:"animation"`
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

type UnifiedAnim struct {
	Elements  []Element `json:"elements"`
	Frames    []Frame   `json:"frames"`
	SVGWidth  float64   `json:"svgW"`
	SVGHeight float64   `json:"svgH"`
}

type Element struct {
	ID      string  `json:"id"`
	Kind    string  `json:"kind"`
	X       float64 `json:"x"`
	Y       float64 `json:"y"`
	W       float64 `json:"w,omitempty"`
	H       float64 `json:"h,omitempty"`
	R       float64 `json:"r,omitempty"`
	X2      float64 `json:"x2,omitempty"`
	Y2      float64 `json:"y2,omitempty"`
	Text    string  `json:"text,omitempty"`
	Style   string  `json:"style,omitempty"`
	RX      float64 `json:"rx,omitempty"`
	Visible bool    `json:"visible"`
	Points  string  `json:"points,omitempty"`
	Arrow   bool    `json:"arrow,omitempty"`
}

type Frame struct {
	Description string                 `json:"desc"`
	Delta       map[string]interface{} `json:"delta"`
}

type ChatService struct {
	mu          sync.Mutex
	sessions    map[string]*ChatSession
	totalTokens int
	llmClient   *llm.Client
	db          *storage.DB
}

func NewChatService(llmClient *llm.Client, db *storage.DB) *ChatService {
	cs := &ChatService{
		sessions:  make(map[string]*ChatSession),
		llmClient: llmClient,
		db:        db,
	}
	cs.loadFromDB()
	return cs
}

func (c *ChatService) loadFromDB() {
	if c.db == nil {
		return
	}

	rows, err := c.db.ListSessions()
	if err != nil {
		log.Printf("[ChatService] load sessions: %v", err)
		return
	}

	for _, sr := range rows {
		createdAt, _ := time.Parse(time.RFC3339Nano, sr.CreatedAt)
		updatedAt, _ := time.Parse(time.RFC3339Nano, sr.UpdatedAt)

		session := &ChatSession{
			ID:        sr.ID,
			Title:     sr.Title,
			Messages:  []Message{},
			CreatedAt: createdAt,
			UpdatedAt: updatedAt,
		}

		msgs, err := c.db.GetMessages(sr.ID)
		if err != nil {
			log.Printf("[ChatService] load messages for %s: %v", sr.ID, err)
		}
		for _, mr := range msgs {
			t, _ := time.Parse(time.RFC3339Nano, mr.Time)
			msg := Message{
				Role:    Role(mr.Role),
				Content: mr.Content,
				Time:    t,
			}
			if mr.Animation != "" {
				if a, ok := parseAnimJSON(mr.Animation); ok {
					msg.Animation = a
				}
			}
			session.Messages = append(session.Messages, msg)
		}

		c.sessions[sr.ID] = session
	}

	log.Printf("[ChatService] loaded %d sessions from db", len(rows))
}

func (c *ChatService) NewSession() *ChatSession {
	c.mu.Lock()
	defer c.mu.Unlock()
	id := fmt.Sprintf("session_%d", time.Now().UnixNano())
	now := time.Now()
	session := &ChatSession{
		ID: id, Title: "新对话", Messages: []Message{},
		CreatedAt: now, UpdatedAt: now,
	}
	c.sessions[id] = session

	if c.db != nil {
		_ = c.db.InsertSession(&storage.SessionRow{
			ID:        id,
			Title:     "新对话",
			CreatedAt: now.Format(time.RFC3339Nano),
			UpdatedAt: now.Format(time.RFC3339Nano),
		})
	}

	return session
}

func (c *ChatService) GetSessions() []SessionInfo {
	c.mu.Lock()
	defer c.mu.Unlock()
	result := make([]SessionInfo, 0, len(c.sessions))
	for _, s := range c.sessions {
		result = append(result, SessionInfo{
			ID: s.ID, Title: s.Title, CreatedAt: s.CreatedAt,
			UpdatedAt: s.UpdatedAt, MsgCount: len(s.Messages),
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
	if c.db != nil {
		_ = c.db.DeleteSession(sessionID)
	}
	return nil
}

func (c *ChatService) SendMessage(req SendMessageRequest) SendMessageResponse {
	c.mu.Lock()

	session, ok := c.sessions[req.SessionID]
	if !ok {
		now := time.Now()
		session = &ChatSession{
			ID: req.SessionID, Title: req.Content, Messages: []Message{},
			CreatedAt: now, UpdatedAt: now,
		}
		c.sessions[req.SessionID] = session
		if c.db != nil {
			_ = c.db.InsertSession(&storage.SessionRow{
				ID: req.SessionID, Title: req.Content,
				CreatedAt: now.Format(time.RFC3339Nano),
				UpdatedAt: now.Format(time.RFC3339Nano),
			})
		}
	}

	userMsg := Message{Role: RoleUser, Content: req.Content, Time: time.Now()}
	session.Messages = append(session.Messages, userMsg)
	if len(session.Messages) == 2 {
		session.Title = truncateString(req.Content, 30)
	}
	session.UpdatedAt = time.Now()

	if c.db != nil {
		c.saveSessionMeta(session)
		_ = c.db.InsertMessage(session.ID, &storage.MessageRow{
			SessionID: session.ID, Role: string(RoleUser),
			Content: req.Content, Time: userMsg.Time.Format(time.RFC3339Nano),
		})
	}

	if c.llmClient == nil || !c.llmClient.Available() {
		assistantMsg := Message{
			Role:    RoleAssistant,
			Content: "⚠️ 未配置 API Key，请点击右上角 ⚙ 设置 LLM 连接。\n\n支持 DeepSeek、OpenAI 兼容 API、Ollama 本地模型。",
			Time:    time.Now(),
		}
		session.Messages = append(session.Messages, assistantMsg)
		if c.db != nil {
			_ = c.db.InsertMessage(session.ID, &storage.MessageRow{
				SessionID: session.ID, Role: string(RoleAssistant),
				Content: assistantMsg.Content, Time: assistantMsg.Time.Format(time.RFC3339Nano),
			})
		}
		c.mu.Unlock()
		return SendMessageResponse{
			UserMessage:      userMsg,
			AssistantMessage: assistantMsg,
			SessionID:        session.ID,
		}
	}

	sessionID := session.ID
	c.mu.Unlock()

	go c.streamGenerate(sessionID, req.Content, req.Language)

	assistantMsg := Message{Role: RoleAssistant, Content: "", Time: time.Now()}
	return SendMessageResponse{
		UserMessage:      userMsg,
		AssistantMessage: assistantMsg,
		SessionID:        sessionID,
	}
}

func (c *ChatService) saveSessionMeta(s *ChatSession) {
	if c.db == nil {
		return
	}
	_ = c.db.UpdateSession(s.ID, s.Title, s.UpdatedAt.Format(time.RFC3339Nano))
}

func (c *ChatService) streamGenerate(sessionID, content, language string) {
	ctx := context.Background()
	reader, err := c.llmClient.Stream(ctx, content, language)
	if err != nil {
		c.mu.Lock()
		defer c.mu.Unlock()
		session, ok := c.sessions[sessionID]
		if !ok {
			return
		}
		errMsg := Message{
			Role:    RoleAssistant,
			Content: fmt.Sprintf("❌ LLM 调用失败: %s\n\n请检查 API Key 或网络连接，然后重试。", err.Error()),
			Time:    time.Now(),
		}
		session.Messages = append(session.Messages, errMsg)
		session.UpdatedAt = time.Now()
		if c.db != nil {
			_ = c.db.InsertMessage(sessionID, &storage.MessageRow{
				SessionID: sessionID, Role: string(RoleAssistant),
				Content: errMsg.Content, Time: errMsg.Time.Format(time.RFC3339Nano),
			})
		}
		emit("chat-error", map[string]any{
			"sessionId": sessionID,
			"content":   errMsg.Content,
		})
		return
	}

	var accumulated string
	for {
		msg, recvErr := reader.Recv()
		if recvErr == io.EOF {
			break
		}
		if recvErr != nil {
			emit("chat-error", map[string]any{
				"sessionId": sessionID,
				"content":   fmt.Sprintf("流式读取错误: %v", recvErr),
			})
			return
		}
		accumulated += msg.Content
		emit("chat-chunk", map[string]any{
			"sessionId": sessionID,
			"content":   accumulated,
		})
	}

	parsed := llm.ParseResponse(accumulated)
	assistantContent := parsed.Markdown
	if assistantContent == "" {
		assistantContent = accumulated
	}

	var anim UnifiedAnim
	if parsed.HasAnim {
		if a, ok := parseAnimJSON(parsed.AnimJSON); ok {
			anim = a
		}
	}

	c.mu.Lock()
	session, ok := c.sessions[sessionID]
	if !ok {
		c.mu.Unlock()
		return
	}

	assistantMsg := Message{Role: RoleAssistant, Content: assistantContent, Time: time.Now(), Animation: anim}
	session.Messages = append(session.Messages, assistantMsg)
	session.UpdatedAt = time.Now()

	if c.db != nil {
		c.saveSessionMeta(session)
		var animationJSON string
		if anim.Elements != nil && anim.Frames != nil {
			if b, err := json.Marshal(anim); err == nil {
				animationJSON = string(b)
			}
		}
		_ = c.db.InsertMessage(sessionID, &storage.MessageRow{
			SessionID: sessionID, Role: string(RoleAssistant),
			Content: assistantContent, Time: assistantMsg.Time.Format(time.RFC3339Nano),
			Animation: animationJSON,
		})
	}

	inputTokens := estimateTokens(content)
	outputTokens := estimateTokens(assistantContent)
	sessionTokens := inputTokens + outputTokens
	c.totalTokens += sessionTokens
	tokenUsage := TokenUsage{SessionTokens: sessionTokens, TotalTokens: c.totalTokens}
	c.mu.Unlock()

	emit("chat-complete", map[string]any{
		"sessionId":  sessionID,
		"content":    assistantContent,
		"time":       assistantMsg.Time,
		"tokenUsage": tokenUsage,
		"animation":  anim,
	})
}

func emit(eventName string, data any) {
	app := application.Get()
	if app == nil {
		return
	}
	app.Event.Emit(eventName, data)
}

func parseAnimJSON(raw string) (UnifiedAnim, bool) {
	var anim UnifiedAnim
	if err := json.Unmarshal([]byte(raw), &anim); err != nil {
		return anim, false
	}
	if len(anim.Elements) == 0 || len(anim.Frames) == 0 {
		return anim, false
	}
	return anim, true
}

func (c *ChatService) GetTokenUsage() TokenUsage {
	c.mu.Lock()
	defer c.mu.Unlock()
	return TokenUsage{TotalTokens: c.totalTokens}
}

func (c *ChatService) UpdateLLMConfig(apiKey, baseURL, model string) string {
	c.mu.Lock()
	defer c.mu.Unlock()

	if apiKey == "" {
		c.llmClient = nil
		return "API Key 已清空"
	}

	client, err := llm.NewClient(&llm.Config{
		APIKey:  apiKey,
		BaseURL: baseURL,
		Model:   model,
	})
	if err != nil {
		c.llmClient = nil
		return "连接失败: " + err.Error()
	}

	c.llmClient = client
	return "连接成功，模型: " + model
}

func (c *ChatService) GetLLMStatus() string {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.llmClient != nil && c.llmClient.Available() {
		return "connected"
	}
	return "mock"
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
