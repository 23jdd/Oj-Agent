package llm

import (
	"encoding/json"
	"regexp"
	"strings"
)

type AnimResponse struct {
	Markdown  string
	AnimJSONs []string
	AnimLabel []string
	HasAnim   bool
}

func ParseResponse(raw string) AnimResponse {
	result := AnimResponse{}

	const sep = "---ANIM---"

	firstIdx := strings.Index(raw, sep)
	if firstIdx == -1 {
		idx := strings.Index(raw, "```json")
		if idx == -1 {
			result.Markdown = raw
			return result
		}
		result.Markdown = strings.TrimSpace(raw[:idx])
		rest := raw[idx+7:]
		end := strings.Index(rest, "```")
		if end != -1 {
			jsonBlock := strings.TrimSpace(rest[:end])
			if isValidAnimJSON(jsonBlock) {
				result.AnimJSONs = append(result.AnimJSONs, jsonBlock)
				result.AnimLabel = append(result.AnimLabel, "")
				result.HasAnim = true
			}
			result.Markdown += "\n\n" + strings.TrimSpace(rest[end+3:])
		} else {
			jsonBlock := strings.TrimSpace(rest)
			if isValidAnimJSON(jsonBlock) {
				result.AnimJSONs = append(result.AnimJSONs, jsonBlock)
				result.AnimLabel = append(result.AnimLabel, "")
				result.HasAnim = true
			}
		}
	} else {
		result.Markdown = strings.TrimSpace(raw[:firstIdx])

		rest := raw[firstIdx:]

		for {
			idx := strings.Index(rest, sep)
			if idx == -1 {
				break
			}
			rest = rest[idx+len(sep):]

			nextIdx := strings.Index(rest, sep)

			var block string
			var remaining string
			if nextIdx != -1 {
				block = rest[:nextIdx]
				remaining = rest[nextIdx:]
			} else {
				block = rest
			}

			label, jsonStr := parseAnimBlock(strings.TrimSpace(block))
			if jsonStr != "" && isValidAnimJSON(jsonStr) {
				result.AnimJSONs = append(result.AnimJSONs, jsonStr)
				result.AnimLabel = append(result.AnimLabel, label)
				result.HasAnim = true
			}

			if nextIdx == -1 {
				break
			}
			rest = remaining
		}
	}

	result.Markdown = strings.TrimSpace(result.Markdown)
	if result.Markdown == "" {
		result.Markdown = raw
	}

	return result
}

func parseAnimBlock(block string) (label string, jsonStr string) {
	block = strings.TrimSpace(block)
	if block == "" {
		return "", ""
	}

	jsonStart := strings.Index(block, "```json")
	if jsonStart != -1 {
		if jsonStart > 0 {
			label = extractLabel(block[:jsonStart])
		}
		inner := block[jsonStart+7:]
		jsonEnd := strings.Index(inner, "```")
		if jsonEnd != -1 {
			jsonStr = strings.TrimSpace(inner[:jsonEnd])
		} else {
			jsonStr = strings.TrimSpace(inner)
		}
		return label, jsonStr
	}

	jsonStart = strings.Index(block, "```")
	if jsonStart != -1 {
		if jsonStart > 0 {
			label = extractLabel(block[:jsonStart])
		}
		inner := block[jsonStart+3:]
		jsonEnd := strings.Index(inner, "```")
		if jsonEnd != -1 {
			jsonStr = strings.TrimSpace(inner[:jsonEnd])
		} else {
			jsonStr = strings.TrimSpace(inner)
		}
		return label, jsonStr
	}

	jsonIdx := findTopLevelJSONStart(block)
	if jsonIdx > 0 {
		label = extractLabel(block[:jsonIdx])
		jsonStr = strings.TrimSpace(block[jsonIdx:])
		return label, jsonStr
	}
	if jsonIdx == 0 {
		jsonStr = block
		return "", jsonStr
	}

	return "", block
}

func findTopLevelJSONStart(s string) int {
	if len(s) > 0 && s[0] == '{' {
		return 0
	}
	if len(s) > 0 && s[0] == '[' {
		return 0
	}
	idx := strings.Index(s, "\n{")
	if idx != -1 {
		return idx + 1
	}
	idx = strings.Index(s, "\n[")
	if idx != -1 {
		return idx + 1
	}
	return strings.Index(s, "{")
}

func extractLabel(line string) string {
	line = strings.TrimSpace(line)
	line = strings.TrimLeft(line, "#*/- \t")

	prefixes := []string{
		"动画标签：", "动画标签:", "标签：", "标签:",
		"条件：", "条件:", "示例：", "示例:", "例子：", "例子:",
		"分支：", "分支:", "场景：", "场景:",
	}
	for _, p := range prefixes {
		if strings.HasPrefix(line, p) {
			return strings.TrimSpace(strings.TrimPrefix(line, p))
		}
	}

	return line
}

var trailingCommaRE = regexp.MustCompile(`,(\s*[}\]])`)

func RepairJSON(raw string) string {
	return repairJSON(raw)
}

func repairJSON(raw string) string {
	raw = strings.TrimSpace(raw)
	if raw == "" {
		return raw
	}

	// 去掉尾逗号: ,} 或 ,] 或 ,\n}
	raw = trailingCommaRE.ReplaceAllString(raw, "$1")

	// 补齐缺失的闭合括号
	depth := 0
	inString := false
	escaped := false
	for _, c := range raw {
		if escaped {
			escaped = false
			continue
		}
		if c == '\\' && inString {
			escaped = true
			continue
		}
		if c == '"' {
			inString = !inString
			continue
		}
		if inString {
			continue
		}
		if c == '{' || c == '[' {
			depth++
		} else if c == '}' || c == ']' {
			depth--
		}
	}

	if depth > 0 {
		// 检查最后一个完整结构是对象还是数组
		bracketCount := 0
		lastStruct := byte('{')
		for i := len(raw) - 1; i >= 0; i-- {
			if raw[i] == '}' {
				bracketCount++
			} else if raw[i] == '{' {
				bracketCount--
			} else if raw[i] == ']' {
				bracketCount++
			} else if raw[i] == '[' {
				bracketCount--
			}
			if bracketCount < 0 && raw[i] == '{' {
				lastStruct = '{'
				break
			}
			if bracketCount < 0 && raw[i] == '[' {
				lastStruct = '['
				break
			}
		}

		closing := ""
		for i := 0; i < depth; i++ {
			if lastStruct == '{' {
				closing += "}"
			} else {
				closing += "]"
			}
		}
		raw += closing
	}

	return raw
}

func isValidAnimJSON(raw string) bool {
	if raw == "" {
		return false
	}
	repaired := repairJSON(raw)
	var test map[string]interface{}
	if err := json.Unmarshal([]byte(repaired), &test); err != nil {
		// try as array of anim objects
		var arr []map[string]interface{}
		if err2 := json.Unmarshal([]byte(repaired), &arr); err2 != nil {
			return false
		}
		if len(arr) > 0 {
			test = arr[0]
		} else {
			return false
		}
	}
	elems, _ := test["elements"]
	frames, _ := test["frames"]
	_, hasElems := elems.([]interface{})
	_, hasFrames := frames.([]interface{})
	return hasElems && hasFrames
}
