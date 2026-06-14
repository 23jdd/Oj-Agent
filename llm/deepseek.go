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

var systemPrompt = `你是一个算法题解动画生成助手。根据用户输入的算法题目，生成详细的题解和动画步骤。

## 输出规则
1. **题目分析**：题目类型、数据结构、难度
2. **解题思路**：核心思路 + 时间/空间复杂度
3. **算法步骤**：拆分为5-10个关键步骤
4. **代码实现**：完整可运行代码（用 {language} 语言）

## 动画颜色
- blue = 当前关注元素  |  yellow = 比较中  |  red = 交换中
- green = 结果/已完成  |  purple = pivot/基线

## 输出格式（Markdown）
## 题目分析
（分析内容）

## 解题思路
- 时间复杂度：O(?)
- 空间复杂度：O(?)

## 算法步骤
（5-10步）

## 代码实现
` + "```{language}" + `
完整代码
` + "```" + `

请保持回答专业、准确。`

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

	template := prompt.FromMessages(schema.FString,
		schema.SystemMessage(systemPrompt),
		schema.UserMessage("题目: {problem}\n语言: {language}"),
	)
	if err := graph.AddChatTemplateNode("template", template); err != nil {
		return nil, err
	}

	if err := graph.AddChatModelNode("model", cm); err != nil {
		return nil, err
	}

	graph.AddEdge(compose.START, "template")
	graph.AddEdge("template", "model")
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

var _ = strings.TrimSpace

func isPlaceholderKey(key string) bool {
	k := strings.ToLower(key)
	return strings.Contains(k, "your-") ||
		strings.Contains(k, "xxxx") ||
		strings.Contains(k, "placeholder") ||
		strings.Contains(k, "sk-xxx")
}
