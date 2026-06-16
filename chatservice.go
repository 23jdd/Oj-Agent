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
	"Oj-Agent/memory"
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
	Role       Role          `json:"role"`
	Content    string        `json:"content"`
	Time       time.Time     `json:"time"`
	Animations []UnifiedAnim `json:"animations,omitempty"`
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
	UserMessage      Message       `json:"userMessage"`
	AssistantMessage Message       `json:"assistantMessage"`
	SessionID        string        `json:"sessionId"`
	TokenUsage       TokenUsage    `json:"tokenUsage"`
	Animations       []UnifiedAnim `json:"animations"`
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
	Label     string    `json:"label,omitempty"`
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
				anims := parseAnimsJSON(mr.Animation)
				msg.Animations = anims
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

	historyMsgs := make([]Message, len(session.Messages)-1)
	copy(historyMsgs, session.Messages)
	sessionID := session.ID
	c.mu.Unlock()

	go c.streamGenerate(sessionID, req.Content, req.Language, historyMsgs)

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

func (c *ChatService) streamGenerate(sessionID, content, language string, historyMsgs []Message) {
	ctx := context.Background()

	entries := make([]memory.HistoryEntry, 0, len(historyMsgs))
	for _, m := range historyMsgs {
		entries = append(entries, memory.HistoryEntry{Role: string(m.Role), Content: m.Content})
	}
	history := memory.BuildHistory(entries, 0)

	reader, err := c.llmClient.Stream(ctx, content, language, history)
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

	var anims []UnifiedAnim
	for i, aj := range parsed.AnimJSONs {
		if a, ok := parseAnimJSON(aj); ok {
			if i < len(parsed.AnimLabel) {
				a.Label = parsed.AnimLabel[i]
			}
			anims = append(anims, a)
		}
	}

	needRetry := len(anims) == 0
	if !needRetry {
		if violations := checkAnimsBounds(anims); len(violations) > 0 {
			log.Printf("[streamGenerate] bounds violations in first response: %v", violations)
			needRetry = true
			anims = nil
		}
	}

	if needRetry && assistantContent != "" && c.llmClient != nil && c.llmClient.Available() {
		log.Printf("[streamGenerate] retrying (max %d rounds)...", llm.MaxRetryRounds)
		retryContent, retryBlocks, retryErr := c.llmClient.GenerateWithRetry(ctx, content, language, history)
		if retryErr != nil {
			log.Printf("[streamGenerate] retry failed: %v", retryErr)
		}
		for _, block := range retryBlocks {
			if a, ok := parseAnimJSON(block.JSON); ok {
				if block.Label != "" {
					a.Label = block.Label
				}
				anims = append(anims, a)
			}
		}
		_ = retryContent
	}

	c.mu.Lock()
	session, ok := c.sessions[sessionID]
	if !ok {
		c.mu.Unlock()
		return
	}

	assistantMsg := Message{Role: RoleAssistant, Content: assistantContent, Time: time.Now(), Animations: anims}
	session.Messages = append(session.Messages, assistantMsg)
	session.UpdatedAt = time.Now()

	if c.db != nil {
		c.saveSessionMeta(session)
		var animationJSON string
		if len(anims) > 0 {
			if b, err := json.Marshal(anims); err == nil {
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
		"animations": anims,
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

func parseAnimsJSON(raw string) []UnifiedAnim {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return nil
	}
	if raw[0] == '[' {
		var anims []UnifiedAnim
		if err := json.Unmarshal([]byte(raw), &anims); err != nil {
			return nil
		}
		result := make([]UnifiedAnim, 0, len(anims))
		for _, a := range anims {
			if len(a.Elements) > 0 && len(a.Frames) > 0 {
				result = append(result, a)
			}
		}
		return result
	}
	var single UnifiedAnim
	if err := json.Unmarshal([]byte(raw), &single); err != nil {
		return nil
	}
	if len(single.Elements) > 0 && len(single.Frames) > 0 {
		return []UnifiedAnim{single}
	}
	return nil
}

func validateAnimBounds(anim UnifiedAnim) []string {
	const margin = 8.0
	w, h := anim.SVGWidth, anim.SVGHeight
	if w <= 0 || h <= 0 {
		return []string{fmt.Sprintf("svgW=%.0f svgH=%.0f 无效", w, h)}
	}
	var violations []string
	for _, e := range anim.Elements {
		switch e.Kind {
		case "rect":
			if e.X < margin {
				violations = append(violations, fmt.Sprintf("%s x=%.0f 太靠左", e.ID, e.X))
			}
			if e.Y < margin {
				violations = append(violations, fmt.Sprintf("%s y=%.0f 太靠上", e.ID, e.Y))
			}
			if e.W > 0 && e.X+e.W > w-margin {
				violations = append(violations, fmt.Sprintf("%s 超出右边 svgW=%.0f", e.ID, w))
			}
			if e.H > 0 && e.Y+e.H > h-margin {
				violations = append(violations, fmt.Sprintf("%s 超出下边 svgH=%.0f", e.ID, h))
			}
		case "circle":
			r := e.R
			if r <= 0 {
				r = 22
			}
			if e.X-r < margin {
				violations = append(violations, fmt.Sprintf("%s 圆超出左边", e.ID))
			}
			if e.X+r > w-margin {
				violations = append(violations, fmt.Sprintf("%s 圆超出右边 svgW=%.0f", e.ID, w))
			}
			if e.Y-r < margin {
				violations = append(violations, fmt.Sprintf("%s 圆超出上边", e.ID))
			}
			if e.Y+r > h-margin {
				violations = append(violations, fmt.Sprintf("%s 圆超出下边 svgH=%.0f", e.ID, h))
			}
		case "line":
			if e.X < -margin || e.X > w+margin {
				violations = append(violations, fmt.Sprintf("%s x1=%.0f 越界", e.ID, e.X))
			}
			if e.X2 < -margin || e.X2 > w+margin {
				violations = append(violations, fmt.Sprintf("%s x2=%.0f 越界", e.ID, e.X2))
			}
			if e.Y < -margin || e.Y > h+margin {
				violations = append(violations, fmt.Sprintf("%s y1=%.0f 越界", e.ID, e.Y))
			}
			if e.Y2 < -margin || e.Y2 > h+margin {
				violations = append(violations, fmt.Sprintf("%s y2=%.0f 越界", e.ID, e.Y2))
			}
		case "label":
			if e.X < -margin || e.X > w+margin {
				violations = append(violations, fmt.Sprintf("%s x=%.0f 越界 svgW=%.0f", e.ID, e.X, w))
			}
			if e.Y < -margin || e.Y > h+margin {
				violations = append(violations, fmt.Sprintf("%s y=%.0f 越界 svgH=%.0f", e.ID, e.Y, h))
			}
		}
	}
	if len(violations) > 5 {
		violations = append(violations[:5], fmt.Sprintf("...还有%d处越界", len(violations)-5))
	}
	return violations
}

func checkAnimsBounds(anims []UnifiedAnim) []string {
	var all []string
	for i, a := range anims {
		for _, v := range validateAnimBounds(a) {
			all = append(all, fmt.Sprintf("[动画%d]%s", i+1, v))
		}
	}
	return all
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

type statusInfo struct {
	Status string `json:"status"`
	Model  string `json:"model"`
}

func (c *ChatService) GetLLMStatus() string {
	c.mu.Lock()
	defer c.mu.Unlock()
	s := statusInfo{Status: "mock"}
	if c.llmClient != nil && c.llmClient.Available() {
		s.Status = "connected"
		s.Model = c.llmClient.Model()
	}
	b, _ := json.Marshal(s)
	return string(b)
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
