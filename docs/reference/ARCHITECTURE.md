# Architecture / 系统架构

[文档索引 / Documentation index](../README.md)

> Scope / 阅读范围：This is an architectural design reference. Some pipeline and directory examples describe intended boundaries rather than the exact current file layout. For the implemented desktop workflow, read [Chat](../CHAT.md), [Usage](../USAGE.md) and [src/lib.rs](../../src/lib.rs).

## Current desktop workflow

The desktop is now a conversational workspace. Markdown messages and explicit
`@` attachment snapshots enter a bounded Rig agent loop. It may answer without
tools, call `sandbox.verify`, or publish verified multi-case animations via
`animation.create`. Registered `oj-animation:` links navigate to a native page
inside the application; they do not represent a web service. The portable
JavaScript VM has no host capabilities. Original non-JavaScript source is not
claimed to be executed when AI uses a translation. See [chat architecture](../CHAT.md).
The IR/compiler/rendering layers below remain internal; the user no longer
selects an algorithm template or fills sample forms.

## 1. Overview

OJ Resolve Animation Agent is an AI-assisted system for converting online judge problems, solutions, and source code into structured algorithm visualizations and animations.

The system should be able to accept inputs such as:

- OJ problem statements
- Example inputs and outputs
- Solution explanations
- Source code
- User-provided algorithm descriptions

and generate:

- algorithm analysis
- execution traces
- visualization plans
- animation scenes
- interactive previews
- exported video or GIF output

The primary goal is:

> Convert algorithm reasoning into deterministic, inspectable, and renderable animation data.

The LLM must not be tightly coupled to the rendering implementation.

---

## 2. Core Design Principle

The architecture follows this pipeline:

```text
Problem
   ↓
Problem Analysis
   ↓
Algorithm Model
   ↓
Execution Trace
   ↓
Visualization Plan
   ↓
Animation IR
   ↓
Renderer
   ↓
Preview / GIF / MP4
```

The most important architectural rule is:

> AI produces structured intermediate representations, not final pixels.

The renderer must remain deterministic.

---

## 3. High-Level Architecture

```text
┌─────────────────────────────┐
│          User Input         │
│                             │
│ Problem / Code / Solution   │
└──────────────┬──────────────┘
               │
               ▼
┌─────────────────────────────┐
│      Agent Orchestrator     │
│                             │
│ controls the whole workflow │
└──────────────┬──────────────┘
               │
      ┌────────┴────────┐
      ▼                 ▼
┌─────────────┐   ┌─────────────┐
│ Problem     │   │ Code        │
│ Analyzer    │   │ Analyzer    │
└──────┬──────┘   └──────┬──────┘
       └──────────┬───────┘
                  ▼
         ┌─────────────────┐
         │ Algorithm Model │
         └────────┬────────┘
                  ▼
         ┌─────────────────┐
         │ Trace Generator │
         └────────┬────────┘
                  ▼
        ┌───────────────────┐
        │ Visualization     │
        │ Planner           │
        └─────────┬─────────┘
                  ▼
         ┌─────────────────┐
         │ Animation IR    │
         └────────┬────────┘
                  ▼
      ┌─────────────────────────┐
      │ Rendering / Export      │
      │                         │
      │ Preview / GIF / MP4     │
      └─────────────────────────┘
```

---

## 4. Major Modules

### 4.1 Input Layer

Responsible for normalizing user input.

Supported inputs may include:

```text
ProblemStatement
SourceCode
SolutionText
SampleInput
SampleOutput
CustomInput
```

Normalized representation:

```text
ProblemInput {
    title
    description
    constraints
    samples
    source_code?
    explanation?
}
```

The input layer must not perform deep algorithm reasoning.

Its responsibility is normalization only.

---

## 5. Agent Orchestrator

The Agent Orchestrator coordinates all AI-related work.

Responsibilities:

- understand user intent
- decide which analyzers are required
- invoke problem analysis
- invoke code analysis
- request missing structured information
- validate intermediate results
- request corrections when output is invalid
- coordinate visualization planning

The orchestrator should not contain rendering logic.

Example flow:

```text
User Input
   ↓
Orchestrator
   ↓
Analyze Problem
   ↓
Analyze Code
   ↓
Merge Understanding
   ↓
Generate Trace Model
   ↓
Generate Visualization Plan
   ↓
Validate
   ↓
Animation IR
```

---

## 6. Problem Analyzer

The Problem Analyzer extracts the semantic structure of an OJ problem.

Output should include:

```text
ProblemModel {
    title
    problem_type
    inputs
    outputs
    constraints
    invariants
    algorithm_candidates
    important_entities
}
```

Example problem types:

```text
sorting
two-pointer
sliding-window
binary-search
dynamic-programming
greedy
graph
tree
dfs
bfs
union-find
shortest-path
string
geometry
simulation
```

The analyzer should also identify what concepts are worth visualizing.

Example:

```text
Two Sum

Important entities:
- array
- current index
- hash map
- current target
- discovered pair
```

---

## 7. Code Analyzer

The Code Analyzer converts source code into algorithm-level semantics.

It should not attempt to visualize raw syntax directly.

Responsibilities:

- identify main algorithm phases
- identify important variables
- identify data structures
- identify loops
- identify recursion
- identify state transitions
- identify conditions that affect visualization
- map code locations to semantic actions

Example:

```rust
for i in 0..nums.len() {
    let target = goal - nums[i];

    if let Some(j) = map.get(&target) {
        return (j, i);
    }

    map.insert(nums[i], i);
}
```

Semantic output:

```text
Loop over array

For each index:
1. highlight current element
2. calculate required complement
3. search complement in hash map
4. if found:
      highlight matching pair
      terminate
5. otherwise:
      insert current value into hash map
```

---

## 8. Algorithm Model

The Algorithm Model represents the algorithm independently from:

- programming language
- animation engine
- UI framework
- renderer

Example:

```text
AlgorithmModel {
    name: "Two Sum Hash Map"

    entities: [
        Array(nums),
        Map(seen),
        Scalar(target),
        Scalar(index)
    ]

    phases: [
        Initialize,
        Iterate,
        Lookup,
        Insert,
        Finish
    ]
}
```

The Algorithm Model is semantic.

It describes:

> What the algorithm does.

It does not describe:

> How it should look.

---

## 9. Trace Generator

The Trace Generator converts an algorithm execution into discrete events.

This is one of the most important modules.

Example input:

```text
nums = [2, 7, 11, 15]
target = 9
```

Example trace:

```text
Step 0:
Initialize map = {}

Step 1:
index = 0
value = 2
complement = 7
lookup 7
not found

Step 2:
insert 2 -> 0

Step 3:
index = 1
value = 7
complement = 2
lookup 2
found at index 0

Step 4:
return [0, 1]
```

Machine representation:

```text
TraceEvent {
    timestamp
    event_type
    entity
    payload
    source_location?
}
```

Possible event types:

```text
Create
Read
Write
Compare
Highlight
Insert
Remove
Move
Swap
Push
Pop
Visit
Relax
Branch
Call
Return
Update
Finish
```

---

## 10. Trace IR

The trace format should remain generic.

Example:

```json
{
  "type": "compare",
  "left": {
    "entity": "nums",
    "index": 1
  },
  "right": {
    "value": 7
  }
}
```

Another example:

```json
{
  "type": "visit-node",
  "graph": "graph",
  "node": 5
}
```

This layer should be deterministic whenever execution can be reproduced locally.

Prefer real execution traces over LLM-invented traces.

---

## 11. Visualization Planner

The Visualization Planner converts semantic events into a visual explanation.

Its responsibility is deciding:

- which objects should be visible
- how they are arranged
- which events deserve animation
- which steps can be grouped
- which explanatory text should appear
- camera or viewport transitions
- pacing

Example:

```text
Algorithm:
Two Sum

Visualization:

Left:
Array

Right:
Hash map

Bottom:
Current expression:
target - nums[i]

Step:
Highlight current array cell.

Then:
Display complement.

Then:
Animate lookup in hash map.

Then:
Insert value or highlight matching pair.
```

The planner produces a `VisualizationPlan`.

---

## 12. Visualization Model

Example:

```text
VisualizationPlan {
    scene_type: ArrayAndMap

    elements: [
        ArrayView(nums),
        MapView(seen),
        FormulaView(complement),
        ExplanationText
    ]

    mappings: {
        array-read -> highlight-cell
        map-lookup -> highlight-map-key
        map-insert -> insert-map-entry
        return -> success-highlight
    }
}
```

---

## 13. Animation IR

Animation IR is the final renderer-independent animation description.

This layer answers:

> What changes visually over time?

Example:

```text
Scene
 ├── Nodes
 │    ├── Array
 │    ├── Label
 │    ├── Arrow
 │    └── HashMap
 │
 └── Timeline
      ├── Highlight
      ├── Move
      ├── FadeIn
      ├── FadeOut
      ├── UpdateText
      └── Wait
```

Example:

```json
{
  "duration": 0.4,
  "action": "highlight",
  "target": "array[1]",
  "style": "active"
}
```

---

## 14. Animation Actions

Core animation primitives should remain small.

Recommended primitives:

```text
Create
Delete

Show
Hide

Move
Resize
Rotate

FadeIn
FadeOut

Highlight
Unhighlight

UpdateText
UpdateValue

Connect
Disconnect

CameraMove
CameraZoom

Wait
Group
Sequence
Parallel
```

Complex animation should be built by composing these primitives.

---

## 15. Scene Model

A scene consists of visual objects.

Example:

```text
Scene {
    id
    width
    height

    objects

    timeline
}
```

Visual objects may include:

```text
Text
Array
Matrix
Stack
Queue
Tree
Graph
Node
Edge
Pointer
Arrow
Label
CodeBlock
Formula
Table
Shape
```

---

## 16. Visual Semantic Components

Prefer semantic visualization components over raw drawing primitives.

Good:

```text
ArrayView
TreeView
GraphView
StackView
QueueView
HashMapView
DPTableView
PointerView
CodeView
```

Less desirable for Agent-generated output:

```text
Rectangle
Line
Text
Circle
Rectangle
Line
Text
...
```

Semantic components make generated animations more robust.

---

## 17. Renderer

The Renderer consumes Animation IR.

It must not perform algorithm reasoning.

Responsibilities:

- layout
- drawing
- interpolation
- timing
- transitions
- text rendering
- frame rendering
- export

Possible implementations:

```text
Skia
Vello
wgpu
HTML Canvas
SVG
Ebitengine
```

Renderer APIs should remain behind an abstraction.

Example:

```text
Renderer {
    load(scene)
    seek(time)
    render(frame)
    export(config)
}
```

---

## 18. Preview

Interactive preview should support:

```text
play
pause
seek
step-forward
step-backward
speed
restart
```

Optional:

```text
show-code
show-explanation
toggle-debug-events
```

Preview rendering should consume the exact same Animation IR as export.

Avoid separate preview and export animation logic.

---

## 19. Export Pipeline

Supported output formats may include:

```text
PNG
GIF
MP4
WebM
SVG
JSON Animation
```

Pipeline:

```text
Animation IR
     ↓
Timeline Player
     ↓
Frame Renderer
     ↓
Frame Sequence
     ↓
Encoder
     ↓
GIF / MP4 / WebM
```

---

## 20. Agent Output Validation

All LLM-generated structured output must be validated.

Pipeline:

```text
LLM
 ↓
Parse
 ↓
Schema Validation
 ↓
Semantic Validation
 ↓
Repair if possible
 ↓
Accept
```

Never trust raw model output.

Validation examples:

- unknown entity reference
- invalid animation target
- negative duration
- duplicate object ID
- nonexistent trace variable
- unsupported action type
- invalid graph edge
- impossible scene dependency

---

## 21. Deterministic vs AI Responsibilities

AI is good at:

```text
understanding
summarization
semantic mapping
visual planning
explanation
```

AI should not be responsible for:

```text
frame timing precision
geometry math
video encoding
state synchronization
trace execution when deterministic execution is possible
```

Prefer deterministic systems whenever possible.

---

## 22. Source Code Execution

Where practical, source code should be instrumented and executed.

Example:

```text
Source Code
   ↓
Instrumentation
   ↓
Execution
   ↓
Trace Events
```

This is more reliable than asking the LLM:

> Tell me exactly what every runtime state is.

---

## 23. Language Adapters

Execution tracing should use language-specific adapters.

Possible structure:

```text
runtime/
├── rust/
├── go/
├── python/
├── cpp/
└── javascript/
```

Each adapter emits the same Trace IR.

Example:

```text
Rust execution
     ↓
Rust Adapter
     ↓
Trace IR

Python execution
     ↓
Python Adapter
     ↓
Trace IR
```

---

## 24. Algorithm Visualization Templates

Common algorithms should use templates.

Examples:

```text
ArrayTemplate
TwoPointerTemplate
SlidingWindowTemplate
BinarySearchTemplate

StackTemplate
QueueTemplate

TreeTraversalTemplate
GraphTraversalTemplate

DPTableTemplate
SortingTemplate
UnionFindTemplate
```

Templates provide stable defaults.

The Agent should customize templates instead of generating every animation from scratch.

---

## 25. Example End-to-End Flow

Input:

```text
LeetCode Two Sum
+
Rust solution
+
[2,7,11,15]
target = 9
```

Pipeline:

```text
Problem Analyzer
    ↓
Two Sum semantics

Code Analyzer
    ↓
Hash map algorithm

Runtime
    ↓
Execution trace

Visualization Planner
    ↓
Array + HashMap layout

Animation Compiler
    ↓
Animation IR

Renderer
    ↓
Preview

Exporter
    ↓
two-sum.mp4
```

---

## 26. Suggested Directory Structure

```text
project/
├── AGENTS.md
├── GIT.md
├── README.md
├── README.zh-CN.md
├── docs/
│   ├── README.md
│   ├── CHAT.md / USAGE.md / STYLE.md / RENDERER.md
│   ├── reference/
│   │   ├── ARCHITECTURE.md / DESIGN.md / ERROR_HANDLING.md
│   │   └── IR.md / TOOLS.md / SESSIONS.md
│   └── archive/
│
├── src/
│   ├── agent/
│   │   ├── orchestrator
│   │   ├── problem_analyzer
│   │   ├── code_analyzer
│   │   └── visualization_planner
│   │
│   ├── model/
│   │   ├── problem
│   │   ├── algorithm
│   │   ├── trace
│   │   └── animation
│   │
│   ├── trace/
│   │   ├── runtime
│   │   ├── instrumentation
│   │   └── validator
│   │
│   ├── visualization/
│   │   ├── planner
│   │   ├── templates
│   │   └── layout
│   │
│   ├── animation/
│   │   ├── ir
│   │   ├── compiler
│   │   ├── timeline
│   │   └── player
│   │
│   ├── renderer/
│   │   ├── renderer
│   │   ├── text
│   │   └── primitives
│   │
│   ├── export/
│   │   ├── gif
│   │   ├── video
│   │   └── image
│   │
│   └── app/
│
├── tests/
│
└── examples/
```

Exact file organization may vary by programming language.

Module boundaries are more important than directory names.

---

## 27. Dependency Direction

Dependencies should flow downward.

```text
App
 ↓
Agent
 ↓
Algorithm / Trace
 ↓
Visualization
 ↓
Animation
 ↓
Renderer
```

The reverse direction should not occur.

For example:

```text
renderer -> agent
```

is forbidden.

Likewise:

```text
algorithm -> renderer
```

is forbidden.

Algorithm logic must not depend on visual implementation.

---

## 28. Serialization

Important intermediate representations should be serializable.

Recommended serialized artifacts:

```text
problem.json
algorithm.json
trace.json
visualization.json
animation.json
```

Benefits:

- debugging
- caching
- reproducibility
- testing
- manual editing
- model evaluation

---

## 29. Caching

AI calls may be expensive.

Cache outputs based on:

```text
input hash
model version
prompt version
schema version
```

Possible pipeline:

```text
Problem analysis cache
Algorithm analysis cache
Visualization plan cache
```

Execution traces should also be cacheable.

---

## 30. Versioning

IR formats should include versions.

Example:

```json
{
  "schema": "animation-ir",
  "version": 1
}
```

Do not silently change serialized formats.

---

## 31. Testing Strategy

Tests should exist at each boundary.

### Unit Tests

Test:

```text
trace events
animation timeline
layout
interpolation
validation
```

### Golden Tests

Input:

```text
known problem
known trace
```

Expected:

```text
known Animation IR
```

### Snapshot Tests

Useful for:

```text
JSON IR
scene graph
rendered frames
```

### Integration Tests

Test full flow:

```text
Problem
→ Trace
→ Visualization
→ Animation
→ Render
```

---

## 32. Observability

Each generation job should have an ID.

Example:

```text
generation_id
```

Important stages should record:

```text
stage
duration
input
output
warnings
errors
model
token usage
```

Logs must make it possible to answer:

> At which stage did generation fail?

---

## 33. Failure Isolation

Each pipeline stage should fail independently.

Example:

```text
Problem analysis succeeded
Code analysis succeeded
Trace generation succeeded
Visualization planning failed
```

Do not collapse this into:

```text
Generation failed.
```

Errors must retain stage information.

Detailed error rules are defined in [ERROR_HANDLING.md](ERROR_HANDLING.md).

---

## 34. Extensibility

The architecture should allow future support for:

- new programming languages
- new algorithm templates
- new renderers
- new LLM providers
- new export formats
- interactive animation
- voice narration
- subtitles
- code synchronization
- user editing

without rewriting the entire pipeline.

---

## 35. Non-Goals

The core system should not initially attempt to:

- perfectly understand arbitrary programs
- visualize every source code statement
- automatically produce cinematic animation
- depend entirely on LLM-generated execution state
- couple one specific renderer to the algorithm layer

Start with common OJ algorithm patterns.

---

## 36. MVP Scope

Recommended initial MVP:

Supported algorithms:

```text
Array traversal
Two pointers
Sliding window
Binary search
Basic sorting
Stack
Queue
DFS
BFS
Simple DP
```

Supported visual components:

```text
Array
Pointer
Text
Code
Stack
Queue
Tree
Graph
DP Table
```

Supported outputs:

```text
Interactive preview
PNG
GIF
MP4
```

The MVP should prioritize correctness and understandability over visual complexity.

---

## 37. Architectural Principle

Whenever deciding where logic belongs, ask:

> Is this about what the algorithm means?

Put it in:

```text
Algorithm / Trace
```

> Is this about how to explain the algorithm visually?

Put it in:

```text
Visualization
```

> Is this about what changes over time?

Put it in:

```text
Animation
```

> Is this about pixels?

Put it in:

```text
Renderer
```

This boundary must remain clear.

---

## Final Principle

The system should not be:

```text
LLM
 ↓
generate random animation code
 ↓
hope it works
```

It should be:

```text
LLM reasoning
      ↓
structured semantic models
      ↓
validated intermediate representations
      ↓
deterministic animation engine
      ↓
reproducible output
```

The intermediate representations are the core of the architecture.
