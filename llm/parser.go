package llm

import (
	"encoding/json"
	"strings"
)

type AnimResponse struct {
	Markdown string
	AnimJSON string
	HasAnim  bool
}

func ParseResponse(raw string) AnimResponse {
	result := AnimResponse{}

	idx := strings.Index(raw, "---ANIM---")
	if idx == -1 {
		idx = strings.Index(raw, "```json")
		if idx == -1 {
			result.Markdown = raw
			return result
		}
		result.Markdown = strings.TrimSpace(raw[:idx])
		rest := raw[idx+7:]
		end := strings.Index(rest, "```")
		if end != -1 {
			result.AnimJSON = strings.TrimSpace(rest[:end])
			result.Markdown += "\n\n" + strings.TrimSpace(rest[end+3:])
		} else {
			result.AnimJSON = strings.TrimSpace(rest)
		}
	} else {
		result.Markdown = strings.TrimSpace(raw[:idx])
		animPart := raw[idx+11:]
		result.AnimJSON = strings.TrimSpace(animPart)
	}

	if result.AnimJSON != "" {
		var test map[string]interface{}
		if json.Unmarshal([]byte(result.AnimJSON), &test) == nil {
			result.HasAnim = true
		}
	}

	result.Markdown = strings.TrimSpace(result.Markdown)
	if result.Markdown == "" {
		result.Markdown = raw
	}

	return result
}
