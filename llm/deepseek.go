package llm

import (
	"context"
	"log"
	"os"
	"strings"

	"github.com/cloudwego/eino-ext/components/model/deepseek"
	"github.com/cloudwego/eino/components/prompt"
	"github.com/cloudwego/eino/compose"
	"github.com/cloudwego/eino/schema"
	"github.com/joho/godotenv"
)

var ojSystemPrompt = `你是一个算法题解动画生成助手。根据用户输入的算法题目，生成详细的题解和动画步骤。

## 输出格式

你必须严格按以下两部分输出，用 "---ANIM---" 分隔：

### 第一部分：Markdown 题解
使用标准 Markdown，结构如下：

## 题目分析
（题目类型、数据结构、难度）

## 解题思路
- 时间复杂度：O(?)
- 空间复杂度：O(?)

## 算法步骤
1. 步骤1描述
2. 步骤2描述
...

## 代码实现
` + "```{{.language}}" + `
完整可运行代码
` + "```" + `

### 第二部分：动画 JSON
输出一个 JSON 对象，包含以下字段：

{
  "svgW": 数字（SVG 宽度，按元素数量估算），
  "svgH": 数字（SVG 高度），
  "elements": [
    {
      "id": "唯一ID",
      "kind": "rect|circle|line|label",
      "x": 数字, "y": 数字,
      "w": 数字（仅rect）, "h": 数字（仅rect）,
      "r": 数字（仅circle，建议22）,
      "x2": 数字（仅line）, "y2": 数字（仅line）,
      "text": "显示文字",
      "style": "normal|highlight|compare|swap|result|pivot|dim",
      "rx": 数字（rect圆角，建议6）,
      "arrow": true/false（line是否有箭头）,
      "visible": true
    }
  ],
  "frames": [
    {
      "desc": "本步骤的文字描述",
      "delta": {
        "element_id": {"style": "highlight", "text": "新文字"},
        "element_id": {"x": 新X坐标}
      }
    }
  ]
}

## 元素类型
- rect: 数组格/链表节点/DP单元格, w=44~52, h=40~44, gap=52~56, rx=6
- circle: 树节点, r=22
- line: 边/指针, 需要 x2,y2, arrow=true 画箭头, style 可用 #色值
- label: 纯文本标签

## 颜色（style）
- normal=灰, highlight=蓝, compare=黄, swap=红, result=绿, pivot=紫, dim=暗灰

## 帧规则
- 每帧只写变化量（delta），第一帧设初始状态，最后一帧标记result

## 示例

用户: 反转链表

---ANIM---
{
  "svgW": 380, "svgH": 180,
  "elements": [
    {"id":"n0","kind":"rect","x":30,"y":50,"w":52,"h":38,"text":"1","style":"normal","rx":6,"visible":true},
    {"id":"n1","kind":"rect","x":160,"y":50,"w":52,"h":38,"text":"2","style":"normal","rx":6,"visible":true},
    {"id":"n2","kind":"rect","x":290,"y":50,"w":52,"h":38,"text":"3","style":"normal","rx":6,"visible":true},
    {"id":"e0","kind":"line","x":82,"y":69,"x2":160,"y2":69,"style":"#4b5563","arrow":true,"visible":true},
    {"id":"e1","kind":"line","x":212,"y":69,"x2":290,"y2":69,"style":"#4b5563","arrow":true,"visible":true},
    {"id":"head","kind":"label","x":56,"y":34,"text":"head","style":"dim"}
  ],
  "frames": [
    {"desc":"初始链表: 1→2→3","delta":{"n0":{"style":"highlight"},"head":{"text":"head"}}},
    {"desc":"prev=nil, curr=1, next=2","delta":{"n0":{"style":"highlight"},"n1":{"style":"highlight"}}},
    {"desc":"1.Next→nil, prev=1, curr=2","delta":{"n0":{"style":"result"},"n1":{"style":"highlight"},"e0":{"style":"#6b7280"}}},
    {"desc":"2.Next→1, prev=2, curr=3","delta":{"n1":{"style":"result"},"n2":{"style":"highlight"}}},
    {"desc":"3.Next→2, prev=3, curr=nil 完成","delta":{"n2":{"style":"result"},"head":{"text":"new head","x":316,"y":34}}}
  ]
}

请严格按此格式输出。`

var generalSystemPrompt = `你是一个有用的AI助手。请用中文回答用户的问题。回答应简洁、准确、有帮助。`

var animGeneratePrompt = `你是一个算法动画 JSON 生成器。根据输入的信息，生成符合规范的动画 JSON。

输出必须是纯 JSON 对象（不含 markdown 标记），格式如下：

{
  "svgW": 数字,
  "svgH": 数字,
  "elements": [
    {
      "id": "唯一ID",
      "kind": "rect|circle|line|label",
      "x": 数字, "y": 数字,
      "w": 数字（rect）, "h": 数字（rect）,
      "r": 数字（circle）, "x2": 数字（line）, "y2": 数字（line）,
      "text": "文字",
      "style": "normal|highlight|compare|swap|result|pivot|dim",
      "rx": 6, "arrow": true, "visible": true
    }
  ],
  "frames": [
    {"desc": "描述", "delta": {"id": {"style": "highlight"}}}
  ]
}

样式: normal=灰, highlight=蓝, compare=黄, swap=红, result=绿, pivot=紫, dim=暗灰
rect=数组格(w44-52,h40-44,rx6), circle=树节点(r22), line=指针/边, label=标签

直接输出JSON。`

type Client struct {
	model    *deepseek.ChatModel
	config   *Config
	runnable compose.Runnable[map[string]any, *schema.Message]
}

type Config struct {
	APIKey  string
	BaseURL string
	Model   string
}

func NewClient(cfg *Config) (*Client, error) {
	_ = godotenv.Load(".env", ".env.local")

	if cfg == nil {
		cfg = &Config{
			APIKey:  os.Getenv("DEEPSEEK_API_KEY"),
			BaseURL: os.Getenv("DEEPSEEK_BASE_URL"),
			Model:   os.Getenv("DEEPSEEK_MODEL"),
		}
	}
	if cfg.APIKey == "" {
		cfg.APIKey = os.Getenv("OPENAI_API_KEY")
	}
	if cfg.APIKey == "" || isPlaceholderKey(cfg.APIKey) {
		return nil, nil
	}
	if cfg.BaseURL == "" {
		cfg.BaseURL = os.Getenv("OPENAI_BASE_URL")
	}
	if cfg.BaseURL == "" {
		cfg.BaseURL = "https://api.deepseek.com"
	}
	if cfg.Model == "" {
		cfg.Model = os.Getenv("LLM_MODEL")
	}
	if cfg.Model == "" {
		cfg.Model = "deepseek-chat"
	}

	cm, err := deepseek.NewChatModel(context.Background(), &deepseek.ChatModelConfig{
		APIKey:  cfg.APIKey,
		BaseURL: cfg.BaseURL,
		Model:   cfg.Model,
	})
	if err != nil {
		return nil, err
	}

	graph := compose.NewGraph[map[string]any, *schema.Message]()

	ojTemplate := prompt.FromMessages(schema.GoTemplate,
		schema.SystemMessage(ojSystemPrompt),
		schema.UserMessage("题目: {{.problem}}\n语言: {{.language}}{{if .history}}\n\n## 对话历史\n{{.history}}{{end}}"),
	)
	if err := graph.AddChatTemplateNode("oj_template", ojTemplate); err != nil {
		return nil, err
	}

	generalTemplate := prompt.FromMessages(schema.GoTemplate,
		schema.SystemMessage(generalSystemPrompt),
		schema.UserMessage("{{if .history}}## 对话历史\n{{.history}}\n\n{{end}}{{.problem}}"),
	)
	if err := graph.AddChatTemplateNode("general_template", generalTemplate); err != nil {
		return nil, err
	}

	if err := graph.AddChatModelNode("model", cm); err != nil {
		return nil, err
	}

	graph.AddBranch(compose.START, compose.NewGraphBranch(
		func(ctx context.Context, input map[string]any) (string, error) {
			if isOJ, ok := input["isOJ"].(bool); ok && isOJ {
				return "oj_template", nil
			}
			return "general_template", nil
		},
		map[string]bool{
			"oj_template":      true,
			"general_template": true,
		},
	))

	graph.AddEdge("oj_template", "model")
	graph.AddEdge("general_template", "model")
	graph.AddEdge("model", compose.END)

	runnable, err := graph.Compile(context.Background())
	if err != nil {
		return nil, err
	}

	return &Client{model: cm, config: cfg, runnable: runnable}, nil
}

func (c *Client) Available() bool {
	return c != nil && c.model != nil && c.config.APIKey != ""
}

func (c *Client) Model() string {
	if c != nil && c.config != nil {
		return c.config.Model
	}
	return ""
}

func (c *Client) Generate(ctx context.Context, problem string, language string, history string) (string, error) {
	input := map[string]any{
		"problem":  problem,
		"language": language,
		"history":  history,
		"isOJ":     c.judge(ctx, problem),
	}

	result, err := c.runnable.Invoke(ctx, input)
	if err != nil {
		log.Printf("[LLM] Generate error: %v", err)
		return "", err
	}

	return result.Content, nil
}

func (c *Client) Stream(ctx context.Context, problem string, language string, history string) (*schema.StreamReader[*schema.Message], error) {
	input := map[string]any{
		"problem":  problem,
		"language": language,
		"history":  history,
		"isOJ":     c.judge(ctx, problem),
	}
	return c.runnable.Stream(ctx, input)
}

var judgeSystemPrompt = `你是一个分类器。判断用户输入是否为算法/编程/数据结构题目。
如果输入包含算法题、编程题、LeetCode风格问题、或需要代码实现的问题，回答 "true"。
如果输入是普通闲聊、知识问答、或非编程问题，回答 "false"。
只回答一个单词：true 或 false。`

func (c *Client) judge(ctx context.Context, problem string) bool {
	msgs := []*schema.Message{
		schema.SystemMessage(judgeSystemPrompt),
		schema.UserMessage(problem),
	}

	result, err := c.model.Generate(ctx, msgs)
	if err != nil {
		log.Printf("[LLM] judge error, fallback to keyword: %v", err)
		return isOJProblem(problem)
	}

	content := strings.TrimSpace(strings.ToLower(result.Content))
	log.Printf("[LLM] judge result: %q -> %v", result.Content, strings.Contains(content, "true"))
	return strings.Contains(content, "true")
}

func isOJProblem(problem string) bool {
	p := strings.ToLower(problem)
	ojKeywords := []string{
		"算法", "题解", "时间复杂度", "空间复杂度",
		"数组", "链表", "树", "排序", "查找", "遍历",
		"dp", "动态规划", "递归", "二叉树", "图", "堆", "栈", "队列",
		"哈希", "双指针", "滑动窗口", "回溯", "贪心", "分治", "二分",
		"前缀和", "并查集", "字典树", "拓扑", "最短路", "最小生成树",
		"two sum", "reverse", "traversal", "sort", "search",
		"dynamic programming", "binary search", "bfs", "dfs",
		"leetcode", "力扣", "牛客", "acm", "ac ", "oj",
		"题目", "代码实现",
		"反转链表", "合并", "子集", "排列", "组合",
		"背包", "快排", "快速排序", "归并", "冒泡",
		"最长", "最大", "最小", "路径", "深度", "广度",
		"中序遍历", "前序遍历", "后序遍历", "层序遍历",
	}
	for _, kw := range ojKeywords {
		if strings.Contains(p, kw) {
			return true
		}
	}
	return false
}

var _ = strings.TrimSpace

func isPlaceholderKey(key string) bool {
	k := strings.ToLower(key)
	return strings.Contains(k, "your-") ||
		strings.Contains(k, "xxxx") ||
		strings.Contains(k, "placeholder") ||
		strings.Contains(k, "sk-xxx")
}
