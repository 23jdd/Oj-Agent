package llm

import (
	"strings"
	"testing"
)

func TestParseResponse_SingleAnim(t *testing.T) {
	raw := `## 题解

内容...

---ANIM---
动画标签：主循环
{"svgW":400,"svgH":200,"elements":[{"id":"a0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"1","style":"normal","rx":6,"visible":true}],"frames":[{"desc":"初始","delta":{"a0":{"style":"highlight"}}}]}`

	result := ParseResponse(raw)
	if !result.HasAnim {
		t.Fatal("expected HasAnim=true")
	}
	if len(result.AnimJSONs) != 1 {
		t.Fatalf("expected 1 anim, got %d", len(result.AnimJSONs))
	}
	if result.AnimLabel[0] != "主循环" {
		t.Fatalf("expected label '主循环', got '%s'", result.AnimLabel[0])
	}
	if !strings.Contains(result.Markdown, "题解") {
		t.Fatal("markdown should contain 题解")
	}
}

func TestParseResponse_MultiAnim(t *testing.T) {
	raw := `## 题解

---ANIM---
动画标签：条件: arr[i] < pivot
{"svgW":400,"svgH":200,"elements":[{"id":"a0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"3","style":"normal","rx":6,"visible":true}],"frames":[{"desc":"小于pivot","delta":{"a0":{"style":"highlight"}}}]}
---ANIM---
动画标签：条件: arr[i] >= pivot
{"svgW":400,"svgH":200,"elements":[{"id":"a0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"8","style":"normal","rx":6,"visible":true}],"frames":[{"desc":"大于等于pivot","delta":{"a0":{"style":"compare"}}}]}`

	result := ParseResponse(raw)
	if !result.HasAnim {
		t.Fatal("expected HasAnim=true")
	}
	if len(result.AnimJSONs) != 2 {
		t.Fatalf("expected 2 anims, got %d", len(result.AnimJSONs))
	}
	if result.AnimLabel[0] != "条件: arr[i] < pivot" {
		t.Fatalf("unexpected label[0]: '%s'", result.AnimLabel[0])
	}
	if result.AnimLabel[1] != "条件: arr[i] >= pivot" {
		t.Fatalf("unexpected label[1]: '%s'", result.AnimLabel[1])
	}
}

func TestParseResponse_NoLabel(t *testing.T) {
	raw := `## 题解

---ANIM---
{"svgW":400,"svgH":200,"elements":[{"id":"a0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"1","style":"normal","rx":6,"visible":true}],"frames":[{"desc":"step","delta":{}}]}`

	result := ParseResponse(raw)
	if !result.HasAnim {
		t.Fatal("expected HasAnim=true")
	}
	if len(result.AnimJSONs) != 1 {
		t.Fatalf("expected 1 anim, got %d", len(result.AnimJSONs))
	}
}

func TestParseResponse_CodeFenceJSON(t *testing.T) {
	raw := `## 题解

---ANIM---
动画标签：通配符匹配
` + "```json" + `
{"svgW":400,"svgH":200,"elements":[{"id":"a0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true}],"frames":[{"desc":"step","delta":{}}]}
` + "```" + ``

	result := ParseResponse(raw)
	if !result.HasAnim {
		t.Fatal("expected HasAnim=true for code-fenced JSON")
	}
	if len(result.AnimJSONs) != 1 {
		t.Fatalf("expected 1 anim, got %d", len(result.AnimJSONs))
	}
	if result.AnimLabel[0] != "通配符匹配" {
		t.Fatalf("expected label '通配符匹配', got '%s'", result.AnimLabel[0])
	}
}

func TestParseResponse_MultiLineJSON(t *testing.T) {
	raw := `## 题解

---ANIM---
动画标签：多行JSON
{
  "svgW": 400,
  "svgH": 200,
  "elements": [
    {"id":"a0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"1","style":"normal","rx":6,"visible":true}
  ],
  "frames": [
    {"desc":"step1","delta":{"a0":{"style":"highlight"}}}
  ]
}`

	result := ParseResponse(raw)
	if !result.HasAnim {
		t.Fatal("expected HasAnim=true for multi-line JSON")
	}
	if len(result.AnimJSONs) != 1 {
		t.Fatalf("expected 1 anim, got %d", len(result.AnimJSONs))
	}
}

func TestParseResponse_EmptyJSON(t *testing.T) {
	raw := `## 题解

---ANIM---
{}`

	result := ParseResponse(raw)
	if result.HasAnim {
		t.Fatal("expected HasAnim=false for empty JSON")
	}
}

func TestParseResponse_GlobMultiAnim(t *testing.T) {
	raw := `## 通配符匹配题解

---ANIM---
动画标签：示例: *匹配1个字符
{"svgW":360,"svgH":140,"elements":[{"id":"s0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true}],"frames":[{"desc":"匹配","delta":{"s0":{"style":"highlight"}}}]}
---ANIM---
动画标签：示例: *匹配0个字符
{"svgW":320,"svgH":140,"elements":[{"id":"s0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true}],"frames":[{"desc":"跳过","delta":{"s0":{"style":"highlight"}}}]}
---ANIM---
动画标签：示例: .匹配单个字符
{"svgW":320,"svgH":140,"elements":[{"id":"s0","kind":"rect","x":10,"y":50,"w":44,"h":40,"text":"a","style":"normal","rx":6,"visible":true}],"frames":[{"desc":"单字符","delta":{"s0":{"style":"highlight"}}}]}`

	result := ParseResponse(raw)
	if !result.HasAnim {
		t.Fatal("expected HasAnim=true")
	}
	if len(result.AnimJSONs) != 3 {
		t.Fatalf("expected 3 anims, got %d", len(result.AnimJSONs))
	}
	if result.AnimLabel[0] != "示例: *匹配1个字符" {
		t.Fatalf("unexpected label[0]: '%s'", result.AnimLabel[0])
	}
	if result.AnimLabel[1] != "示例: *匹配0个字符" {
		t.Fatalf("unexpected label[1]: '%s'", result.AnimLabel[1])
	}
	if result.AnimLabel[2] != "示例: .匹配单个字符" {
		t.Fatalf("unexpected label[2]: '%s'", result.AnimLabel[2])
	}
}

func TestParseResponse_NoAnim(t *testing.T) {
	raw := `## 普通回答

没有动画内容`

	result := ParseResponse(raw)
	if result.HasAnim {
		t.Fatal("expected HasAnim=false")
	}
	if result.Markdown != raw {
		t.Fatalf("expected full markdown, got '%s'", result.Markdown)
	}
}

func TestExtractLabel(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{"动画标签：主循环", "主循环"},
		{"动画标签: 条件分支", "条件分支"},
		{"标签：测试", "测试"},
		{"条件：arr[i] < pivot", "arr[i] < pivot"},
		{"示例：*匹配0个字符", "*匹配0个字符"},
		{"例子：test case", "test case"},
		{"分支：递归", "递归"},
		{"场景：base case", "base case"},
		{"  动画标签：  带空格  ", "带空格"},
		{"普通文本", "普通文本"},
		{"", ""},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got := extractLabel(tt.input)
			if got != tt.expected {
				t.Fatalf("extractLabel(%q) = %q, want %q", tt.input, got, tt.expected)
			}
		})
	}
}
