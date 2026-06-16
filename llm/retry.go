package llm

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strings"
)

const MaxRetryRounds = 3
const boundsMargin = 8.0

func (c *Client) GenerateWithRetry(ctx context.Context, problem, language, history string) (content string, anims []AnimBlock, err error) {
	content, err = c.Generate(ctx, problem, language, history)
	if err != nil {
		return "", nil, err
	}

	parsed := ParseResponse(content)
	anims = buildAnimBlocks(parsed)
	outOfBounds := checkBoundsAll(anims)

	if len(anims) > 0 && len(outOfBounds) == 0 {
		log.Printf("[LLM] first attempt: %d valid animations, all in bounds", len(anims))
		return content, anims, nil
	}

	var correctionMsg string
	if len(anims) == 0 {
		correctionMsg = "动画 JSON 格式有问题（无法解析或缺少 elements/frames 字段）。\n\n"
	} else {
		correctionMsg = fmt.Sprintf("动画元素超出面板边界！%s。\n\n", strings.Join(outOfBounds, "; "))
	}

	correctionHistory := buildCorrectionHistory(history, content)

	for round := 1; round <= MaxRetryRounds; round++ {
		correctionPrompt := fmt.Sprintf(
			"之前的输出中%s\n请重新生成动画 JSON。注意：\n"+
				"1. 用 ---ANIM--- 分隔多个动画\n"+
				"2. 每个动画标签行后紧跟完整的 JSON 对象\n"+
				"3. JSON 必须包含 svgW, svgH, elements, frames 四个字段\n"+
				"4. 所有元素的坐标(x/y/x2/y2)必须在面板范围内（0 到 svgW/svgH）\n"+
				"5. 元素间留足够间距(gap=52~60)，不要挤在边缘\n"+
				"6. 不要用 markdown 代码块包裹 JSON\n\n"+
				"题目: %s\n语言: %s", correctionMsg, problem, language)

		animContent, retryErr := c.Generate(ctx, correctionPrompt, language, correctionHistory)
		if retryErr != nil {
			log.Printf("[LLM] retry %d: generate error: %v", round, retryErr)
			continue
		}

		animContent = stripMarkdown(animContent)

		retryParsed := ParseResponse(animContent)
		retryAnims := buildAnimBlocks(retryParsed)
		if len(retryAnims) == 0 {
			log.Printf("[LLM] retry %d: still no valid animations, raw=[%s]", round, truncateStr(animContent, 200))
			correctionMsg = "动画 JSON 格式有问题（无法解析或缺少 elements/frames 字段）。"
			continue
		}

		retryOOB := checkBoundsAll(retryAnims)
		if len(retryOOB) > 0 {
			log.Printf("[LLM] retry %d: %d anims but bounds violations: %v", round, len(retryAnims), retryOOB)
			correctionMsg = fmt.Sprintf("动画元素超出面板边界！%s。坐标范围必须在 0~svgW 和 0~svgH 之内。", strings.Join(retryOOB, "; "))
			continue
		}

		log.Printf("[LLM] retry %d: got %d valid animations, all in bounds", round, len(retryAnims))
		return content, retryAnims, nil
	}

	log.Printf("[LLM] all %d retries exhausted, returning content without animations", MaxRetryRounds)
	return content, nil, nil
}

type AnimBlock struct {
	Label string
	JSON  string
}

type rawElement struct {
	ID   string  `json:"id"`
	Kind string  `json:"kind"`
	X    float64 `json:"x"`
	Y    float64 `json:"y"`
	W    float64 `json:"w"`
	H    float64 `json:"h"`
	R    float64 `json:"r"`
	X2   float64 `json:"x2"`
	Y2   float64 `json:"y2"`
}

type rawAnim struct {
	SVGW float64       `json:"svgW"`
	SVGH float64       `json:"svgH"`
	Els  []rawElement  `json:"elements"`
	Frs  []interface{} `json:"frames"`
}

func checkBoundsAll(blocks []AnimBlock) []string {
	var violations []string
	for bi, b := range blocks {
		v := checkBounds(b.JSON)
		for _, msg := range v {
			violations = append(violations, fmt.Sprintf("[动画%d] %s", bi+1, msg))
		}
	}
	return violations
}

func checkBounds(raw string) []string {
	var a rawAnim
	if err := json.Unmarshal([]byte(raw), &a); err != nil {
		return nil
	}
	if a.SVGW <= 0 || a.SVGH <= 0 {
		return []string{fmt.Sprintf("svgW=%.0f svgH=%.0f 无效", a.SVGW, a.SVGH)}
	}

	var violations []string
	for _, e := range a.Els {
		switch e.Kind {
		case "rect":
			if e.X < boundsMargin {
				violations = append(violations, fmt.Sprintf("%s x=%.0f 太靠左(需≥%.0f)", e.ID, e.X, boundsMargin))
			}
			if e.Y < boundsMargin {
				violations = append(violations, fmt.Sprintf("%s y=%.0f 太靠上(需≥%.0f)", e.ID, e.Y, boundsMargin))
			}
			if e.W > 0 && e.X+e.W > a.SVGW-boundsMargin {
				violations = append(violations, fmt.Sprintf("%s x+w=%.0f 超出右边界 svgW=%.0f", e.ID, e.X+e.W, a.SVGW))
			}
			if e.H > 0 && e.Y+e.H > a.SVGH-boundsMargin {
				violations = append(violations, fmt.Sprintf("%s y+h=%.0f 超出下边界 svgH=%.0f", e.ID, e.Y+e.H, a.SVGH))
			}
		case "circle":
			r := e.R
			if r <= 0 {
				r = 22
			}
			if e.X-r < boundsMargin {
				violations = append(violations, fmt.Sprintf("%s cx-r=%.0f 超出左边界", e.ID, e.X-r))
			}
			if e.Y-r < boundsMargin {
				violations = append(violations, fmt.Sprintf("%s cy-r=%.0f 超出上边界", e.ID, e.Y-r))
			}
			if e.X+r > a.SVGW-boundsMargin {
				violations = append(violations, fmt.Sprintf("%s cx+r=%.0f 超出右边界 svgW=%.0f", e.ID, e.X+r, a.SVGW))
			}
			if e.Y+r > a.SVGH-boundsMargin {
				violations = append(violations, fmt.Sprintf("%s cy+r=%.0f 超出下边界 svgH=%.0f", e.ID, e.Y+r, a.SVGH))
			}
		case "line":
			if e.X < -boundsMargin || e.X > a.SVGW+boundsMargin {
				violations = append(violations, fmt.Sprintf("%s x1=%.0f 超出横向范围 [0,%.0f]", e.ID, e.X, a.SVGW))
			}
			if e.X2 < -boundsMargin || e.X2 > a.SVGW+boundsMargin {
				violations = append(violations, fmt.Sprintf("%s x2=%.0f 超出横向范围 [0,%.0f]", e.ID, e.X2, a.SVGW))
			}
			if e.Y < -boundsMargin || e.Y > a.SVGH+boundsMargin {
				violations = append(violations, fmt.Sprintf("%s y1=%.0f 超出纵向范围 [0,%.0f]", e.ID, e.Y, a.SVGH))
			}
			if e.Y2 < -boundsMargin || e.Y2 > a.SVGH+boundsMargin {
				violations = append(violations, fmt.Sprintf("%s y2=%.0f 超出纵向范围 [0,%.0f]", e.ID, e.Y2, a.SVGH))
			}
		case "label":
			if e.X < -boundsMargin || e.X > a.SVGW+boundsMargin {
				violations = append(violations, fmt.Sprintf("%s x=%.0f 超出横向范围 svgW=%.0f", e.ID, e.X, a.SVGW))
			}
			if e.Y < -boundsMargin || e.Y > a.SVGH+boundsMargin {
				violations = append(violations, fmt.Sprintf("%s y=%.0f 超出纵向范围 svgH=%.0f", e.ID, e.Y, a.SVGH))
			}
		}
	}

	if len(violations) > 5 {
		violations = append(violations[:5], fmt.Sprintf("... 还有 %d 个越界元素", len(violations)-5))
	}

	return violations
}

func buildAnimBlocks(parsed AnimResponse) []AnimBlock {
	result := make([]AnimBlock, 0, len(parsed.AnimJSONs))
	for i, jsonStr := range parsed.AnimJSONs {
		if isValidAnimJSON(jsonStr) {
			label := ""
			if i < len(parsed.AnimLabel) {
				label = parsed.AnimLabel[i]
			}
			result = append(result, AnimBlock{Label: label, JSON: jsonStr})
		}
	}
	return result
}

func buildCorrectionHistory(originalHistory, fullContent string) string {
	contentPart := fullContent
	if idx := strings.Index(fullContent, "---ANIM---"); idx >= 0 {
		contentPart = strings.TrimSpace(fullContent[:idx])
	}
	if originalHistory == "" {
		return contentPart
	}
	return originalHistory + "\n\n" + contentPart
}

func stripMarkdown(s string) string {
	s = strings.TrimSpace(s)
	if strings.HasPrefix(s, "```json") {
		s = strings.TrimPrefix(s, "```json")
	} else if strings.HasPrefix(s, "```") {
		s = strings.TrimPrefix(s, "```")
	}
	if strings.HasSuffix(s, "```") {
		s = strings.TrimSuffix(s, "```")
	}
	return strings.TrimSpace(s)
}

func truncateStr(s string, maxLen int) string {
	rs := []rune(s)
	if len(rs) <= maxLen {
		return s
	}
	return string(rs[:maxLen]) + "..."
}
