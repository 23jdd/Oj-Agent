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
  "svgW": 数字（SVG 宽度，按元素数量估算，比如 7个数组元素约500，4个链表节点约400，5个树节点约560，DP表约400），
  "svgH": 数字（SVG 高度，数组约200，树约380，DP表约230），
  "elements": [
    {
      "id": "唯一ID（如cell_0, edge_1, ptr_l, head_label）",
      "kind": "rect|circle|line|label",
      "x": 数字, "y": 数字,
      "w": 数字（仅rect）, "h": 数字（仅rect）,
      "r": 数字（仅circle，建议22）,
      "x2": 数字（仅line终点X）, "y2": 数字（仅line终点Y）,
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

## 元素类型说明

- **rect**: 数组格、链表节点、DP表单元格。数组格通常 w=44~52, h=40~44, gap=52~56
  例: {"id":"cell_0","kind":"rect","x":30,"y":55,"w":48,"h":42,"text":"2","style":"normal","rx":6,"visible":true}
- **circle**: 树节点。r=22
  例: {"id":"n_1","kind":"circle","x":300,"y":50,"r":22,"text":"1","style":"normal","visible":true}
- **line**: 边、指针、窗口标记。需要 x2,y2 表示终点。arrow=true 时画箭头
  例: {"id":"edge_0","kind":"line","x":300,"y":72,"x2":170,"y2":138,"style":"#4b5563","visible":true}
- **label**: 纯文本标签
  例: {"id":"ptr_l","kind":"label","x":56,"y":34,"text":"▲ L","style":"dim"}

## 颜色风格（style 字段）
- "normal" = 灰色（未操作）
- "highlight" = 蓝色（当前焦点）
- "compare" = 黄色（比较中）
- "swap" = 红色（交换中）
- "result" = 绿色（最终结果）
- "pivot" = 紫色（基准/中枢）
- "dim" = 暗灰（已操作过/非活跃）
- "#3b82f6"等 = 直接指定颜色（仅用于 line 的 style）

## 帧（frames）规则
- 每帧只描述相对于上一帧的**变化量**（delta），but 每帧应该拥有完整的信息 
- delta 的 key 是元素 ID，value 是该元素变化的属性
- 不变的元素不用写
- 第一帧可以设置初始状态（各元素的style）
- 最后一帧标记 result 元素

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
		schema.UserMessage("题目: {{.problem}}\n语言: {{.language}}"),
	)
	if err := graph.AddChatTemplateNode("oj_template", ojTemplate); err != nil {
		return nil, err
	}

	generalTemplate := prompt.FromMessages(schema.GoTemplate,
		schema.SystemMessage(generalSystemPrompt),
		schema.UserMessage("{{.problem}}"),
	)
	if err := graph.AddChatTemplateNode("general_template", generalTemplate); err != nil {
		return nil, err
	}

	if err := graph.AddChatModelNode("model", cm); err != nil {
		return nil, err
	}

	graph.AddBranch(compose.START, compose.NewGraphBranch(
		func(ctx context.Context, input map[string]any) (string, error) {
			problem, _ := input["problem"].(string)
			if isOJProblem(problem) {
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

func (c *Client) Generate(ctx context.Context, problem string, language string, rules string) (string, error) {
	input := map[string]any{
		"problem":  problem,
		"language": language,
	}

	result, err := c.runnable.Invoke(ctx, input)
	if err != nil {
		log.Printf("[LLM] Generate error: %v", err)
		return "", err
	}

	return result.Content, nil
}

func (c *Client) Stream(ctx context.Context, problem string, language string) (*schema.StreamReader[*schema.Message], error) {
	input := map[string]any{
		"problem":  problem,
		"language": language,
	}
	return c.runnable.Stream(ctx, input)
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
		"题目", "代码实现", "时间复杂度", "空间复杂度",
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
