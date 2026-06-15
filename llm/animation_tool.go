package llm

import (
	"context"
	"encoding/json"

	"github.com/cloudwego/eino/components/tool"
	"github.com/cloudwego/eino/schema"
)

type AnimationTool struct {
	model func(ctx context.Context, problem, language string) (string, error)
}

type animParams struct {
	Problem  string `json:"problem"`
	Language string `json:"language"`
	Elements string `json:"elements_desc"`
	Steps    string `json:"algorithm_steps"`
}

func NewAnimationTool(modelFn func(ctx context.Context, problem, language string) (string, error)) *AnimationTool {
	return &AnimationTool{model: modelFn}
}

func (t *AnimationTool) Info(ctx context.Context) (*schema.ToolInfo, error) {
	return &schema.ToolInfo{
		Name: "generate_animation",
		Desc: `生成算法步骤的 SVG 动画 JSON。当你完成题解后调用此工具，传入题目描述、元素描述、算法步骤来生成可视化动画。

参数说明：
- problem: 原始题目描述
- language: 编程语言
- elements_desc: 需要展示的元素描述（如"7个数组元素"、"3个链表节点"、"5个树节点"等）
- algorithm_steps: 算法步骤的文字描述，每一步用分号分隔`,
		ParamsOneOf: schema.NewParamsOneOfByParams(map[string]*schema.ParameterInfo{
			"problem": {
				Type:     schema.String,
				Desc:     "原始题目描述",
				Required: true,
			},
			"language": {
				Type:     schema.String,
				Desc:     "编程语言",
				Required: true,
			},
			"elements_desc": {
				Type:     schema.String,
				Desc:     "需要展示的元素描述",
				Required: true,
			},
			"algorithm_steps": {
				Type:     schema.String,
				Desc:     "算法步骤描述，用分号分隔",
				Required: true,
			},
		}),
	}, nil
}

func (t *AnimationTool) InvokableRun(ctx context.Context, argumentsInJSON string, opts ...tool.Option) (string, error) {
	var params animParams
	if err := json.Unmarshal([]byte(argumentsInJSON), &params); err != nil {
		return "", err
	}

	prompt := `根据以下信息生成算法动画 JSON，输出符合动画规范。

## 题目
` + params.Problem + `

## 语言
` + params.Language + `

## 元素描述
` + params.Elements + `

## 算法步骤
` + params.Steps + `

请直接输出 JSON（不要包含 ---ANIM--- 或 markdown），格式如下：
{
  "svgW": 数字,
  "svgH": 数字,
  "elements": [...],
  "frames": [...]
}`

	result, err := t.model(ctx, prompt, params.Language)
	if err != nil {
		return "", err
	}
	return result, nil
}
