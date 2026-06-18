package llm

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strings"
)

const MaxRetryRounds = 10
const boundsMargin = 8.0

func (c *Client) GenerateWithRetry(ctx context.Context, problem, language, history string) (content string, anims []AnimBlock, err error) {
	content, err = c.Generate(ctx, problem, language, history)
	if err != nil {
		return "", nil, err
	}

	parsed := ParseResponse(content)
	anims = buildAnimBlocks(parsed)
	outOfBounds := checkBoundsAll(anims)
	deltaErrors := checkDeltaReferences(anims)

	if len(anims) > 0 && len(outOfBounds) == 0 && len(deltaErrors) == 0 {
		log.Printf("[LLM] first attempt: %d valid animations, all in bounds, all deltas valid", len(anims))
		return content, anims, nil
	}

	var correctionMsg string
	var problems []string

	if len(anims) == 0 {
		// 没有任何有效动画，分析原始内容
		rawAnimHint := analyzeAnimContent(content)
		problems = append(problems, "未生成有效的动画 JSON")
		if rawAnimHint != "" {
			problems = append(problems, rawAnimHint)
		}
	} else {
		if len(outOfBounds) > 0 {
			problems = append(problems, outOfBounds...)
		}
		if len(deltaErrors) > 0 {
			problems = append(problems, deltaErrors...)
		}
	}

	correctionMsg = fmt.Sprintf("之前的输出有以下问题：\n%s\n\n", strings.Join(problems, "\n"))

	correctionHistory := buildCorrectionHistory(history, content)

	for round := 1; round <= MaxRetryRounds; round++ {
		correctionPrompt := fmt.Sprintf(
			"之前的输出中%s\n请重新生成动画 JSON。注意：\n"+
				"1. 用 ---ANIM--- 分隔多个动画\n"+
				"2. 每个动画标签行后紧跟完整的 JSON 对象\n"+
				"3. JSON 必须包含 svgW, svgH, elements, frames 四个字段\n"+
				"4. 所有元素的坐标(x/y/x2/y2)必须在面板范围内（0 到 svgW/svgH）\n"+
				"5. 元素间留足够间距(gap=52~60)，不要挤在边缘\n"+
				"6. delta 中的每个 key 必须是 elements 中已定义的 id\n"+
				"7. 不要用 markdown 代码块（```json）包裹 JSON\n"+
				"8. 确保 JSON 语法正确，无多余尾逗号\n\n"+
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
			correctionMsg = "未生成有效的动画 JSON（可能缺少 elements/frames 字段，或 JSON 格式错误）。"
			continue
		}

		retryOOB := checkBoundsAll(retryAnims)
		retryDelta := checkDeltaReferences(retryAnims)
		if len(retryOOB) > 0 || len(retryDelta) > 0 {
			log.Printf("[LLM] retry %d: %d anims but issues: bounds=%v deltas=%v", round, len(retryAnims), retryOOB, retryDelta)
			var retryProblems []string
			retryProblems = append(retryProblems, retryOOB...)
			retryProblems = append(retryProblems, retryDelta...)
			correctionMsg = fmt.Sprintf("动画仍有问题：\n%s\n\n坐标必须在 0~svgW 和 0~svgH 之内，delta key 必须是 elements 中定义的 id。", strings.Join(retryProblems, "\n"))
			continue
		}

		log.Printf("[LLM] retry %d: got %d valid animations, all in bounds, all deltas valid", round, len(retryAnims))
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
	if err := json.Unmarshal([]byte(repairJSON(raw)), &a); err != nil {
		return nil
	}
	if a.SVGW <= 0 || a.SVGH <= 0 {
		return []string{fmt.Sprintf("svgW=%.0f svgH=%.0f 无效", a.SVGW, a.SVGH)}
	}

	a = autoResizeAnim(a)

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

func autoResizeAnim(a rawAnim) rawAnim {
	const resizeMargin = 12.0
	const resizePadding = 32.0

	minX, minY := 1e9, 1e9
	maxX, maxY := -1e9, -1e9

	for _, e := range a.Els {
		ex, ey, ew, eh := e.X, e.Y, 0.0, 0.0
		switch e.Kind {
		case "rect":
			ew, eh = e.W, e.H
		case "circle":
			r := e.R
			if r <= 0 {
				r = 22
			}
			ex, ey = e.X-r, e.Y-r
			ew, eh = r*2, r*2
		case "line":
			ex = e.X
			if e.X2 < ex {
				ex = e.X2
			}
			ey = e.Y
			if e.Y2 < ey {
				ey = e.Y2
			}
			ew = e.X2 - e.X
			if ew < 0 {
				ew = -ew
			}
			ew += 2
			eh = e.Y2 - e.Y
			if eh < 0 {
				eh = -eh
			}
			eh += 2
		default:
			ew, eh = 2, 2
		}
		if ex < minX {
			minX = ex
		}
		if ey < minY {
			minY = ey
		}
		if ex+ew > maxX {
			maxX = ex + ew
		}
		if ey+eh > maxY {
			maxY = ey + eh
		}
	}

	shiftX := 0.0
	shiftY := 0.0
	if minX < resizeMargin {
		shiftX = resizeMargin - minX
	}
	if minY < resizeMargin {
		shiftY = resizeMargin - minY
	}

	neededW := maxX + shiftX + resizePadding
	neededH := maxY + shiftY + resizePadding
	if neededW > a.SVGW {
		a.SVGW = neededW
	}
	if neededH > a.SVGH {
		a.SVGH = neededH
	}

	if shiftX == 0 && shiftY == 0 {
		return a
	}

	for i := range a.Els {
		a.Els[i].X += shiftX
		a.Els[i].Y += shiftY
		if a.Els[i].Kind == "line" {
			a.Els[i].X2 += shiftX
			a.Els[i].Y2 += shiftY
		}
	}
	return a
}

func checkDeltaReferences(blocks []AnimBlock) []string {
	var violations []string
	for bi, b := range blocks {
		v := validateDeltaRefs(b.JSON)
		for _, msg := range v {
			violations = append(violations, fmt.Sprintf("[动画%d] %s", bi+1, msg))
		}
	}
	if len(violations) > 5 {
		violations = append(violations[:5], fmt.Sprintf("... 还有 %d 个引用错误", len(violations)-5))
	}
	return violations
}

func validateDeltaRefs(raw string) []string {
	var a rawAnim
	if err := json.Unmarshal([]byte(repairJSON(raw)), &a); err != nil {
		return nil
	}

	elemIDs := make(map[string]bool)
	for _, e := range a.Els {
		elemIDs[e.ID] = true
	}

	var violations []string
	for fi, f := range a.Frs {
		fm, ok := f.(map[string]interface{})
		if !ok {
			continue
		}
		delta, ok := fm["delta"].(map[string]interface{})
		if !ok {
			continue
		}
		for key := range delta {
			if !elemIDs[key] {
				violations = append(violations, fmt.Sprintf("帧%d delta 引用未知元素 %q", fi+1, key))
				if len(violations) >= 5 {
					return violations
				}
			}
		}
	}
	return violations
}

func analyzeAnimContent(content string) string {
	const sep = "---ANIM---"
	idx := strings.Index(content, sep)
	if idx == -1 {
		return "输出中未找到 ---ANIM--- 分隔符，请确保使用该分隔符标记动画区域"
	}
	animPart := content[idx+len(sep):]

	// 检查是否在 anim 部分中有 JSON
	jsonStart := findTopLevelJSONStart(animPart)
	if jsonStart < 0 {
		return "---ANIM--- 后面未找到有效的 JSON 对象（应以 { 开头）"
	}

	// 尝试修复后验证
	jsonStr := strings.TrimSpace(animPart[jsonStart:])
	repaired := repairJSON(jsonStr)
	var test map[string]interface{}
	if err := json.Unmarshal([]byte(repaired), &test); err != nil {
		return fmt.Sprintf("JSON 格式错误: %v。请检查括号配对和逗号位置", err)
	}

	if _, ok := test["elements"]; !ok {
		return "JSON 缺少 elements 字段"
	}
	if _, ok := test["frames"]; !ok {
		return "JSON 缺少 frames 字段"
	}
	if _, ok := test["svgW"]; !ok {
		return "JSON 缺少 svgW 字段"
	}
	if _, ok := test["svgH"]; !ok {
		return "JSON 缺少 svgH 字段"
	}

	return "JSON 结构基本正确但无法验证（可能是 elements/frames 数组格式问题）"
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
