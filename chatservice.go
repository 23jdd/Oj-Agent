package main

import (
	"encoding/json"
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
	UserMessage      Message       `json:"userMessage"`
	AssistantMessage Message       `json:"assistantMessage"`
	SessionID        string        `json:"sessionId"`
	TokenUsage       TokenUsage    `json:"tokenUsage"`
	Animation        AnimationData `json:"animation"`
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

// ---- Animation Data Types ----

type AnimationData struct {
	Type string          `json:"type"`
	Array *ArrayData     `json:"array,omitempty"`
	Tree  *TreeData      `json:"tree,omitempty"`
	Table *DpTableData   `json:"table,omitempty"`
	List  *LinkedListData `json:"list,omitempty"`
	Steps []AnimStep     `json:"steps"`
}

type ArrayData struct {
	Values []int    `json:"values"`
	Labels []string `json:"labels"`
}

type TreeNodeData struct {
	ID    string `json:"id"`
	Value string `json:"value"`
	Left  string `json:"left,omitempty"`
	Right string `json:"right,omitempty"`
	X     int    `json:"x"`
	Y     int    `json:"y"`
}

type TreeData struct {
	Nodes []TreeNodeData `json:"nodes"`
	Root  string         `json:"root"`
}

type DpTableData struct {
	Rows       int      `json:"rows"`
	Cols       int      `json:"cols"`
	RowHeaders []string `json:"rowHeaders"`
	ColHeaders []string `json:"colHeaders"`
}

type ListNodeData struct {
	ID    string `json:"id"`
	Value string `json:"value"`
	Next  string `json:"next,omitempty"`
	X     int    `json:"x"`
	Y     int    `json:"y"`
}

type LinkedListData struct {
	Nodes []ListNodeData `json:"nodes"`
	Head  string         `json:"head"`
}

type AnimStep struct {
	Description string   `json:"description"`

	HighlightIdx []int  `json:"highlightIdx,omitempty"`
	CompareIdx   []int  `json:"compareIdx,omitempty"`
	ResultIdx    []int  `json:"resultIdx,omitempty"`
	SwapIdx      []int  `json:"swapIdx,omitempty"`
	PivotIdx     int    `json:"pivotIdx,omitempty"`
	PointerLeft  int    `json:"pointerLeft,omitempty"`
	PointerRight int    `json:"pointerRight,omitempty"`
	PointerMid   int    `json:"pointerMid,omitempty"`
	WindowStart  int    `json:"windowStart,omitempty"`
	WindowEnd    int    `json:"windowEnd,omitempty"`
	Values       []int  `json:"values,omitempty"`

	Row       int        `json:"row,omitempty"`
	Col       int        `json:"col,omitempty"`
	CellValue string     `json:"cellValue,omitempty"`
	TableGrid [][]string `json:"tableGrid,omitempty"`

	NodeID    string         `json:"nodeId,omitempty"`
	NodePath  []string       `json:"nodePath,omitempty"`
	ListNodes []ListNodeData `json:"listNodes,omitempty"`
}

// ---- ChatService ----

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

	userMsg := Message{Role: RoleUser, Content: req.Content, Time: time.Now()}
	session.Messages = append(session.Messages, userMsg)

	if len(session.Messages) == 2 {
		session.Title = truncateString(req.Content, 30)
	}

	problemType := detectProblemType(req.Content)
	assistantMsg, animData := c.generateResponse(req, problemType)
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
		Animation: animData,
	}
}

func (c *ChatService) GetTokenUsage() TokenUsage {
	c.mu.Lock()
	defer c.mu.Unlock()
	return TokenUsage{TotalTokens: c.totalTokens}
}

// ---- Problem Detection ----

func detectProblemType(content string) string {
	lower := strings.ToLower(content)
	switch {
	case strings.Contains(lower, "两数之和") || strings.Contains(lower, "two sum"):
		return "array"
	case strings.Contains(lower, "回文") || strings.Contains(lower, "palindrom"):
		return "twopointer"
	case strings.Contains(lower, "二分") || strings.Contains(lower, "binary search"):
		return "array"
	case strings.Contains(lower, "二叉树") || strings.Contains(lower, "binary tree") || strings.Contains(lower, "遍历"):
		return "tree"
	case strings.Contains(lower, "动态规划") || strings.Contains(lower, "dp") || strings.Contains(lower, "背包") || strings.Contains(lower, "斐波那契"):
		return "dptable"
	case strings.Contains(lower, "链表") || strings.Contains(lower, "linked list") || strings.Contains(lower, "反转链表"):
		return "linkedlist"
	case strings.Contains(lower, "图") || strings.Contains(lower, "graph") || strings.Contains(lower, "dfs") || strings.Contains(lower, "bfs"):
		return "graph"
	case strings.Contains(lower, "滑动窗口") || strings.Contains(lower, "sliding window"):
		return "slidingwindow"
	case strings.Contains(lower, "双指针") || strings.Contains(lower, "two pointer"):
		return "twopointer"
	case strings.Contains(lower, "排序") || strings.Contains(lower, "sort") || strings.Contains(lower, "快排") || strings.Contains(lower, "冒泡") || strings.Contains(lower, "归并"):
		return "sorting"
	default:
		types := []string{"array", "twopointer", "tree", "dptable"}
		return types[rand.Intn(len(types))]
	}
}

// ---- Response Generation ----

func (c *ChatService) generateResponse(req SendMessageRequest, probType string) (Message, AnimationData) {
	switch probType {
	case "tree":
		return c.genTreeResponse(req)
	case "dptable":
		return c.genDpResponse(req)
	case "twopointer":
		return c.genTwoPointerResponse(req)
	case "linkedlist":
		return c.genLinkedListResponse(req)
	case "slidingwindow":
		return c.genSlidingWindowResponse(req)
	case "sorting":
		return c.genSortingResponse(req)
	default:
		return c.genArrayResponse(req)
	}
}

func (c *ChatService) genArrayResponse(req SendMessageRequest) (Message, AnimationData) {
	values := []int{2, 7, 11, 15, 3, 8, 5}
	code := fmt.Sprintf("```%s\nfunc twoSum(nums []int, target int) []int {\n    m := make(map[int]int)\n    for i, v := range nums {\n        if j, ok := m[target-v]; ok {\n            return []int{j, i}\n        }\n        m[v] = i\n    }\n    return nil\n}\n```", req.Language)

	content := fmt.Sprintf(`## 题目分析
这是一个典型的**哈希表查找**问题，需要从数组中找出两个数之和等于目标值。

## 解题思路
使用哈希表存储已遍历元素的值和索引，实现 O(n) 时间复杂度。
- 时间复杂度：O(n)
- 空间复杂度：O(n)

## 算法步骤
1. 初始化哈希表 map[2:0]
2. 遍历 nums[1]=7，查找 map 中是否有 target-7=2，命中！返回 [0,1]

## 代码实现
%s`, code)

	anim := AnimationData{
		Type: "array",
		Array: &ArrayData{
			Values: values,
			Labels: []string{"i=0", "i=1", "i=2", "i=3", "i=4", "i=5", "i=6"},
		},
		Steps: []AnimStep{
			{Description: "初始数组", Values: values, HighlightIdx: nil},
			{Description: "i=0, v=2, 存入哈希表", Values: values, HighlightIdx: []int{0}},
			{Description: "i=1, v=7, 查找 target-7=2", Values: values, CompareIdx: []int{0, 1}},
			{Description: "在哈希表中找到2，返回[0,1]", Values: values, ResultIdx: []int{0, 1}},
		},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, anim
}

func (c *ChatService) genTwoPointerResponse(req SendMessageRequest) (Message, AnimationData) {
	values := []int{1, 8, 6, 2, 5, 4, 8, 3, 7}
	code := fmt.Sprintf("```%s\nfunc maxArea(height []int) int {\n    left, right := 0, len(height)-1\n    maxArea := 0\n    for left < right {\n        h := min(height[left], height[right])\n        area := h * (right - left)\n        if area > maxArea { maxArea = area }\n        if height[left] < height[right] { left++ } else { right-- }\n    }\n    return maxArea\n}\n```", req.Language)

	content := fmt.Sprintf(`## 题目分析
**盛最多水的容器**，使用双指针从两端向中间收缩。

## 解题思路
左右指针分别指向数组两端，每次移动较矮的一侧，计算并更新最大面积。
- 时间复杂度：O(n)
- 空间复杂度：O(1)

## 算法步骤
1. left=0(value=1), right=8(value=7), area=1*8=8
2. left 矮，left++ → left=1(value=8)
3. left=1(8), right=8(7), area=7*7=49, 更新 maxArea=49
4. right 矮，right-- → right=7(value=3)
5. 继续移动直到 left >= right，最终 maxArea=49

## 代码实现
%s`, code)

	anim := AnimationData{
		Type: "twopointer",
		Array: &ArrayData{
			Values: values,
			Labels: []string{"L", "", "", "", "", "", "", "R", ""},
		},
		Steps: []AnimStep{
			{Description: "初始化 L=0, R=8", Values: values, PointerLeft: 0, PointerRight: 8, HighlightIdx: []int{0, 8}},
			{Description: "h=min(1,7)=1, area=8", Values: values, PointerLeft: 0, PointerRight: 8, CompareIdx: []int{0, 8}},
			{Description: "左矮，L++ → L=1", Values: values, PointerLeft: 1, PointerRight: 8, HighlightIdx: []int{1, 8}},
			{Description: "h=min(8,7)=7, area=49", Values: values, PointerLeft: 1, PointerRight: 8, CompareIdx: []int{1, 8}},
			{Description: "右矮，R-- → R=7", Values: values, PointerLeft: 1, PointerRight: 7, HighlightIdx: []int{1, 7}},
			{Description: "L=1, R=7, h=min(8,3)=3, area=18", Values: values, PointerLeft: 1, PointerRight: 7, CompareIdx: []int{1, 7}},
			{Description: "最终 maxArea=49", Values: values, ResultIdx: []int{1, 8}},
		},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, anim
}

func (c *ChatService) genTreeResponse(req SendMessageRequest) (Message, AnimationData) {
	code := fmt.Sprintf("```%s\ntype TreeNode struct {\n    Val   int\n    Left  *TreeNode\n    Right *TreeNode\n}\n\nfunc inorderTraversal(root *TreeNode) []int {\n    var result []int\n    var dfs func(*TreeNode)\n    dfs = func(node *TreeNode) {\n        if node == nil { return }\n        dfs(node.Left)\n        result = append(result, node.Val)\n        dfs(node.Right)\n    }\n    dfs(root)\n    return result\n}\n```", req.Language)

	content := fmt.Sprintf(`## 题目分析
**二叉树的中序遍历**，按照 左→根→右 的顺序遍历所有节点。

## 解题思路
使用递归 DFS，先递归左子树，再访问根节点，最后递归右子树。
- 时间复杂度：O(n)
- 空间复杂度：O(h)，h 为树高

## 算法步骤
1. 从根节点1开始，递归进入左子树
2. 访问节点2，递归进入左子树
3. 访问节点4，无左子树，输出 4
4. 回溯到2，输出 2
5. 进入2的右子树，访问节点5，输出 5
6. 回溯到1，输出 1
7. 进入1的右子树，访问节点3，输出 3
8. 遍历完成：中序结果 [4, 2, 5, 1, 3]

## 代码实现
%s`, code)

	nodes := []TreeNodeData{
		{ID: "1", Value: "1", Left: "2", Right: "3", X: 300, Y: 40},
		{ID: "2", Value: "2", Left: "4", Right: "5", X: 180, Y: 140},
		{ID: "3", Value: "3", Left: "", Right: "", X: 420, Y: 140},
		{ID: "4", Value: "4", Left: "", Right: "", X: 100, Y: 240},
		{ID: "5", Value: "5", Left: "", Right: "", X: 260, Y: 240},
	}

	anim := AnimationData{
		Type: "tree",
		Tree: &TreeData{Nodes: nodes, Root: "1"},
		Steps: []AnimStep{
			{Description: "从根节点 1 开始", NodeID: "1", NodePath: []string{"1"}},
			{Description: "递归进入左子树，访问节点 2", NodeID: "2", NodePath: []string{"1", "2"}},
			{Description: "递归进入左子树，访问节点 4", NodeID: "4", NodePath: []string{"1", "2", "4"}},
			{Description: "节点 4 无左子树，输出 4", NodeID: "4", NodePath: []string{"1", "2", "4"}},
			{Description: "回溯到节点 2，输出 2", NodeID: "2", NodePath: []string{"1", "2"}},
			{Description: "进入节点 5，输出 5", NodeID: "5", NodePath: []string{"1", "2", "5"}},
			{Description: "回溯到根节点 1，输出 1", NodeID: "1", NodePath: []string{"1"}},
			{Description: "进入右子树，访问节点 3，输出 3", NodeID: "3", NodePath: []string{"1", "3"}},
		},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, anim
}

func (c *ChatService) genDpResponse(req SendMessageRequest) (Message, AnimationData) {
	code := fmt.Sprintf("```%s\nfunc knapsack(weights []int, values []int, capacity int) int {\n    n := len(weights)\n    dp := make([][]int, n+1)\n    for i := range dp {\n        dp[i] = make([]int, capacity+1)\n    }\n    for i := 1; i <= n; i++ {\n        for j := 0; j <= capacity; j++ {\n            if weights[i-1] > j {\n                dp[i][j] = dp[i-1][j]\n            } else {\n                dp[i][j] = max(dp[i-1][j], dp[i-1][j-weights[i-1]]+values[i-1])\n            }\n        }\n    }\n    return dp[n][capacity]\n}\n```", req.Language)

	content := fmt.Sprintf(`## 题目分析
**0/1 背包问题**，经典动态规划，选择物品使总价值最大且不超过容量。

## 解题思路
dp[i][j] 表示前 i 个物品放入容量 j 的最大价值。
状态转移：dp[i][j] = max(dp[i-1][j], dp[i-1][j-w[i-1]] + v[i-1])
- 时间复杂度：O(n * capacity)
- 空间复杂度：O(n * capacity)

## 代码实现
%s`, code)

	// weights=[2,3,4], values=[3,4,5], capacity=5
	grid := [][]string{
		{"0", "0", "0", "0", "0", "0"},
		{"0", "0", "3", "3", "3", "3"},
		{"0", "0", "3", "4", "4", "7"},
		{"0", "0", "3", "4", "5", "7"},
	}

	anim := AnimationData{
		Type: "dptable",
		Table: &DpTableData{
			Rows:       4,
			Cols:       6,
			RowHeaders: []string{"0", "物品1(2kg/3¥)", "物品2(3kg/4¥)", "物品3(4kg/5¥)"},
			ColHeaders: []string{"0", "1", "2", "3", "4", "5"},
		},
		Steps: []AnimStep{
			{Description: "初始化 dp 表，dp[0][*] = 0", TableGrid: grid, Row: 0, Col: -1},
			{Description: "i=1, j=2: w[0]=2≤2, dp=3", TableGrid: grid, Row: 1, Col: 2, CellValue: "3"},
			{Description: "i=1: 容量<2 时不可选，dp=0", TableGrid: grid, Row: 1, Col: 1},
			{Description: "i=2, j=3: max(dp[1][3]=3, dp[1][0]+4=4)=4", TableGrid: grid, Row: 2, Col: 3, CellValue: "4"},
			{Description: "i=2, j=5: max(dp[1][5]=3, dp[1][2]+4=7)=7", TableGrid: grid, Row: 2, Col: 5, CellValue: "7"},
			{Description: "i=3, j=4: max(dp[2][4]=4, dp[2][0]+5=5)=5", TableGrid: grid, Row: 3, Col: 4, CellValue: "5"},
			{Description: "最终答案 dp[3][5] = 7", TableGrid: grid, Row: 3, Col: 5, CellValue: "7"},
		},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, anim
}

func (c *ChatService) genLinkedListResponse(req SendMessageRequest) (Message, AnimationData) {
	code := fmt.Sprintf("```%s\ntype ListNode struct {\n    Val  int\n    Next *ListNode\n}\n\nfunc reverseList(head *ListNode) *ListNode {\n    var prev *ListNode\n    curr := head\n    for curr != nil {\n        next := curr.Next\n        curr.Next = prev\n        prev = curr\n        curr = next\n    }\n    return prev\n}\n```", req.Language)

	content := fmt.Sprintf(`## 题目分析
**反转链表**，使用迭代法逐个翻转节点指针。

## 解题思路
用三个指针 prev/curr/next 遍历链表，每次将 curr.Next 指向 prev。
- 时间复杂度：O(n)
- 空间复杂度：O(1)

## 算法步骤
1. prev=nil, curr=1，准备反转
2. next=2，curr.Next=prev(nil)，prev=1，curr=2
3. next=3，curr.Next=prev(1)，prev=2，curr=3
4. next=4，curr.Next=prev(2)，prev=3，curr=4
5. curr=nil，反转完成，返回 prev=4

## 代码实现
%s`, code)

	nodes := []ListNodeData{
		{ID: "1", Value: "1", Next: "2", X: 30, Y: 50},
		{ID: "2", Value: "2", Next: "3", X: 160, Y: 50},
		{ID: "3", Value: "3", Next: "4", X: 290, Y: 50},
		{ID: "4", Value: "4", Next: "", X: 420, Y: 50},
	}

	anim := AnimationData{
		Type: "linkedlist",
		List: &LinkedListData{Nodes: nodes, Head: "1"},
		Steps: []AnimStep{
			{Description: "初始链表: 1→2→3→4→null", ListNodes: nodes, NodeID: "1", HighlightIdx: []int{0}},
			{Description: "prev=nil, curr=1, next=2", ListNodes: nodes, NodeID: "1", HighlightIdx: []int{0, 1}},
			{Description: "curr.Next=prev(nil), prev=1, curr=2", ListNodes: nodes, NodeID: "2", HighlightIdx: []int{1, 2}},
			{Description: "curr.Next=prev(1), prev=2, curr=3", ListNodes: nodes, NodeID: "3", HighlightIdx: []int{2, 3}},
			{Description: "curr.Next=prev(2), prev=3, curr=4", ListNodes: nodes, NodeID: "4", HighlightIdx: []int{3}},
			{Description: "curr=nil, 反转完成, head=prev=4", ListNodes: nodes, ResultIdx: []int{3}},
		},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, anim
}

func (c *ChatService) genSlidingWindowResponse(req SendMessageRequest) (Message, AnimationData) {
	values := []int{2, 3, 1, 2, 4, 3, 5}
	code := fmt.Sprintf("```%s\nfunc minSubArrayLen(target int, nums []int) int {\n    left := 0\n    sum := 0\n    minLen := math.MaxInt32\n    for right := 0; right < len(nums); right++ {\n        sum += nums[right]\n        for sum >= target {\n            minLen = min(minLen, right-left+1)\n            sum -= nums[left]\n            left++\n        }\n    }\n    return minLen\n}\n```", req.Language)

	content := fmt.Sprintf(`## 题目分析
**长度最小的子数组**，使用滑动窗口找到和 ≥ target 的最短连续子数组。

## 解题思路
右指针扩展窗口直到 sum ≥ target，然后左指针收缩窗口求最小长度。
- 时间复杂度：O(n)
- 空间复杂度：O(1)

## 算法步骤
1. R=0, sum=2 < 7，继续扩展
2. R=1, sum=5 < 7，继续扩展
3. R=2, sum=6 < 7，继续扩展
4. R=3, sum=8 ≥ 7，记录 len=4，收缩 L
5. L=1, sum=6 < 7，继续扩展 R
6. R=4, sum=10 ≥ 7，记录 len=4，收缩 L
7. L=2, sum=7 ≥ 7，记录 len=3，收缩 → 最终 minLen=2

## 代码实现
%s`, code)

	anim := AnimationData{
		Type: "slidingwindow",
		Array: &ArrayData{Values: values},
		Steps: []AnimStep{
			{Description: "初始状态：左边界=0", Values: values, WindowStart: 0, WindowEnd: -1},
			{Description: "R=0, sum=2 < 7", Values: values, WindowStart: 0, WindowEnd: 0, HighlightIdx: []int{0}},
			{Description: "R=1, sum=5 < 7", Values: values, WindowStart: 0, WindowEnd: 1, HighlightIdx: []int{0, 1}},
			{Description: "R=2, sum=6 < 7", Values: values, WindowStart: 0, WindowEnd: 2, HighlightIdx: []int{0, 1, 2}},
			{Description: "R=3, sum=8 ≥ 7, minLen=4", Values: values, WindowStart: 0, WindowEnd: 3, CompareIdx: []int{0, 1, 2, 3}},
			{Description: "收缩 L=1, sum=sum-2=6", Values: values, WindowStart: 1, WindowEnd: 3, HighlightIdx: []int{1, 2, 3}},
			{Description: "R=4, sum=10 ≥ 7, minLen=4", Values: values, WindowStart: 1, WindowEnd: 4, CompareIdx: []int{1, 2, 3, 4}},
			{Description: "收缩 L=2, sum=8 → len=3", Values: values, WindowStart: 2, WindowEnd: 4, CompareIdx: []int{2, 3, 4}},
			{Description: "最终 minLen=2 (子数组[4,3])", Values: values, ResultIdx: []int{4, 5}},
		},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, anim
}

func (c *ChatService) genSortingResponse(req SendMessageRequest) (Message, AnimationData) {
	values := []int{5, 3, 8, 4, 2, 7, 1, 6}
	code := fmt.Sprintf("```%s\nfunc quickSort(nums []int, low, high int) {\n    if low >= high { return }\n    pivot := partition(nums, low, high)\n    quickSort(nums, low, pivot-1)\n    quickSort(nums, pivot+1, high)\n}\n\nfunc partition(nums []int, low, high int) int {\n    pivot := nums[high]\n    i := low\n    for j := low; j < high; j++ {\n        if nums[j] < pivot {\n            nums[i], nums[j] = nums[j], nums[i]\n            i++\n        }\n    }\n    nums[i], nums[high] = nums[high], nums[i]\n    return i\n}\n```", req.Language)

	content := fmt.Sprintf(`## 题目分析
**快速排序**，基于分治思想，选取 pivot 将数组划分为两个子数组递归排序。

## 解题思路
选取最后一个元素为 pivot，将小于 pivot 的元素交换到左侧，大于的留在右侧。
- 平均时间复杂度：O(n log n)
- 空间复杂度：O(log n)

## 代码实现
%s`, code)

	anim := AnimationData{
		Type: "sorting",
		Array: &ArrayData{Values: values},
		Steps: []AnimStep{
			{Description: "初始数组", Values: values},
			{Description: "pivot=6, 比较 nums[0]=5 < 6, swap nums[0]↔nums[0]", Values: values, PivotIdx: 7, HighlightIdx: []int{0}},
			{Description: "pivot=6, 比较 nums[1]=3 < 6, swap nums[1]↔nums[1]", Values: values, PivotIdx: 7, HighlightIdx: []int{1}},
			{Description: "pivot=6, 比较 nums[2]=8 > 6, 不交换", Values: values, PivotIdx: 7, CompareIdx: []int{2}},
			{Description: "pivot=6, 比较 nums[3]=4 < 6, swap nums[3]↔nums[2] → [5,3,4,8,2,7,1,6]", Values: []int{5, 3, 4, 8, 2, 7, 1, 6}, PivotIdx: 7, SwapIdx: []int{2, 3}},
			{Description: "pivot=6, 比较 nums[4]=2 < 6, swap → [5,3,4,2,8,7,1,6]", Values: []int{5, 3, 4, 2, 8, 7, 1, 6}, PivotIdx: 7, SwapIdx: []int{3, 4}},
			{Description: "pivot=6, 比较 nums[5]=7 > 6, 不交换", Values: []int{5, 3, 4, 2, 8, 7, 1, 6}, PivotIdx: 7, CompareIdx: []int{5}},
			{Description: "pivot=6, 比较 nums[6]=1 < 6, swap → [5,3,4,2,1,7,8,6]", Values: []int{5, 3, 4, 2, 1, 7, 8, 6}, PivotIdx: 7, SwapIdx: []int{5, 6}},
			{Description: "pivot归位, swap nums[6]↔pivot → [5,3,4,2,1,6,8,7]", Values: []int{5, 3, 4, 2, 1, 6, 8, 7}, ResultIdx: []int{5}},
		},
	}

	return Message{Role: RoleAssistant, Content: content, Time: time.Now()}, anim
}

// ---- Helpers ----

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
