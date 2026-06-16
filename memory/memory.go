package memory

import (
	"strings"
)

const DefaultMaxChars = 8000

type HistoryEntry struct {
	Role    string
	Content string
}

func BuildHistory(entries []HistoryEntry, maxChars int) string {
	if len(entries) == 0 {
		return ""
	}
	if maxChars <= 0 {
		maxChars = DefaultMaxChars
	}

	var sb strings.Builder
	remaining := maxChars

	for i := len(entries) - 1; i >= 0 && remaining > 0; i-- {
		entry := entries[i]
		line := formatEntry(entry)
		if len(line) > remaining {
			line = truncate(line, remaining)
			if !strings.HasSuffix(line, "\n") {
				line += "\n"
			}
		}
		sb.WriteString(line)
		remaining -= len(line)
	}

	result := sb.String()
	result = strings.TrimSpace(result)

	var lines []string
	for _, l := range strings.Split(result, "\n") {
		if l != "" {
			lines = append(lines, l)
		}
	}

	var reversed strings.Builder
	for i := len(lines) - 1; i >= 0; i-- {
		reversed.WriteString(lines[i])
		reversed.WriteString("\n")
	}

	return strings.TrimSpace(reversed.String())
}

func formatEntry(entry HistoryEntry) string {
	prefix := "用户: "
	if entry.Role == "assistant" {
		prefix = "助手: "
	}
	return prefix + entry.Content + "\n\n"
}

func truncate(s string, maxLen int) string {
	rs := []rune(s)
	if len(rs) <= maxLen {
		return s
	}
	return string(rs[:maxLen]) + "..."
}
