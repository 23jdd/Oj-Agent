package main

import (
	"context"
	"encoding/json"
	"fmt"
	"math/rand"
	"strings"
	"sync"
	"time"

	"Oj-Agent/llm"
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

// ---- Unified Animation Types ----

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

// ---- Helpers for building animations ----

func rect(id string, x, y, w, h float64, text string, style string, rx float64, visible bool) Element {
	if style == "" {
		style = string(StyleNormal)
	}
	return Element{ID: id, Kind: "rect", X: x, Y: y, W: w, H: h, Text: text, Style: style, RX: rx, Visible: visible}
}

func circle(id string, cx, cy, r float64, text, style string, visible bool) Element {
	return Element{ID: id, Kind: "circle", X: cx, Y: cy, R: r, Text: text, Style: style, Visible: visible}
}

func line(id string, x1, y1, x2, y2 float64, style string, arrow bool) Element {
	return Element{ID: id, Kind: "line", X: x1, Y: y1, X2: x2, Y2: y2, Style: style, Arrow: arrow, Visible: true}
}

func label(id string, x, y float64, text, style string) Element {
	return Element{ID: id, Kind: "label", X: x, Y: y, Text: text, Style: style, Visible: true}
}

func setX(v float64) map[string]interface{}   { return map[string]interface{}{"x": v} }
func setY(v float64) map[string]interface{}   { return map[string]interface{}{"y": v} }
func setXY(x, y float64) map[string]interface{} {
	return map[string]interface{}{"x": x, "y": y}
}
func setXYWH(x, y, w, h float64) map[string]interface{} {
	return map[string]interface{}{"x": x, "y": y, "w": w, "h": h}
}
func setLine(x1, y1, x2, y2 float64) map[string]interface{} {
	return map[string]interface{}{"x": x1, "y": y1, "x2": x2, "y2": y2}
}
func setText(v string) map[string]interface{}  { return map[string]interface{}{"text": v} }
func setStyle(v Style) map[string]interface{}   { return map[string]interface{}{"style": string(v)} }
func setColor(v string) map[string]interface{} { return map[string]interface{}{"style": v} }
func setVisible(v bool) map[string]interface{}  { return map[string]interface{}{"visible": v} }
func setXYText(x, y float64, text string) map[string]interface{} {
	return map[string]interface{}{"x": x, "y": y, "text": text}
}

// ---- ChatService ----

type ChatService struct {
	mu          sync.Mutex
	sessions    map[string]*ChatSession
	totalTokens int
	llmClient   *llm.Client
}

func NewChatService(llmClient *llm.Client) *ChatService {
	return &ChatService{
		sessions:  make(map[string]*ChatSession),
		llmClient: llmClient,
	}
}

func (c *ChatService) NewSession() *ChatSession {
	c.mu.Lock()
	defer c.mu.Unlock()
	id := fmt.Sprintf("session_%d", time.Now().UnixNano())
	session := &ChatSession{
		ID: id, Title: "新对话", Messages: []Message{},
		CreatedAt: time.Now(), UpdatedAt: time.Now(),
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
	return nil
}

func (c *ChatService) SendMessage(req SendMessageRequest) SendMessageResponse {
	c.mu.Lock()
	defer c.mu.Unlock()

	session, ok := c.sessions[req.SessionID]
	if !ok {
		session = &ChatSession{
			ID: req.SessionID, Title: req.Content, Messages: []Message{},
			CreatedAt: time.Now(), UpdatedAt: time.Now(),
		}
		c.sessions[req.SessionID] = session
	}

	userMsg := Message{Role: RoleUser, Content: req.Content, Time: time.Now()}
	session.Messages = append(session.Messages, userMsg)
	if len(session.Messages) == 2 {
		session.Title = truncateString(req.Content, 30)
	}

	problemType := detectProblemType(req.Content)
	var assistantMsg Message
	var anim UnifiedAnim

	if c.llmClient != nil && c.llmClient.Available() {
		llmResponse, err := c.llmClient.Generate(context.Background(), req.Content, req.Language, animRules)
		if err == nil && llmResponse != "" {
			assistantMsg = Message{Role: RoleAssistant, Content: llmResponse, Time: time.Now()}
			anim = c.buildMockAnim(problemType)
		} else {
			assistantMsg, anim = c.generateUnifiedResponse(req, problemType)
		}
	} else {
		assistantMsg, anim = c.generateUnifiedResponse(req, problemType)
	}

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
		TokenUsage:       TokenUsage{SessionTokens: sessionTokens, TotalTokens: c.totalTokens},
		Animation:        anim,
	}
}

func (c *ChatService) buildMockAnim(probType string) UnifiedAnim {
	switch probType {
	case "tree":
		_, anim := c.genTreeUnified(SendMessageRequest{Language: "go"})
		return anim
	case "dptable":
		_, anim := c.genDpUnified(SendMessageRequest{Language: "go"})
		return anim
	case "linkedlist":
		_, anim := c.genLinkedListUnified(SendMessageRequest{Language: "go"})
		return anim
	case "slidingwindow":
		_, anim := c.genSlidingWindowUnified(SendMessageRequest{Language: "go"})
		return anim
	case "sorting":
		_, anim := c.genSortingUnified(SendMessageRequest{Language: "go"})
		return anim
	case "twopointer":
		_, anim := c.genTwoPointerUnified(SendMessageRequest{Language: "go"})
		return anim
	default:
		_, anim := c.genArrayUnified(SendMessageRequest{Language: "go"})
		return anim
	}
}

var animRules = `1. 题目解析：识别类型（数组/链表/树/图/DP等）、输入输出、难度
2. 解题思路：核心思路1-2句 + 时间/空间复杂度
3. 算法步骤：拆分为5-10个关键步骤，每步是明确状态变化
4. 代码生成：完整可运行代码，变量名与步骤标签一致
5. 输出Markdown格式：## 题目分析 / ## 解题思路 / ## 算法步骤 / ## 代码实现`

func (c *ChatService) GetTokenUsage() TokenUsage {
	c.mu.Lock()
	defer c.mu.Unlock()
	return TokenUsage{TotalTokens: c.totalTokens}
}

func detectProblemType(content string) string {
	lower := strings.ToLower(content)
	switch {
	case strings.Contains(lower, "二叉树") || strings.Contains(lower, "binary tree") || strings.Contains(lower, "中序") || strings.Contains(lower, "前序") || strings.Contains(lower, "后序") || strings.Contains(lower, "层序"):
		return "tree"
	case strings.Contains(lower, "动态规划") || strings.Contains(lower, "dp") || strings.Contains(lower, "背包") || strings.Contains(lower, "斐波那契"):
		return "dptable"
	case strings.Contains(lower, "链表") || strings.Contains(lower, "linked list") || strings.Contains(lower, "反转"):
		return "linkedlist"
	case strings.Contains(lower, "滑动窗口") || strings.Contains(lower, "sliding window"):
		return "slidingwindow"
	case strings.Contains(lower, "排序") || strings.Contains(lower, "sort") || strings.Contains(lower, "快排") || strings.Contains(lower, "冒泡") || strings.Contains(lower, "归并"):
		return "sorting"
	case strings.Contains(lower, "双指针") || strings.Contains(lower, "two pointer") || strings.Contains(lower, "回文") || strings.Contains(lower, "盛水"):
		return "twopointer"
	case strings.Contains(lower, "图") || strings.Contains(lower, "graph") || strings.Contains(lower, "dfs") || strings.Contains(lower, "bfs"):
		return "graph"
	default:
		types := []string{"array", "twopointer", "tree", "dptable"}
		return types[rand.Intn(len(types))]
	}
}

func (c *ChatService) generateUnifiedResponse(req SendMessageRequest, probType string) (Message, UnifiedAnim) {
	switch probType {
	case "tree":
		return c.genTreeUnified(req)
	case "dptable":
		return c.genDpUnified(req)
	case "linkedlist":
		return c.genLinkedListUnified(req)
	case "slidingwindow":
		return c.genSlidingWindowUnified(req)
	case "sorting":
		return c.genSortingUnified(req)
	case "twopointer":
		return c.genTwoPointerUnified(req)
	default:
		return c.genArrayUnified(req)
	}
}

// ---- Unified Animation Generators ----

func (c *ChatService) genArrayUnified(req SendMessageRequest) (Message, UnifiedAnim) {
	vals := []int{2, 7, 11, 15, 3, 8, 5}
	code := fmt.Sprintf("```%s\nfunc twoSum(nums []int, target int) []int {\n    m := make(map[int]int)\n    for i, v := range nums {\n        if j, ok := m[target-v]; ok {\n            return []int{j, i}\n        }\n        m[v] = i\n    }\n    return nil\n}\n```", req.Language)
	content := fmt.Sprintf(`## 题目分析
使用**哈希表**以 O(n) 时间找出两数之和等于目标值。

## 解题思路
遍历数组，每次检查 target-当前值 是否已在哈希表中。

## 算法步骤
1. i=0, v=2, 存入 map{2:0}
2. i=1, v=7, 查找 target-7=2，命中 map！返回 [0,1]

## 代码实现
%s`, code)

	bw, bh, gap := 48.0, 42.0, 56.0
	padL, padT := 30.0, 30.0
	n := len(vals)

	elements := []Element{}
	for i, v := range vals {
		elements = append(elements,
			rect(fmt.Sprintf("cell_%d", i), padL+float64(i)*gap, padT, bw, bh, fmt.Sprint(v), string(StyleNormal), 6, true),
		)
	}
	elements = append(elements,
		label("hash_hint", padL, padT+bh+36, "哈希表: {}", string(StyleDim)),
	)

	frames := []Frame{
		{Description: "初始数组", Delta: map[string]interface{}{}},
		{
			Description: "i=0, v=2, 存入哈希表",
			Delta: map[string]interface{}{
				"cell_0":    setStyle(StyleHighlight),
				"hash_hint": setText("哈希表: {2:0}"),
			},
		},
		{
			Description: "i=1, v=7, 查找 target-7=2 → 命中！",
			Delta: map[string]interface{}{
				"cell_0": setStyle(StyleCompare),
				"cell_1": setStyle(StyleHighlight),
			},
		},
		{
			Description: "返回 [0, 1]",
			Delta: map[string]interface{}{
				"cell_0": setStyle(StyleResult),
				"cell_1": setStyle(StyleResult),
			},
		},
	}

	anim := UnifiedAnim{
		Elements:  elements,
		Frames:    frames,
		SVGWidth:  padL*2 + float64(n)*gap,
		SVGHeight: padT*2 + bh + 60,
	}
	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, anim
}

func (c *ChatService) genTwoPointerUnified(req SendMessageRequest) (Message, UnifiedAnim) {
	vals := []int{1, 8, 6, 2, 5, 4, 8, 3, 7}
	code := fmt.Sprintf("```%s\nfunc maxArea(height []int) int {\n    l, r := 0, len(height)-1\n    maxA := 0\n    for l < r {\n        h := min(height[l], height[r])\n        a := h * (r - l)\n        if a > maxA { maxA = a }\n        if height[l] < height[r] { l++ } else { r-- }\n    }\n    return maxA\n}\n```", req.Language)
	content := fmt.Sprintf(`## 题目分析
**盛最多水的容器**，双指针从两端向中间收缩。

## 解题思路
左右指针指向两端，每次移动较矮侧，更新最大面积。
- O(n) 时间, O(1) 空间

## 代码实现
%s`, code)

	bw, bh, gap := 44.0, 40.0, 52.0
	padL, padT := 30.0, 50.0
	n := len(vals)

	elements := []Element{}
	for i, v := range vals {
		elements = append(elements,
			rect(fmt.Sprintf("cell_%d", i), padL+float64(i)*gap, padT, bw, bh, fmt.Sprint(v), string(StyleNormal), 6, true),
		)
	}
	elements = append(elements,
		label("ptr_l", padL, padT-18, "▲ L", string(StyleDim)),
		label("ptr_r", padL+float64(n-1)*gap+bw/2, padT-18, "▲ R", string(StyleDim)),
	)

	frames := []Frame{
		{Description: "初始化: L=0, R=8", Delta: map[string]interface{}{
			"cell_0": setStyle(StyleHighlight),
			"cell_8": setStyle(StyleHighlight),
			"ptr_l":  setXYText(padL+bw/2, padT-18, "▲ L"),
			"ptr_r":  setXYText(padL+float64(n-1)*gap+bw/2, padT-18, "▲ R"),
		}},
		{Description: "h=min(1,7)=1, area=1×8=8, L矮→L++", Delta: map[string]interface{}{
			"cell_0": setStyle(StyleCompare),
			"cell_8": setStyle(StyleCompare),
		}},
		{Description: "L=1(v=8), R=8(v=7), area=7×7=49, 更新max=49", Delta: map[string]interface{}{
			"cell_0": setStyle(StyleDim),
			"cell_1": setStyle(StyleHighlight),
			"ptr_l":  setXYText(padL+1*gap+bw/2, padT-18, "▲ L"),
		}},
		{Description: "R矮→R=7(v=3), area=3×6=18", Delta: map[string]interface{}{
			"cell_8": setStyle(StyleDim),
			"cell_7": setStyle(StyleHighlight),
			"ptr_r":  setXYText(padL+7*gap+bw/2, padT-18, "▲ R"),
		}},
		{Description: "L≥R 结束，maxArea=49", Delta: map[string]interface{}{
			"cell_1": setStyle(StyleResult),
			"cell_8": setStyle(StyleResult),
		}},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, UnifiedAnim{
		Elements: elements, Frames: frames,
		SVGWidth: padL*2 + float64(n)*gap, SVGHeight: padT*2 + bh + 40,
	}
}

func (c *ChatService) genLinkedListUnified(req SendMessageRequest) (Message, UnifiedAnim) {
	code := fmt.Sprintf("```%s\ntype ListNode struct { Val int; Next *ListNode }\nfunc reverseList(head *ListNode) *ListNode {\n    var prev *ListNode\n    curr := head\n    for curr != nil {\n        next := curr.Next\n        curr.Next = prev\n        prev = curr\n        curr = next\n    }\n    return prev\n}\n```", req.Language)
	content := fmt.Sprintf(`## 题目分析
**反转链表**，迭代翻转每个节点的 Next 指针。

## 算法步骤
1. prev=nil, curr=1
2. 保存 next=2, curr.Next=nil, prev=1, curr=2
3. 继续直到 curr=nil, 返回 prev=4

## 代码实现
%s`, code)

	nodeW, nodeH := 52.0, 38.0
	gap := 70.0
	padL, padT := 30.0, 50.0
	vals := []string{"1", "2", "3", "4"}

	elements := []Element{}
	for i, v := range vals {
		x := padL + float64(i)*gap
		// Main node rect
		elements = append(elements, rect(fmt.Sprintf("n%d", i), x, padT, nodeW, nodeH, v, string(StyleNormal), 6, true))
		// Next-divider line (visual)
		elements = append(elements, line(fmt.Sprintf("div_%d", i), x+nodeW*0.65, padT, x+nodeW*0.65, padT+nodeH, "#4b5563", false))
		// Arrow between nodes
		if i < len(vals)-1 {
			elements = append(elements, line(fmt.Sprintf("edge_%d", i), x+nodeW, padT+nodeH/2, x+gap, padT+nodeH/2, "#4b5563", true))
		}
	}
	elements = append(elements,
		label("head_label", padL+nodeW/2, padT-16, "head", string(StyleDim)),
		label("null_label", padL+float64(len(vals)-1)*gap+nodeW/2, padT+nodeH+22, "null", string(StyleDim)),
	)

	frames := []Frame{
		{Description: "初始: 1→2→3→4→null", Delta: map[string]interface{}{"n0": setStyle(StyleHighlight)}},
		{Description: "prev=nil, curr=1, next=2", Delta: map[string]interface{}{
			"n0": setStyle(StyleHighlight),
			"n1": setStyle(StyleHighlight),
		}},
		{Description: "1.Next=nil, prev=1, curr=2", Delta: map[string]interface{}{
			"n0":       setStyle(StyleResult),
			"n1":       setStyle(StyleHighlight),
			"edge_0":   setStyle(StyleDim),
			"n2":       setStyle(StyleHighlight),
		}},
		{Description: "2.Next=1, prev=2, curr=3", Delta: map[string]interface{}{
			"n1": setStyle(StyleResult), "n2": setStyle(StyleHighlight), "n3": setStyle(StyleHighlight),
		}},
		{Description: "3.Next=2, prev=3, curr=4", Delta: map[string]interface{}{
			"n2": setStyle(StyleResult), "n3": setStyle(StyleHighlight),
		}},
		{Description: "4.Next=3, prev=4, curr=nil → 完成!", Delta: map[string]interface{}{
			"n3": setStyle(StyleResult),
		}},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, UnifiedAnim{
		Elements: elements, Frames: frames,
		SVGWidth: padL*2 + float64(len(vals))*gap, SVGHeight: padT*2 + nodeH + 60,
	}
}

func (c *ChatService) genSlidingWindowUnified(req SendMessageRequest) (Message, UnifiedAnim) {
	vals := []int{2, 3, 1, 2, 4, 3, 5}
	code := fmt.Sprintf("```%s\nfunc minSubArrayLen(target int, nums []int) int {\n    l, sum, minL := 0, 0, math.MaxInt32\n    for r := 0; r < len(nums); r++ {\n        sum += nums[r]\n        for sum >= target {\n            if r-l+1 < minL { minL = r-l+1 }\n            sum -= nums[l]; l++\n        }\n    }\n    return minL\n}\n```", req.Language)
	content := fmt.Sprintf(`## 题目分析
**长度最小子数组**，滑动窗口找和≥target的最短连续子数组。

## 解题思路
右指针扩展窗口，和≥target时收缩左指针求最小长度。

## 代码实现
%s`, code)

	bw, bh, gap := 44.0, 40.0, 52.0
	padL, padT := 30.0, 55.0
	n := len(vals)

	elements := []Element{}
	for i, v := range vals {
		elements = append(elements,
			rect(fmt.Sprintf("c%d", i), padL+float64(i)*gap, padT, bw, bh, fmt.Sprint(v), string(StyleNormal), 6, true),
		)
	}
	elements = append(elements,
		line("win_l", 0, padT-8, 0, padT+8, "transparent", false),
		line("win_r", 0, padT-8, 0, padT+8, "transparent", false),
		line("win_top", 0, padT-6, 0, padT-6, "transparent", false),
		label("win_label", 0, 0, "", string(StyleDim)),
	)

	rx := func(idx float64) float64 { return padL + idx*gap }

		frames := []Frame{
		{Description: "初始状态", Delta: map[string]interface{}{}},
		{Description: "R=0, sum=2 < 7", Delta: map[string]interface{}{
			"c0": setStyle(StyleHighlight),
			"win_l":  setLine(rx(0), padT-8, rx(0), padT+8),
			"win_r":  setLine(rx(0)+bw, padT-8, rx(0)+bw, padT+8),
			"win_top": setLine(rx(0), padT-6, rx(0)+bw, padT-6),
		}},
		{Description: "R=3, sum=8≥7, minLen=4", Delta: map[string]interface{}{
			"c0": setStyle(StyleCompare), "c1": setStyle(StyleCompare),
			"c2": setStyle(StyleCompare), "c3": setStyle(StyleCompare),
			"win_r":  setLine(rx(3)+bw, padT-8, rx(3)+bw, padT+8),
			"win_top": setLine(rx(0), padT-6, rx(3)+bw, padT-6),
			"win_label": setXYText((rx(0)+rx(3)+bw)/2, padT-16, "len=4"),
		}},
		{Description: "收缩 L=1, sum=6<7, 继续扩展R=4, sum=10≥7", Delta: map[string]interface{}{
			"c0": setStyle(StyleDim), "c1": setStyle(StyleHighlight), "c4": setStyle(StyleCompare),
			"win_l": setLine(rx(1), padT-8, rx(1), padT+8),
			"win_top": setLine(rx(1), padT-6, rx(4)+bw, padT-6),
		}},
		{Description: "最终 minLen=2 (子数组[4,3])", Delta: map[string]interface{}{
			"c4": setStyle(StyleResult), "c5": setStyle(StyleResult),
			"win_l": setLine(rx(4), padT-8, rx(4), padT+8),
			"win_r": setLine(rx(5)+bw, padT-8, rx(5)+bw, padT+8),
			"win_top": setLine(rx(4), padT-6, rx(5)+bw, padT-6),
			"win_label": setXYText((rx(4)+rx(5)+bw)/2, padT-16, "minLen=2"),
		}},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, UnifiedAnim{
		Elements: elements, Frames: frames,
		SVGWidth: padL*2 + float64(n)*gap, SVGHeight: padT*2 + bh + 40,
	}
}

func (c *ChatService) genSortingUnified(req SendMessageRequest) (Message, UnifiedAnim) {
	vals := []int{5, 3, 8, 4, 2, 7, 1, 6}
	code := fmt.Sprintf("```%s\nfunc partition(nums []int, lo, hi int) int {\n    pivot := nums[hi]\n    i := lo\n    for j := lo; j < hi; j++ {\n        if nums[j] < pivot {\n            nums[i], nums[j] = nums[j], nums[i]\n            i++\n        }\n    }\n    nums[i], nums[hi] = nums[hi], nums[i]\n    return i\n}\n```", req.Language)
	content := fmt.Sprintf(`## 题目分析
**快速排序 Partition**，以最后一个元素为 pivot 划分数组。

## 算法步骤
小于 pivot 的元素交换到左边，大于的在右边。

## 代码实现
%s`, code)

	bw, bh, gap := 44.0, 40.0, 52.0
	padL, padT := 30.0, 55.0
	n := len(vals)
	pivotIdx := n - 1

	elements := []Element{}
	for i, v := range vals {
		elements = append(elements,
			rect(fmt.Sprintf("c%d", i), padL+float64(i)*gap, padT, bw, bh, fmt.Sprint(v), string(StyleNormal), 6, true),
		)
	}
	elements = append(elements,
		label("pivot_area", padL+float64(pivotIdx)*gap+bw/2, padT-24, "▲ pivot", string(StyleDim)),
	)

	t := func(idx int) float64 { return padL + float64(idx)*gap + bw/2 }

	frames := []Frame{
		{Description: "初始数组, pivot=6(idx=7)", Delta: map[string]interface{}{
			"c7": setStyle(StylePivot),
			"pivot_area": setXYText(t(7), padT-24, "▲ pivot=6"),
		}},
		{Description: "5<6 ✓ swap(i=0,j=0)", Delta: map[string]interface{}{
			"c0": setStyle(StyleCompare),
		}},
		{Description: "3<6 ✓ swap(i=1,j=1)", Delta: map[string]interface{}{
			"c0": setStyle(StyleResult), "c1": setStyle(StyleCompare),
		}},
		{Description: "8>6 ✗ 跳过", Delta: map[string]interface{}{
			"c1": setStyle(StyleResult), "c2": setStyle(StyleSwap),
		}},
		{Description: "4<6 ✓ swap(i=2,j=3) → [5,3,4,8,...]", Delta: map[string]interface{}{
			"c2": setStyle(StyleResult), "c3": setStyle(StyleHighlight),
		}},
		{Description: "2<6 ✓ swap(i=3,j=4) → [5,3,4,2,...]", Delta: map[string]interface{}{
			"c3": setStyle(StyleResult), "c4": setStyle(StyleHighlight),
		}},
		{Description: "7>6 ✗ 跳过", Delta: map[string]interface{}{
			"c4": setStyle(StyleResult), "c5": setStyle(StyleSwap),
		}},
		{Description: "1<6 ✓ swap(i=4,j=6)", Delta: map[string]interface{}{
			"c5": setStyle(StyleResult), "c6": setStyle(StyleHighlight),
		}},
		{Description: "pivot 归位, swap(i=5,pivot) → [5,3,4,2,1,6,8,7]", Delta: map[string]interface{}{
			"c5": setStyle(StyleResult), "c6": setStyle(StyleDim),
			"c7":       setStyle(StyleResult),
			"pivot_area": setXYText(t(5), padT-24, ""),
		}},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, UnifiedAnim{
		Elements: elements, Frames: frames,
		SVGWidth: padL*2 + float64(n)*gap, SVGHeight: padT*2 + bh + 40,
	}
}

func (c *ChatService) genTreeUnified(req SendMessageRequest) (Message, UnifiedAnim) {
	code := fmt.Sprintf("```%s\ntype TreeNode struct { Val int; Left, Right *TreeNode }\nfunc inorder(root *TreeNode) []int {\n    if root == nil { return nil }\n    res := inorder(root.Left)\n    res = append(res, root.Val)\n    res = append(res, inorder(root.Right)...)\n    return res\n}\n```", req.Language)
	content := fmt.Sprintf(`## 题目分析
**二叉树中序遍历**: 左→根→右，递归访问所有节点。

## 算法步骤
结果: [4, 2, 5, 1, 3]

## 代码实现
%s`, code)

	// Node positions (centered tree layout)
	//         1(300,40)
	//       /      \
	//   2(170,150)  3(430,150)
	//   /   \
	// 4(80,260) 5(260,260)

	nodes := []struct{ id, val string; cx, cy float64 }{ {"1","1",300,50}, {"2","2",170,160}, {"3","3",430,160}, {"4","4",80,270}, {"5","5",260,270} }
	edges := []struct{ from, to string }{ {"1","2"}, {"1","3"}, {"2","4"}, {"2","5"} }

	elements := []Element{}
	for _, n := range nodes {
		elements = append(elements, circle(fmt.Sprintf("n_%s", n.id), n.cx, n.cy, 22, n.val, string(StyleNormal), true))
	}
	for i, e := range edges {
		fn := nodeByID(nodes, e.from)
		tn := nodeByID(nodes, e.to)
		elements = append(elements, line(fmt.Sprintf("edge_%d", i), fn.cx, fn.cy+22, tn.cx, tn.cy-22, "#4b5563", false))
	}
	elements = append(elements,
		label("path_label", 320, 330, "", string(StyleDim)),
	)

	frames := []Frame{
		{Description: "从根节点 1 开始", Delta: map[string]interface{}{"n_1": setStyle(StyleHighlight)}},
		{Description: "递归左子树 → 节点 2", Delta: map[string]interface{}{
			"n_1": setStyle(StyleDim), "n_2": setStyle(StyleHighlight), "edge_0": setColor("#3b82f6"),
		}},
		{Description: "递归左子树 → 节点 4", Delta: map[string]interface{}{
			"n_2": setStyle(StyleDim), "n_4": setStyle(StyleHighlight), "edge_2": setColor("#3b82f6"),
		}},
		{Description: "节点4无左子，输出 4", Delta: map[string]interface{}{
			"n_4": setStyle(StyleResult), "path_label": setText("输出: [4]"),
		}},
		{Description: "回溯2，输出 2", Delta: map[string]interface{}{
			"n_2": setStyle(StyleResult), "path_label": setText("输出: [4, 2]"),
		}},
		{Description: "进入节点5，输出 5", Delta: map[string]interface{}{
			"n_5": setStyle(StyleResult), "edge_3": setStyle("#3b82f6"), "path_label": setText("输出: [4, 2, 5]"),
		}},
		{Description: "回溯根节点1，输出 1", Delta: map[string]interface{}{
			"n_1": setStyle(StyleResult), "path_label": setText("输出: [4, 2, 5, 1]"),
		}},
		{Description: "进入右子树3，输出 3 → 完成 [4,2,5,1,3]", Delta: map[string]interface{}{
			"n_3": setStyle(StyleResult), "edge_1": setStyle("#3b82f6"), "path_label": setText("输出: [4, 2, 5, 1, 3]"),
		}},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, UnifiedAnim{
		Elements: elements, Frames: frames, SVGWidth: 560, SVGHeight: 380,
	}
}

func (c *ChatService) genDpUnified(req SendMessageRequest) (Message, UnifiedAnim) {
	code := fmt.Sprintf("```%s\nfunc knapsack(w []int, v []int, cap int) int {\n    n := len(w)\n    dp := make([][]int, n+1)\n    for i := range dp { dp[i] = make([]int, cap+1) }\n    for i := 1; i <= n; i++ {\n        for j := 0; j <= cap; j++ {\n            if w[i-1] > j { dp[i][j] = dp[i-1][j] }else{ dp[i][j] = max(dp[i-1][j], dp[i-1][j-w[i-1]]+v[i-1]) }\n        }\n    }\n    return dp[n][cap]\n}\n```", req.Language)
	content := fmt.Sprintf(`## 题目分析
**0/1 背包** — 经典DP，dp[i][j]=前i个物品容量j的最大价值。

## 算法步骤
w=[2,3,4], v=[3,4,5], cap=5

## 代码实现
%s`, code)

	// Build DP table: 4 rows x 6 cols
	grid := [][]string{
		{"0", "0", "0", "0", "0", "0"},
		{"0", "0", "3", "3", "3", "3"},
		{"0", "0", "3", "4", "4", "7"},
		{"0", "0", "3", "4", "5", "7"},
	}
	rowH := []string{"空", "物品1(2kg/3)", "物品2(3kg/4)", "物品3(4kg/5)"}
	colH := []string{"0", "1", "2", "3", "4", "5"}

	cw, ch := 44.0, 34.0
	hw := 100.0
	pad := 10.0

	elements := []Element{}
	// Col headers
	for ci, h := range colH {
		elements = append(elements, rect(fmt.Sprintf("ch_%d", ci), hw+float64(ci)*cw, pad, cw, ch, h, string(StyleNormal), 0, true))
	}
	// Row headers + cells
	for ri, row := range grid {
		elements = append(elements, rect(fmt.Sprintf("rh_%d", ri), pad, ch+pad+float64(ri)*ch, hw-pad, ch, rowH[ri], string(StyleNormal), 0, true))
		for ci, val := range row {
			elements = append(elements, rect(fmt.Sprintf("cell_%d_%d", ri, ci), hw+float64(ci)*cw, ch+pad+float64(ri)*ch, cw, ch, val, string(StyleNormal), 0, true))
		}
	}

	frames := []Frame{
		{Description: "初始化 dp[0][*]=0", Delta: map[string]interface{}{}},
		{Description: "i=1, j=2: w[0]=2≤2, dp[1][2]=3", Delta: map[string]interface{}{
			"cell_1_2": setStyle(StyleHighlight),
		}},
		{Description: "i=2, j=3: max(3, dp[1][0]+4)=4", Delta: map[string]interface{}{
			"cell_1_2": setStyle(StyleResult),
			"cell_2_3": setStyle(StyleHighlight),
		}},
		{Description: "i=2, j=5: max(3, dp[1][2]+4=7)=7", Delta: map[string]interface{}{
			"cell_2_3": setStyle(StyleResult),
			"cell_2_5": setStyle(StyleHighlight),
		}},
		{Description: "i=3, j=4: max(4, dp[2][0]+5=5)=5", Delta: map[string]interface{}{
			"cell_2_5": setStyle(StyleResult),
			"cell_3_4": setStyle(StyleHighlight),
		}},
		{Description: "最终答案 dp[3][5]=7", Delta: map[string]interface{}{
			"cell_3_4": setStyle(StyleDim),
			"cell_3_5": setStyle(StyleResult),
		}},
	}

	rows, cols := float64(len(grid)), float64(len(colH))
	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, UnifiedAnim{
		Elements: elements, Frames: frames,
		SVGWidth: hw + cols*cw + pad, SVGHeight: ch + pad + rows*ch + pad,
	}
}

// ---- Helpers ----

func nodeByID(nodes []struct{ id, val string; cx, cy float64 }, id string) struct{ id, val string; cx, cy float64 } {
	for _, n := range nodes {
		if n.id == id {
			return n
		}
	}
	return nodes[0]
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
var _ = json.Marshal
