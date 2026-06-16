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

var ojSystemPrompt = `你是一个算法题解动画生成助手。根据用户输入的算法题目，生成详细的题解和分条件动画步骤。

重要：你必须为代码中的每种主要条件分支生成独立的动画，每个动画使用一个具体示例来演示该分支的执行过程。

规则：
1. 每个动画必须完全独立，拥有自己的 elements、frames、svgW、svgH，不得依赖或引用其他动画中的元素。
2. 每个动画必须使用一个独立的示例数据（如不同的数组、不同的输入值），该示例专门用于演示某一个条件分支。
3. 示例数据应覆盖代码的所有执行路径：if/else 分支、循环的不同迭代阶段、递归的递进/回溯、通配符的不同匹配情况等。
4. 如果一个条件有多个子情况（如 * 匹配0个/1个/多个字符），每个子情况用一个独立动画演示。

每个动画用 "---ANIM---" 分隔，并在该行后第一行给出动画标签（如"条件: arr[i]<pivot"、"示例: *匹配0个字符"、"主循环第1轮"、"递归归并"等）。

## 输出格式

你必须严格按以下格式输出：

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

### 第二部分：动画 JSON（可多个，每个覆盖一个条件分支）
---ANIM---
动画标签：主循环 / 条件：xxx
{...动画JSON...}
---ANIM---
动画标签：递归归并 / 另一条件
{...动画JSON...}

每个动画 JSON 对象包含以下字段：

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
      "visible": true,
      "showGrid": true/false（仅rect，DP表格/矩阵单元格）
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
- rect: 数组格/链表节点/DP单元格, w=44~60, h=40~44, gap=52~60, rx=6
- rect(DP表格): showGrid=true 画出网格背景，适合动态规划二维表
- circle: 树节点/图节点, r=22, gap=60~80
- line: 边/指针/数组箭头, 需要 x2,y2, arrow=true 画箭头, style 可用 #色值
- label: 纯文本标签（索引、变量名、注释等）

## 颜色（style）
- normal=灰(#374151), highlight=蓝(#3b82f6), compare=黄(#f59e0b)
- swap=红(#ef4444), result=绿(#10b981), pivot=紫(#8b5cf6), dim=暗灰(#1e293b)

## 帧规则
- 每帧只写变化量（delta），第一帧设初始状态
- 初始态用 highlight 标记当前关注元素
- 中间过程用 compare 标记比较、swap 标记交换、pivot 标记基准
- 最终态标记 result（已处理完的元素）
- dim 用于非活跃元素

## 示例1：反转链表（rect + line + label）
用户: 反转链表
---ANIM---
动画标签：主循环 反转1→2→3
{"svgW":380,"svgH":180,
"elements":[
  {"id":"n0","kind":"rect","x":30,"y":50,"w":52,"h":38,"text":"1","style":"normal","rx":6,"visible":true},
  {"id":"n1","kind":"rect","x":160,"y":50,"w":52,"h":38,"text":"2","style":"normal","rx":6,"visible":true},
  {"id":"n2","kind":"rect","x":290,"y":50,"w":52,"h":38,"text":"3","style":"normal","rx":6,"visible":true},
  {"id":"e0","kind":"line","x":82,"y":69,"x2":160,"y2":69,"style":"#4b5563","arrow":true,"visible":true},
  {"id":"e1","kind":"line","x":212,"y":69,"x2":290,"y2":69,"style":"#4b5563","arrow":true,"visible":true},
  {"id":"head","kind":"label","x":56,"y":34,"text":"head","style":"dim","visible":true}
],
"frames":[
  {"desc":"初始链表: 1→2→3, head指向1","delta":{"n0":{"style":"highlight"},"head":{"text":"head"}}},
  {"desc":"prev=nil, curr=1, next=2","delta":{}},
  {"desc":"1.Next→nil, prev=1, curr=2","delta":{"n0":{"style":"result"},"n1":{"style":"highlight"},"e0":{"style":"#6b7280"},"head":{"text":"prev"}}},
  {"desc":"2.Next→1, prev=2, curr=3","delta":{"n1":{"style":"result"},"n2":{"style":"highlight"}}},
  {"desc":"3.Next→2, prev=3, curr=nil 完成","delta":{"n2":{"style":"result"},"head":{"text":"new head","x":316,"y":34}}}
]}

## 示例2：二叉树中序遍历（circle + line + label）
用户: 二叉树中序遍历
---ANIM---
动画标签：中序遍历 左→根→右
{"svgW":440,"svgH":240,
"elements":[
  {"id":"t1","kind":"circle","x":220,"y":30,"r":22,"text":"1","style":"normal","visible":true},
  {"id":"t2","kind":"circle","x":100,"y":100,"r":22,"text":"2","style":"normal","visible":true},
  {"id":"t3","kind":"circle","x":340,"y":100,"r":22,"text":"3","style":"normal","visible":true},
  {"id":"t4","kind":"circle","x":40,"y":170,"r":22,"text":"4","style":"normal","visible":true},
  {"id":"t5","kind":"circle","x":160,"y":170,"r":22,"text":"5","style":"normal","visible":true},
  {"id":"t6","kind":"circle","x":280,"y":170,"r":22,"text":"6","style":"normal","visible":true},
  {"id":"e12","kind":"line","x":198,"y":52,"x2":122,"y2":78,"style":"#4b5563","visible":true},
  {"id":"e13","kind":"line","x":242,"y":52,"x2":318,"y2":78,"style":"#4b5563","visible":true},
  {"id":"e24","kind":"line","x":78,"y":122,"x2":62,"y2":148,"style":"#4b5563","visible":true},
  {"id":"e25","kind":"line","x":122,"y":122,"x2":138,"y2":148,"style":"#4b5563","visible":true},
  {"id":"e36","kind":"line","x":318,"y":122,"x2":302,"y2":148,"style":"#4b5563","visible":true},
  {"id":"res","kind":"label","x":220,"y":216,"text":"中序: ","style":"dim","visible":true}
],
"frames":[
  {"desc":"以1为根的中序: 先访问左子树(2)","delta":{"t1":{"style":"highlight"},"t2":{"style":"compare"}}},
  {"desc":"以2为根: 访问左子树(4)","delta":{"t4":{"style":"highlight"},"t2":{"style":"pivot"},"t1":{"style":"normal"}}},
  {"desc":"输出4, 回溯到2, 输出2","delta":{"t4":{"style":"result"},"t2":{"style":"result"},"res":{"text":"中序: 4,2"}}},
  {"desc":"以2为根: 访问右子树(5), 输出5","delta":{"t5":{"style":"highlight"},"res":{"text":"中序: 4,2,5"}}},
  {"desc":"回溯到1, 输出1, 访问右子树(3)","delta":{"t5":{"style":"result"},"t1":{"style":"result"},"t3":{"style":"highlight"},"res":{"text":"中序: 4,2,5,1"}}},
  {"desc":"以3为根: 访问左子树(6), 输出6","delta":{"t6":{"style":"highlight"},"res":{"text":"中序: 4,2,5,1,6"}}},
  {"desc":"输出3, 完成: 4,2,5,1,6,3","delta":{"t6":{"style":"result"},"t3":{"style":"result"},"res":{"text":"中序: 4,2,5,1,6,3"}}}
]}

## 示例3：快排partition — 两个动画覆盖两种条件
用户: 数组快排partition
---ANIM---
动画标签：条件: arr[i] < pivot (元素较小放左边)
{"svgW":440,"svgH":130,
"elements":[
  {"id":"a0","kind":"rect","x":10,"y":50,"w":48,"h":40,"text":"5","style":"normal","rx":6,"visible":true},
  {"id":"a1","kind":"rect","x":62,"y":50,"w":48,"h":40,"text":"3","style":"normal","rx":6,"visible":true},
  {"id":"a2","kind":"rect","x":114,"y":50,"w":48,"h":40,"text":"8","style":"normal","rx":6,"visible":true},
  {"id":"a3","kind":"rect","x":166,"y":50,"w":48,"h":40,"text":"1","style":"normal","rx":6,"visible":true},
  {"id":"piv","kind":"label","x":224,"y":42,"text":"pivot=5","style":"pivot","visible":true}
],
"frames":[
  {"desc":"3<5, swap放到左侧","delta":{"a1":{"style":"highlight"},"a0":{"style":"pivot"}}},
  {"desc":"1<5, swap放到左侧","delta":{"a1":{"style":"result"},"a3":{"style":"highlight"}}},
  {"desc":"<pivot的元素全部归位","delta":{"a3":{"style":"result"}}}
]}
---ANIM---
动画标签：条件: arr[i] >= pivot (元素较大跳过)
{"svgW":440,"svgH":130,
"elements":[
  {"id":"a0","kind":"rect","x":10,"y":50,"w":48,"h":40,"text":"5","style":"normal","rx":6,"visible":true},
  {"id":"a1","kind":"rect","x":62,"y":50,"w":48,"h":40,"text":"3","style":"normal","rx":6,"visible":true},
  {"id":"a2","kind":"rect","x":114,"y":50,"w":48,"h":40,"text":"8","style":"normal","rx":6,"visible":true},
  {"id":"piv","kind":"label","x":170,"y":42,"text":"pivot=5","style":"pivot","visible":true}
],
"frames":[
  {"desc":"8>=5, 跳过不移动","delta":{"a2":{"style":"compare"},"a0":{"style":"pivot"}}},
  {"desc":"跳过元素标记为dim","delta":{"a2":{"style":"dim"}}}
]}

## 示例4：通配符匹配 — 四个动画覆盖所有 * 和 . 条件
用户: 通配符匹配 s="aa", p="a*"
---ANIM---
动画标签：示例: *匹配1个字符 (s="aa", p="a*")
{"svgW":360,"svgH":140,
"elements":[
  {"id":"s0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"s1","kind":"rect","x":62,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"p0","kind":"rect","x":160,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"p1","kind":"rect","x":212,"y":50,"w":44,"h":40,"text":"*","style":"normal","rx":6,"visible":true},
  {"id":"ls","kind":"label","x":32,"y":36,"text":"s","style":"dim","visible":true},
  {"id":"lp","kind":"label","x":182,"y":36,"text":"p","style":"dim","visible":true},
  {"id":"res","kind":"label","x":280,"y":60,"text":"true","style":"result","visible":true}
],
"frames":[
  {"desc":"a匹配a, *匹配剩余的a","delta":{"s0":{"style":"highlight"},"p0":{"style":"highlight"}}},
  {"desc":"a匹配成功, *消耗1个字符","delta":{"s0":{"style":"result"},"p0":{"style":"result"},"s1":{"style":"highlight"},"p1":{"style":"highlight"}}},
  {"desc":"匹配完成 true","delta":{"s1":{"style":"result"},"p1":{"style":"result"},"res":{"text":"true"}}}
]}
---ANIM---
动画标签：示例: *匹配0个字符 (s="a", p="a*")
{"svgW":320,"svgH":140,
"elements":[
  {"id":"s0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"p0","kind":"rect","x":120,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"p1","kind":"rect","x":172,"y":50,"w":44,"h":40,"text":"*","style":"normal","rx":6,"visible":true},
  {"id":"ls","kind":"label","x":32,"y":36,"text":"s","style":"dim","visible":true},
  {"id":"lp","kind":"label","x":142,"y":36,"text":"p","style":"dim","visible":true},
  {"id":"res","kind":"label","x":240,"y":60,"text":"true","style":"result","visible":true}
],
"frames":[
  {"desc":"a匹配a, *匹配0个字符 直接跳过","delta":{"s0":{"style":"highlight"},"p0":{"style":"highlight"}}},
  {"desc":"a匹配成功, *不消耗字符","delta":{"s0":{"style":"result"},"p0":{"style":"result"},"p1":{"style":"highlight"}}},
  {"desc":"*跳过, 匹配完成 true","delta":{"p1":{"style":"result"},"res":{"text":"true"}}}
]}
---ANIM---
动画标签：示例: .匹配单个字符 (s="ab", p="a.")
{"svgW":320,"svgH":140,
"elements":[
  {"id":"s0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"s1","kind":"rect","x":62,"y":50,"w":44,"h":40,"text":"b","style":"normal","rx":6,"visible":true},
  {"id":"p0","kind":"rect","x":120,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"p1","kind":"rect","x":172,"y":50,"w":44,"h":40,"text":".","style":"normal","rx":6,"visible":true},
  {"id":"ls","kind":"label","x":32,"y":36,"text":"s","style":"dim","visible":true},
  {"id":"lp","kind":"label","x":142,"y":36,"text":"p","style":"dim","visible":true},
  {"id":"res","kind":"label","x":240,"y":60,"text":"true","style":"result","visible":true}
],
"frames":[
  {"desc":"a匹配a","delta":{"s0":{"style":"highlight"},"p0":{"style":"highlight"}}},
  {"desc":".匹配任意单字符(b)","delta":{"s0":{"style":"result"},"p0":{"style":"result"},"s1":{"style":"highlight"},"p1":{"style":"highlight"}}},
  {"desc":"匹配完成 true","delta":{"s1":{"style":"result"},"p1":{"style":"result"},"res":{"text":"true"}}}
]}
---ANIM---
动画标签：示例: 不匹配情况 (s="aa", p="b")
{"svgW":280,"svgH":140,
"elements":[
  {"id":"s0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"s1","kind":"rect","x":62,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true},
  {"id":"p0","kind":"rect","x":120,"y":50,"w":44,"h":40,"text":"b","style":"normal","rx":6,"visible":true},
  {"id":"ls","kind":"label","x":32,"y":36,"text":"s","style":"dim","visible":true},
  {"id":"lp","kind":"label","x":142,"y":36,"text":"p","style":"dim","visible":true},
  {"id":"res","kind":"label","x":190,"y":60,"text":"false","style":"swap","visible":true}
],
"frames":[
  {"desc":"a≠b, 直接不匹配","delta":{"s0":{"style":"compare"},"p0":{"style":"compare"}}},
  {"desc":"匹配失败 false","delta":{"s0":{"style":"swap"},"p0":{"style":"swap"},"res":{"text":"false"}}}
]}

请严格按此格式输出。至少生成一个动画（简单题目1个），条件分支多的题目请每个分支使用不同的示例数据生成独立动画。`

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
      "rx": 6, "arrow": true, "visible": true, "showGrid": true（仅DP表格rect）
    }
  ],
  "frames": [
    {"desc": "描述", "delta": {"id": {"style": "highlight"}}}
  ]
}

## 元素类型
- rect: 数组格/链表节点 w=48~60 h=40~44, rx=6; DP表格加 showGrid=true
- circle: 树节点 r=22, gap=60~80
- line: 边/指针/数组箭头, style可用#色值, arrow=true加箭头
- label: 文本标签, 放上方或下方

## 样式
normal=灰 highlight=蓝 compare=黄 swap=红 result=绿 pivot=紫 dim=暗灰

## 帧规则
- 每帧只写delta变化量，第一帧初始化当前关注元素
- 初始标highlight，比较标compare，交换标swap，基准标pivot
- 处理完毕标result，非活跃标dim

## 示例1：数组排序partition（compare+swap+pivot+result）
{"svgW":440,"svgH":130,
"elements":[
  {"id":"a0","kind":"rect","x":10,"y":50,"w":48,"h":40,"text":"5","style":"normal","rx":6,"visible":true},
  {"id":"a1","kind":"rect","x":62,"y":50,"w":48,"h":40,"text":"3","style":"normal","rx":6,"visible":true},
  {"id":"a2","kind":"rect","x":114,"y":50,"w":48,"h":40,"text":"8","style":"normal","rx":6,"visible":true},
  {"id":"a3","kind":"rect","x":166,"y":50,"w":48,"h":40,"text":"1","style":"normal","rx":6,"visible":true},
  {"id":"piv","kind":"label","x":224,"y":42,"text":"pivot=5","style":"pivot","visible":true}
],
"frames":[
  {"desc":"初始数组, pivot=5","delta":{"a0":{"style":"pivot"}}},
  {"desc":"3<5 放左边","delta":{"a1":{"style":"compare"}}},
  {"desc":"8>5 跳过","delta":{"a1":{"style":"result"},"a2":{"style":"compare"}}},
  {"desc":"1<5 放左边","delta":{"a2":{"style":"dim"},"a3":{"style":"compare"}}},
  {"desc":"归位完成","delta":{"a3":{"style":"result"},"a0":{"style":"result"}}}
]}

## 示例2：二叉树遍历（circle+line+label）
{"svgW":320,"svgH":180,
"elements":[
  {"id":"n1","kind":"circle","x":160,"y":20,"r":22,"text":"1","style":"normal","visible":true},
  {"id":"n2","kind":"circle","x":60,"y":90,"r":22,"text":"2","style":"normal","visible":true},
  {"id":"n3","kind":"circle","x":260,"y":90,"r":22,"text":"3","style":"normal","visible":true},
  {"id":"e12","kind":"line","x":138,"y":42,"x2":82,"y2":68,"style":"#4b5563","visible":true},
  {"id":"e13","kind":"line","x":182,"y":42,"x2":238,"y2":68,"style":"#4b5563","visible":true},
  {"id":"res","kind":"label","x":160,"y":160,"text":"输出: ","style":"dim","visible":true}
],
"frames":[
  {"desc":"访问根节点1","delta":{"n1":{"style":"highlight"}}},
  {"desc":"访问左子树2","delta":{"n2":{"style":"highlight"},"n1":{"style":"compare"}}},
  {"desc":"输出2","delta":{"n2":{"style":"result"},"res":{"text":"输出: 2"}}},
  {"desc":"回溯到1, 输出1","delta":{"n1":{"style":"result"},"res":{"text":"输出: 2,1"}}},
  {"desc":"访问右子树3, 输出3","delta":{"n3":{"style":"highlight"},"res":{"text":"输出: 2,1,3"}}},
  {"desc":"完成","delta":{"n3":{"style":"result"}}}
]}

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
