# Tools / 工具协议

[文档索引 / Documentation index](../README.md)

> Scope / 阅读范围：The current chat tool protocol appears first. The broader catalogue also contains design-level interfaces and examples; it does not imply every listed tool is available. Run `cargo run --locked -- tool` for the actual CLI tool list, and see [Chat](../CHAT.md#tools-and-verification) for the desktop tools.

## Current chat tools

The conversation agent calls `sandbox.verify` for bounded portable JavaScript
checks and `animation.create` for publishing multi-case animations from those
execution traces. No algorithm selector is exposed in the desktop. Verification
may be used without animation, and ordinary replies need no tools. Animation
links open inside the native application. See [implemented protocol and limits](../CHAT.md).
The native execution and IR tools below remain separate facilities.

Structured execution observations may include stable node IDs, labels, node
values, semantic states, explicit parent/child or graph edges, edge labels,
pointers and row/column cells. `animation.create` accepts the layouts `array`,
`table`, `stack`, `queue`, `tree`, `graph`, `linked-list` and `grid`; the layout
must match the emitted structure. Values-only traces remain valid. Node, edge,
pointer, cell and text limits are enforced by the sandbox and the animation
compiler; dangling references and mismatched layouts return actionable tool
errors.

## 1. Purpose

This document defines the tool interface used by the OJ Resolve Animation Agent.

The Agent should not perform every task internally.

Instead, it should delegate precise and deterministic work to tools.

The core principle is:

> The Agent decides what to do.
>
> Tools perform concrete operations.

Tools should be:

- explicit
- composable
- deterministic where possible
- inspectable
- validated
- safe
- easy to test

---

## 2. Tool Categories

The tool system is divided into:

```text
Problem Tools
Code Tools
Execution Tools
Trace Tools
IR Tools
Visualization Tools
Animation Tools
Render Tools
Export Tools
Project Tools
```

High-level flow:

```text
Problem
  ↓
Problem Tools
  ↓
Code / Execution Tools
  ↓
Trace Tools
  ↓
IR Tools
  ↓
Visualization Tools
  ↓
Animation Tools
  ↓
Render / Export Tools
```

---

## 3. Tool Naming

Use namespaced tool names.

Examples:

```text
problem.parse
problem.classify

code.detect_language
code.compile
code.run
code.trace

ir.read
ir.write
ir.validate

visualization.plan
visualization.apply_template

animation.compile
animation.validate

render.preview
render.frame
render.export
```

Tool names should be:

- stable
- lowercase
- explicit
- domain-oriented

Avoid names such as:

```text
do_thing
process
magic
execute_all
```

---

## 4. Tool Design Rules

Every tool should have:

```text
name
description
input schema
output schema
error model
side-effect definition
```

Tools should return structured output.

Avoid returning unstructured text when a machine-readable result is possible.

---

## 5. Tool Result Envelope

Recommended common tool result:

```json
{
  "ok": true,
  "data": {},
  "warnings": []
}
```

On failure:

```json
{
  "ok": false,
  "error": {
    "code": "ERROR_CODE",
    "message": "Human readable message",
    "recoverable": true
  }
}
```

Detailed errors should follow [ERROR_HANDLING.md](ERROR_HANDLING.md).

---

## 6. Deterministic vs AI-Assisted Tools

Tools are classified as:

### Deterministic

Same input should produce equivalent output.

Examples:

```text
code.compile
code.run
code.trace
ir.validate
animation.compile
animation.validate
render.frame
render.export
```

---

### AI-Assisted

Output may vary.

Examples:

```text
problem.parse
problem.classify
visualization.plan
```

AI-assisted tools must still return validated structured output.

---

## 7. problem.parse

### Purpose

Parse raw OJ problem text into a structured problem model.

### Input

```json
{
  "text": "...raw problem statement..."
}
```

Optional:

```json
{
  "source": "leetcode",
  "language": "zh-CN"
}
```

### Output

```json
{
  "title": "Two Sum",
  "description": "...",
  "inputs": [],
  "outputs": [],
  "constraints": [],
  "samples": [],
  "tags": []
}
```

### Notes

This tool should extract structure only.

It should not generate animation instructions.

---

## 8. problem.classify

### Purpose

Classify the problem into algorithm and visualization categories.

### Input

```json
{
  "problem": {}
}
```

### Output

```json
{
  "algorithm_categories": [
    "array",
    "hash-map"
  ],
  "likely_templates": [
    "array-map"
  ],
  "confidence": 0.92
}
```

Possible categories:

```text
array
two-pointer
sliding-window
binary-search
sorting
stack
queue
tree
graph
dfs
bfs
dynamic-programming
greedy
union-find
shortest-path
string
geometry
simulation
```

---

## 9. code.detect_language

### Purpose

Detect source code language.

### Input

```json
{
  "source": "..."
}
```

### Output

```json
{
  "language": "rust",
  "confidence": 0.99
}
```

Supported values may include:

```text
rust
cpp
c
go
python
javascript
typescript
java
unknown
```

---

## 10. code.analyze

### Purpose

Analyze source code semantically.

### Input

```json
{
  "language": "rust",
  "source": "..."
}
```

### Output

```json
{
  "functions": [],
  "important_variables": [],
  "data_structures": [],
  "loops": [],
  "branches": [],
  "recursive_calls": [],
  "semantic_steps": []
}
```

This tool helps build Algorithm IR.

It should not emit final Animation IR.

---

## 11. code.compile

### Purpose

Compile user code inside a sandbox.

### Input

```json
{
  "language": "rust",
  "source": "...",
  "options": {}
}
```

### Output

Success:

```json
{
  "artifact_id": "exec_123",
  "warnings": []
}
```

Failure:

```json
{
  "error": {
    "code": "EXECUTION_COMPILE_FAILED",
    "stderr": "..."
  }
}
```

### Rules

Compilation must happen in a sandbox.

Do not compile untrusted code in the main process.

---

## 12. code.run

### Purpose

Execute compiled code.

### Input

```json
{
  "artifact_id": "exec_123",
  "stdin": "..."
}
```

Optional limits:

```json
{
  "timeout_ms": 3000,
  "memory_mb": 256
}
```

### Output

```json
{
  "stdout": "...",
  "stderr": "",
  "exit_code": 0,
  "duration_ms": 18
}
```

---

## 13. code.trace

### Purpose

Execute code and produce standardized Trace IR.

This is one of the most important tools in the system.

### Input

```json
{
  "artifact_id": "exec_123",
  "stdin": "...",
  "trace_config": {
    "max_events": 10000
  }
}
```

### Output

```json
{
  "trace_ir": {}
}
```

The result must follow [the IR specification](IR.md).

### Rule

Prefer real runtime traces over AI-inferred traces.

---

## 14. trace.simulate

### Purpose

Generate a conceptual trace when executable code is unavailable.

### Input

```json
{
  "algorithm_ir": {},
  "input": {}
}
```

### Output

```json
{
  "trace_ir": {},
  "trace_source": "simulation",
  "confidence": "inferred"
}
```

### Rule

This output must never be marked as runtime-verified.

---

## 15. trace.inspect

### Purpose

Inspect and summarize Trace IR.

Useful for debugging and Agent reasoning.

### Input

```json
{
  "trace_ir": {}
}
```

### Output

```json
{
  "event_count": 42,
  "entities_used": [],
  "phases": [],
  "warnings": []
}
```

---

## 16. ir.validate

### Purpose

Validate any IR document.

### Input

```json
{
  "ir_type": "trace",
  "document": {}
}
```

Supported types:

```text
algorithm
trace
visualization
animation
```

### Output

Valid:

```json
{
  "valid": true,
  "errors": []
}
```

Invalid:

```json
{
  "valid": false,
  "errors": [
    {
      "path": "events[4].entity",
      "code": "UNKNOWN_ENTITY",
      "message": "Unknown entity 'left_ptr'"
    }
  ]
}
```

---

## 17. ir.read

### Purpose

Read a serialized IR file.

### Input

```json
{
  "path": "debug/trace.json"
}
```

### Output

```json
{
  "ir_type": "trace",
  "document": {}
}
```

---

## 18. ir.write

### Purpose

Write IR to a file for debugging, caching, or reproducibility.

### Input

```json
{
  "path": "debug/trace.json",
  "document": {}
}
```

### Output

```json
{
  "path": "debug/trace.json",
  "written": true
}
```

---

## 19. ir.migrate

### Purpose

Migrate an IR document between schema versions.

### Input

```json
{
  "ir_type": "animation",
  "from_version": 1,
  "to_version": 2,
  "document": {}
}
```

### Output

```json
{
  "document": {},
  "version": 2
}
```

---

## 20. visualization.apply_template

### Purpose

Apply a known visualization template.

### Input

```json
{
  "template": "binary-search",
  "algorithm_ir": {},
  "trace_ir": {}
}
```

### Output

```json
{
  "visualization_ir": {}
}
```

Preferred templates:

```text
array
two-pointer
sliding-window
binary-search
sorting
stack
queue
tree
dfs
bfs
graph
dp-table
union-find
```

### Rule

Prefer templates over fully AI-generated visualization when a suitable template exists.

---

## 21. visualization.plan

### Purpose

Generate Visualization IR for algorithms that need custom planning.

### Input

```json
{
  "algorithm_ir": {},
  "trace_ir": {},
  "constraints": {}
}
```

### Output

```json
{
  "visualization_ir": {}
}
```

### Rule

Output must be validated with:

```text
ir.validate
```

before animation compilation.

---

## 22. visualization.inspect

### Purpose

Inspect visualization structure.

### Input

```json
{
  "visualization_ir": {}
}
```

### Output

```json
{
  "scene_count": 1,
  "object_count": 8,
  "bindings": [],
  "warnings": []
}
```

---

## 23. animation.compile

### Purpose

Compile Trace IR + Visualization IR into deterministic Animation IR.

### Input

```json
{
  "trace_ir": {},
  "visualization_ir": {}
}
```

### Output

```json
{
  "animation_ir": {}
}
```

### Rule

This tool should preferably be deterministic.

Do not require an LLM for normal compilation.

---

## 24. animation.validate

### Purpose

Validate Animation IR.

### Checks

```text
duplicate IDs
invalid targets
negative durations
invalid timeline order
invalid object lifecycle
unsupported actions
non-finite numeric values
```

### Input

```json
{
  "animation_ir": {}
}
```

### Output

```json
{
  "valid": true,
  "errors": [],
  "warnings": []
}
```

---

## 25. animation.inspect

### Purpose

Inspect timeline and object structure.

### Output Example

```json
{
  "duration": 8.4,
  "object_count": 17,
  "action_count": 63,
  "scene_count": 1
}
```

---

## 26. animation.optimize

### Purpose

Simplify or optimize Animation IR.

Possible optimizations:

```text
merge consecutive waits
remove redundant state changes
combine compatible actions
deduplicate styles
reduce unnecessary keyframes
```

### Input

```json
{
  "animation_ir": {}
}
```

### Output

```json
{
  "animation_ir": {},
  "changes": []
}
```

---

## 27. render.preview

### Purpose

Create an interactive preview session.

### Input

```json
{
  "animation_ir": {}
}
```

### Output

```json
{
  "preview_id": "preview_001",
  "duration": 8.4
}
```

Preview should support:

```text
play
pause
seek
step-forward
step-backward
restart
speed
```

---

## 28. render.frame

### Purpose

Render a single frame at a specific timestamp.

### Input

```json
{
  "animation_ir": {},
  "time": 2.5,
  "width": 1280,
  "height": 720
}
```

### Output

```json
{
  "image_id": "frame_0025"
}
```

This tool is useful for automated visual inspection.

---

## 29. render.frames

### Purpose

Render a frame range.

### Input

```json
{
  "animation_ir": {},
  "start": 0,
  "end": 5,
  "fps": 30
}
```

### Output

```json
{
  "frame_sequence_id": "frames_001"
}
```

---

## 30. render.inspect

### Purpose

Analyze rendered frames for common visual issues.

Possible checks:

```text
object overlap
text clipping
off-screen elements
zero-size objects
unreadable labels
unexpected empty scene
```

### Input

```json
{
  "image_id": "frame_0025"
}
```

### Output

```json
{
  "issues": [
    {
      "type": "text-clipping",
      "target": "explanation_text"
    }
  ]
}
```

This may be AI-assisted or rule-based.

---

## 31. render.export

### Purpose

Export the final animation.

### Input

```json
{
  "animation_ir": {},
  "format": "mp4",
  "output": "output/two-sum.mp4",
  "width": 1280,
  "height": 720,
  "fps": 60
}
```

Supported formats may include:

```text
mp4
webm
gif
png-sequence
```

### Output

```json
{
  "path": "output/two-sum.mp4",
  "duration": 8.4
}
```

---

## 32. asset.load

### Purpose

Load external visual assets.

Examples:

```text
font
icon
image
background
```

### Input

```json
{
  "path": "assets/icon.svg"
}
```

### Output

```json
{
  "asset_id": "asset_123",
  "type": "svg"
}
```

---

## 33. template.list

### Purpose

List available algorithm visualization templates.

### Output

```json
{
  "templates": [
    "array",
    "binary-search",
    "two-pointer",
    "bfs",
    "dfs",
    "dp-table"
  ]
}
```

---

## 34. template.get

### Purpose

Load template metadata.

### Input

```json
{
  "name": "binary-search"
}
```

### Output

```json
{
  "name": "binary-search",
  "required_entities": [
    "array",
    "left",
    "right",
    "mid"
  ]
}
```

---

## 35. cache.get

### Purpose

Read cached intermediate output.

### Input

```json
{
  "key": "..."
}
```

### Output

```json
{
  "hit": true,
  "value": {}
}
```

---

## 36. cache.put

### Purpose

Store reusable output.

### Input

```json
{
  "key": "...",
  "value": {}
}
```

### Output

```json
{
  "stored": true
}
```

---

## 37. project.read

### Purpose

Read project configuration or template files.

Examples:

```text
AGENTS.md
docs/reference/DESIGN.md
docs/reference/IR.md
template definitions
```

This should be read-only unless explicitly writing project files.

---

## 38. project.write

### Purpose

Write generated project artifacts.

Examples:

```text
algorithm.json
trace.json
visualization.json
animation.json
rendered output
```

Do not use this tool to silently modify unrelated source files.

---

## 39. Tool Safety

Tools that execute user code must be sandboxed.

This includes:

```text
code.compile
code.run
code.trace
```

They must enforce:

```text
CPU limit
memory limit
execution timeout
filesystem isolation
network restriction
process limit
output limit
```

---

## 40. Tool Permissions

Some tools are read-only.

Example:

```text
problem.parse
problem.classify
ir.validate
animation.validate
render.frame
```

Some tools produce side effects.

Example:

```text
ir.write
project.write
render.export
cache.put
```

Side effects must be explicit.

---

## 41. Agent Tool Policy

The Agent should:

1. prefer deterministic tools
2. validate AI-generated structured output
3. avoid guessing runtime state
4. use templates when possible
5. inspect rendered output before final export when practical
6. preserve intermediate artifacts for debugging

---

## 42. Preferred Tool Sequence

Typical full generation flow:

```text
problem.parse
    ↓
problem.classify
    ↓
code.detect_language
    ↓
code.compile
    ↓
code.trace
    ↓
ir.validate(trace)
    ↓
visualization.apply_template
    ↓
ir.validate(visualization)
    ↓
animation.compile
    ↓
animation.validate
    ↓
render.preview
    ↓
render.frame
    ↓
render.export
```

---

## 43. Without Source Code

If no source code is available:

```text
problem.parse
    ↓
problem.classify
    ↓
Algorithm IR generation
    ↓
trace.simulate
    ↓
visualization.apply_template
    ↓
animation.compile
```

The output must clearly mark the trace as inferred.

---

## 44. Automatic Repair Flow

If AI-generated IR fails validation:

```text
Generate IR
   ↓
ir.validate
   ↓
invalid
   ↓
Agent repairs only invalid parts
   ↓
ir.validate
```

Do not regenerate the entire pipeline unnecessarily.

---

## 45. Visual Repair Flow

If rendered output is visually poor:

```text
render.frame
    ↓
render.inspect
    ↓
visual issue found
    ↓
modify Visualization IR
    ↓
animation.compile
    ↓
render.frame
```

Prefer fixing Visualization IR rather than manually patching rendered pixels.

---

## 46. Tool Boundaries

Tools should not cross architectural layers unnecessarily.

For example:

`code.trace` should produce Trace IR.

It should not produce MP4.

`visualization.plan` should produce Visualization IR.

It should not compile video.

`render.export` should consume Animation IR.

It should not analyze algorithms.

---

## 47. MVP Tool Set

The first version should implement only the most important tools.

Recommended MVP:

```text
problem.parse

code.detect_language
code.compile
code.run
code.trace

ir.validate

visualization.apply_template
visualization.plan

animation.compile
animation.validate

render.preview
render.frame
render.export
```

This is sufficient for a useful end-to-end system.

---

## 48. Priority Tools

Highest priority:

```text
code.trace
visualization.apply_template
animation.compile
ir.validate
```

These tools provide the most architectural value.

---

## 49. Example Agent Session

User:

```text
Generate an animation for this binary search solution.
```

Agent:

```text
problem.parse
```

Then:

```text
code.detect_language
```

Then:

```text
code.compile
```

Then:

```text
code.trace
```

Result:

```text
Trace IR
```

Then:

```text
visualization.apply_template("binary-search")
```

Then:

```text
animation.compile
```

Then:

```text
render.frame(time=2.0)
```

Agent inspects the frame.

If valid:

```text
render.export(format="mp4")
```

---

## 50. Final Principle

The Agent should not be the executor of low-level work.

The Agent should behave like:

> planner + orchestrator + reviewer

Tools should behave like:

> compiler + runtime + validator + renderer

The preferred architecture is:

```text
Agent
  ↓
Tool Invocation
  ↓
Structured Result
  ↓
Validation
  ↓
Next Decision
```

not:

```text
Agent
  ↓
Generate everything as text
  ↓
Hope it works
```
