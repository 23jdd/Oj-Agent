# IR.md

# Intermediate Representation Specification

## 1. Purpose

This document defines the intermediate representations used by the OJ Resolve Animation Agent.

The system must not directly convert:

```text
Problem
→ Animation Code
```

Instead, it should use multiple structured intermediate representations:

```text
Problem
   ↓
Algorithm IR
   ↓
Trace IR
   ↓
Visualization IR
   ↓
Animation IR
   ↓
Renderer
```

Each IR has a distinct responsibility.

The goal is to make generation:

- deterministic
- inspectable
- serializable
- testable
- debuggable
- renderer-independent
- language-independent
- AI-friendly
- extensible

---

# 2. IR Layers

The system uses four main IR layers.

```text
Algorithm IR
    What the algorithm means

Trace IR
    What happened during execution

Visualization IR
    How the algorithm should be explained visually

Animation IR
    What changes visually over time
```

These layers must remain separate.

---

# 3. Core Rule

Each IR should answer exactly one category of question.

## Algorithm IR

Answers:

> What are the important entities, phases, and operations of this algorithm?

---

## Trace IR

Answers:

> What happened for this specific input?

---

## Visualization IR

Answers:

> How should these algorithm states be represented visually?

---

## Animation IR

Answers:

> Which visual objects change, and when?

---

# 4. Dependency Direction

IR dependencies must flow in one direction.

```text
Algorithm IR
     ↓
Trace IR
     ↓
Visualization IR
     ↓
Animation IR
```

Do not introduce reverse dependencies.

For example:

```text
Trace IR → Renderer
```

is forbidden.

Likewise:

```text
Algorithm IR → Canvas Coordinates
```

is forbidden.

---

# 5. Shared IR Header

Every serialized IR document should contain a common header.

Example:

```json
{
  "schema": "trace-ir",
  "version": 1,
  "id": "trace_two_sum_001"
}
```

Recommended common fields:

```text
schema
version
id
metadata
```

Optional metadata:

```text
created_at
generator
model
source_language
problem_id
generation_id
```

---

# 6. Stable IDs

All significant entities should use stable IDs.

Example:

```text
array_nums
pointer_left
pointer_right
graph_main
node_5
dp_table
```

Prefer semantic IDs.

Good:

```text
current_index
visited_nodes
queue_main
```

Avoid:

```text
obj1
thing2
temp99
```

unless objects are generated dynamically and no semantic name exists.

---

# 7. Algorithm IR

Algorithm IR describes the algorithm at a semantic level.

It should remain independent from:

- source language
- runtime
- rendering
- animation
- screen layout

Example:

```json
{
  "schema": "algorithm-ir",
  "version": 1,
  "id": "two_sum_hashmap",
  "name": "Two Sum with Hash Map",
  "category": "hash-map",
  "entities": [],
  "phases": [],
  "operations": []
}
```

---

# 8. Algorithm Entities

An entity represents a meaningful algorithm object.

Possible types:

```text
Scalar
Array
Matrix
Map
Set
Stack
Queue
Tree
Graph
Node
Edge
Pointer
Range
String
Heap
DisjointSet
Table
Custom
```

Example:

```json
{
  "id": "nums",
  "type": "array",
  "value_type": "int",
  "role": "input"
}
```

Another:

```json
{
  "id": "seen",
  "type": "map",
  "key_type": "int",
  "value_type": "int",
  "role": "state"
}
```

---

# 9. Entity Roles

Recommended entity roles:

```text
input
output
state
temporary
index
pointer
accumulator
result
auxiliary
```

These help visualization planning.

---

# 10. Algorithm Phases

Algorithms may be divided into semantic phases.

Example:

```json
{
  "id": "scan_array",
  "name": "Scan Array",
  "order": 1
}
```

Example phases for two-sum:

```text
Initialize
Scan
Lookup
Insert
Finish
```

Example phases for BFS:

```text
Initialize
EnqueueStart
VisitNode
InspectNeighbors
EnqueueNeighbor
Finish
```

---

# 11. Algorithm Operations

Operations describe semantic behavior.

Recommended common operation kinds:

```text
Read
Write
Compare
Assign
Insert
Remove
Swap
Push
Pop
Enqueue
Dequeue
Visit
Relax
Union
Find
Branch
Loop
Call
Return
Update
Lookup
Mark
Unmark
```

Example:

```json
{
  "id": "lookup_complement",
  "kind": "lookup",
  "target": "seen",
  "description": "Search complement in hash map"
}
```

---

# 12. Algorithm Relations

Relations describe connections between entities.

Example:

```json
{
  "type": "points-to",
  "from": "left_pointer",
  "to": "nums"
}
```

Possible relations:

```text
points-to
indexes
contains
parent-of
child-of
connects
depends-on
derived-from
```

---

# 13. Algorithm IR Example

Example for two-sum:

```json
{
  "schema": "algorithm-ir",
  "version": 1,
  "id": "two_sum_hashmap",

  "name": "Two Sum with Hash Map",
  "category": "hash-map",

  "entities": [
    {
      "id": "nums",
      "type": "array",
      "value_type": "int",
      "role": "input"
    },
    {
      "id": "target",
      "type": "scalar",
      "value_type": "int",
      "role": "input"
    },
    {
      "id": "seen",
      "type": "map",
      "key_type": "int",
      "value_type": "int",
      "role": "state"
    },
    {
      "id": "i",
      "type": "scalar",
      "value_type": "int",
      "role": "index"
    },
    {
      "id": "complement",
      "type": "scalar",
      "value_type": "int",
      "role": "temporary"
    }
  ],

  "phases": [
    {
      "id": "initialize",
      "order": 0
    },
    {
      "id": "scan",
      "order": 1
    },
    {
      "id": "finish",
      "order": 2
    }
  ]
}
```

---

# 14. Trace IR

Trace IR represents one concrete execution.

Algorithm IR says:

> A lookup happens.

Trace IR says:

> At index 1, value 7 looked up key 2 and found index 0.

Trace IR must be concrete.

---

# 15. Trace Structure

Recommended structure:

```text
Trace
├── Initial State
├── Events
└── Final State
```

Example:

```json
{
  "schema": "trace-ir",
  "version": 1,
  "algorithm": "two_sum_hashmap",

  "initial_state": {},
  "events": [],
  "final_state": {}
}
```

---

# 16. Trace Events

Each event should contain:

```text
id
seq
kind
entity
payload
source?
```

Example:

```json
{
  "id": "event_003",
  "seq": 3,
  "kind": "lookup",
  "entity": "seen",
  "payload": {
    "key": 2,
    "found": true,
    "value": 0
  }
}
```

---

# 17. Trace Sequence

Trace events should use deterministic ordering.

Prefer:

```text
seq: 0
seq: 1
seq: 2
seq: 3
```

Do not depend solely on timestamps.

Timestamps may be optional metadata.

---

# 18. State Snapshots

Trace events may contain state deltas.

Example:

```json
{
  "kind": "write",
  "entity": "i",
  "before": 0,
  "after": 1
}
```

Prefer deltas over complete snapshots when practical.

However, periodic snapshots may be used for:

- seeking
- debugging
- replay optimization

---

# 19. Trace Event Taxonomy

Recommended trace events:

```text
Initialize

Read
Write
Assign

Compare
Branch

Insert
Remove
Lookup

Push
Pop

Enqueue
Dequeue

VisitNode
VisitEdge

RelaxEdge

Swap

Call
Return

LoopStart
LoopEnd

Mark
Unmark

EmitResult

Finish
```

The taxonomy should remain relatively small.

---

# 20. Trace Payloads

Payload format depends on event kind.

Example comparison:

```json
{
  "kind": "compare",
  "payload": {
    "left": {
      "entity": "nums",
      "index": 2,
      "value": 11
    },
    "operator": ">",
    "right": {
      "value": 7
    },
    "result": true
  }
}
```

---

# 21. Source Mapping

Trace events may reference source code.

Example:

```json
{
  "source": {
    "file": "solution.rs",
    "line": 14,
    "column": 8
  }
}
```

Optional:

```text
function
span_start
span_end
```

This enables synchronized code highlighting.

---

# 22. Trace Confidence

A trace should indicate its origin.

Example:

```json
{
  "trace_source": "runtime",
  "confidence": "verified"
}
```

Possible sources:

```text
runtime
instrumentation
simulation
model
manual
```

Possible confidence values:

```text
verified
derived
inferred
unknown
```

Never treat AI-inferred execution as runtime-verified execution.

---

# 23. Trace IR Example

```json
{
  "schema": "trace-ir",
  "version": 1,

  "algorithm": "two_sum_hashmap",

  "initial_state": {
    "nums": [2, 7, 11, 15],
    "target": 9,
    "seen": {}
  },

  "events": [
    {
      "id": "e0",
      "seq": 0,
      "kind": "write",
      "entity": "i",
      "after": 0
    },

    {
      "id": "e1",
      "seq": 1,
      "kind": "read",
      "entity": "nums",
      "payload": {
        "index": 0,
        "value": 2
      }
    },

    {
      "id": "e2",
      "seq": 2,
      "kind": "assign",
      "entity": "complement",
      "after": 7
    },

    {
      "id": "e3",
      "seq": 3,
      "kind": "lookup",
      "entity": "seen",
      "payload": {
        "key": 7,
        "found": false
      }
    },

    {
      "id": "e4",
      "seq": 4,
      "kind": "insert",
      "entity": "seen",
      "payload": {
        "key": 2,
        "value": 0
      }
    }
  ]
}
```

---

# 24. Visualization IR

Visualization IR maps algorithm semantics into visual semantics.

Trace IR contains:

```text
lookup key 7 in map
```

Visualization IR may decide:

```text
Highlight the current array cell.
Show complement = 7.
Highlight the map lookup region.
```

Visualization IR does not contain frame-level animation.

---

# 25. Visualization IR Responsibilities

Visualization IR decides:

- which entities are visible
- which visual representation to use
- visual grouping
- semantic highlighting
- annotations
- explanations
- layout relationships
- event-to-visual mappings

It should not decide precise per-frame interpolation.

---

# 26. Visual Components

Recommended semantic components:

```text
ArrayView
MatrixView

ScalarView

MapView
SetView

StackView
QueueView

TreeView
GraphView

NodeView
EdgeView

PointerView
RangeView

CodeView

FormulaView
TextView

DPTableView

LegendView
```

---

# 27. Visualization Object

Example:

```json
{
  "id": "nums_view",
  "type": "array-view",
  "bind": "nums"
}
```

Another:

```json
{
  "id": "index_pointer",
  "type": "pointer-view",
  "bind": "i",
  "target": "nums_view"
}
```

---

# 28. Binding

Visualization components should bind to semantic entities.

Example:

```text
nums_view
    binds to
nums
```

This is preferable to manually updating every visual object.

Bindings may include:

```text
value
index
range
membership
selection
visited-state
```

---

# 29. Layout Relations

Visualization IR should express relative layout.

Prefer:

```json
{
  "placement": {
    "below": "title",
    "align": "center"
  }
}
```

over:

```json
{
  "x": 381,
  "y": 217
}
```

Absolute coordinates belong later in layout compilation.

---

# 30. Layout Primitives

Useful layout relations:

```text
above
below
left-of
right-of
inside
center
align-start
align-end
align-center
stack-horizontal
stack-vertical
grid
overlay
```

Example:

```json
{
  "id": "seen_view",
  "type": "map-view",
  "placement": {
    "right_of": "nums_view",
    "gap": "large"
  }
}
```

---

# 31. Visual State

Visual objects may define semantic states.

Example:

```text
normal
active
selected
visited
candidate
matched
success
error
disabled
muted
```

The exact colors are controlled by the design/theme layer.

Visualization IR should prefer semantic states.

Good:

```json
{
  "state": "active"
}
```

Avoid:

```json
{
  "color": "#ff0000"
}
```

unless the visual effect specifically requires a raw value.

---

# 32. Event Mapping

Visualization IR can map Trace IR events to visual reactions.

Example:

```json
{
  "event": "read",
  "entity": "nums",
  "visual_action": "activate-element"
}
```

Another:

```json
{
  "event": "lookup",
  "entity": "seen",
  "visual_action": "focus-key"
}
```

---

# 33. Explanations

Visualization IR may contain narration or explanation text.

Example:

```json
{
  "id": "explain_lookup",
  "type": "annotation",
  "text": "Check whether the complement already exists."
}
```

Explanations should remain semantic.

Animation timing determines when they appear.

---

# 34. Visualization Scene

A visualization may contain multiple conceptual scenes.

Example:

```text
Scene 1
Explain input

Scene 2
Run algorithm

Scene 3
Show result
```

Recommended representation:

```json
{
  "scenes": [
    {
      "id": "intro",
      "objects": []
    },
    {
      "id": "execution",
      "objects": []
    },
    {
      "id": "result",
      "objects": []
    }
  ]
}
```

---

# 35. Visualization IR Example

```json
{
  "schema": "visualization-ir",
  "version": 1,

  "scene": {
    "id": "execution",

    "objects": [
      {
        "id": "nums_view",
        "type": "array-view",
        "bind": "nums"
      },

      {
        "id": "seen_view",
        "type": "map-view",
        "bind": "seen",
        "placement": {
          "right_of": "nums_view"
        }
      },

      {
        "id": "index_pointer",
        "type": "pointer-view",
        "bind": "i",
        "target": "nums_view"
      },

      {
        "id": "complement_formula",
        "type": "formula-view",
        "expression": "target - nums[i]"
      }
    ],

    "mappings": [
      {
        "trace_event": "read",
        "entity": "nums",
        "action": "activate-element"
      },

      {
        "trace_event": "lookup",
        "entity": "seen",
        "action": "focus-key"
      }
    ]
  }
}
```

---

# 36. Animation IR

Animation IR describes concrete visual changes over time.

It is the final IR consumed by the animation engine.

It should be:

- deterministic
- renderer-independent
- timeline-based
- easy to replay
- easy to seek
- easy to serialize

---

# 37. Animation IR Structure

Recommended structure:

```text
Animation
├── Resources
├── Scene Graph
├── Timeline
└── Metadata
```

Example:

```json
{
  "schema": "animation-ir",
  "version": 1,
  "scene": {},
  "timeline": []
}
```

---

# 38. Scene Graph

Scene graph objects represent renderable objects.

Possible types:

```text
Group
Text
Rectangle
Circle
Line
Arrow
Path

Array
Matrix
Map
Stack
Queue

Tree
Graph

CodeBlock
Formula

Image
```

Semantic components may survive into Animation IR if the renderer understands them.

Otherwise they can be compiled into primitives.

---

# 39. Visual Object Properties

Common properties:

```text
position
size
opacity
rotation
scale
visible
style
z_index
```

Example:

```json
{
  "id": "cell_0",
  "type": "rectangle",
  "position": {
    "x": 100,
    "y": 200
  },
  "size": {
    "width": 64,
    "height": 48
  },
  "style": "array-cell"
}
```

---

# 40. Timeline

Animation IR uses a timeline.

Example:

```json
{
  "at": 1.2,
  "duration": 0.3,
  "action": "highlight",
  "target": "cell_0"
}
```

Events should remain deterministic.

---

# 41. Core Animation Actions

Recommended primitive actions:

```text
Create
Delete

Show
Hide

Move
Resize
Rotate
Scale

FadeIn
FadeOut

Highlight
Unhighlight

SetStyle
SetState

UpdateText
UpdateValue

Connect
Disconnect

CameraMove
CameraZoom

Wait

Sequence
Parallel
```

Avoid adding extremely specialized actions unless necessary.

---

# 42. Animation Composition

Complex animation should be composed.

Example:

```text
Sequence
├── Highlight array cell
├── Show complement formula
├── Parallel
│   ├── Focus map
│   └── Highlight lookup key
└── Wait
```

Representation:

```json
{
  "action": "sequence",
  "children": [
    {},
    {},
    {
      "action": "parallel",
      "children": []
    }
  ]
}
```

---

# 43. Timing

Time is expressed in seconds.

Example:

```json
{
  "at": 2.0,
  "duration": 0.25
}
```

Rules:

```text
at >= 0
duration >= 0
```

`NaN` and infinite values are invalid.

---

# 44. Easing

Animation actions may specify easing.

Recommended standard easing names:

```text
linear
ease-in
ease-out
ease-in-out
```

Optional advanced easing:

```text
cubic-bezier
```

Avoid renderer-specific easing identifiers.

---

# 45. Parallel Actions

Example:

```json
{
  "action": "parallel",
  "children": [
    {
      "action": "move",
      "target": "pointer"
    },
    {
      "action": "highlight",
      "target": "cell_2"
    }
  ]
}
```

Parallel actions start together.

---

# 46. Sequence Actions

Example:

```json
{
  "action": "sequence",
  "children": [
    {
      "action": "highlight",
      "target": "cell_2"
    },
    {
      "action": "wait",
      "duration": 0.4
    },
    {
      "action": "unhighlight",
      "target": "cell_2"
    }
  ]
}
```

---

# 47. Style References

Animation IR should reference theme styles whenever possible.

Example:

```json
{
  "style": "algorithm.active"
}
```

Prefer this over:

```json
{
  "fill": "#FF0000"
}
```

This keeps Animation IR independent from visual themes.

---

# 48. Camera

Optional camera state:

```text
position
zoom
rotation
viewport
```

Recommended animation actions:

```text
CameraMove
CameraZoom
CameraFit
```

Example:

```json
{
  "action": "camera-fit",
  "targets": [
    "tree_root",
    "tree_level_3"
  ]
}
```

---

# 49. Code Highlighting

Animation IR may include code-related objects.

Example:

```json
{
  "action": "highlight-code",
  "target": "solution_code",
  "line": 14
}
```

This should originate from Trace IR source mapping.

---

# 50. Animation IR Example

```json
{
  "schema": "animation-ir",
  "version": 1,

  "scene": {
    "objects": [
      {
        "id": "nums",
        "type": "array"
      },
      {
        "id": "seen",
        "type": "map"
      },
      {
        "id": "pointer_i",
        "type": "pointer"
      }
    ]
  },

  "timeline": [
    {
      "at": 0.0,
      "duration": 0.3,
      "action": "show",
      "target": "nums"
    },

    {
      "at": 0.4,
      "duration": 0.2,
      "action": "move",
      "target": "pointer_i",
      "to": {
        "index": 0
      }
    },

    {
      "at": 0.6,
      "duration": 0.2,
      "action": "highlight",
      "target": "nums[0]",
      "state": "active"
    }
  ]
}
```

---

# 51. IR Compilation

IR conversion should happen through explicit compiler stages.

```text
Algorithm IR
    ↓
Trace Builder

Trace IR
    ↓
Visualization Compiler

Visualization IR
    ↓
Animation Compiler

Animation IR
    ↓
Renderer
```

Each compiler should have clear inputs and outputs.

---

# 52. Validation

Every IR layer must be validated.

Pipeline:

```text
Generate
 ↓
Parse
 ↓
Schema Validate
 ↓
Semantic Validate
 ↓
Accept
```

Do not delay validation until rendering.

---

# 53. Algorithm IR Validation

Check:

```text
unique entity IDs
valid entity types
valid operation references
valid phase references
valid relations
```

---

# 54. Trace IR Validation

Check:

```text
monotonic event sequence
valid entity references
valid indexes
valid operation semantics
valid state transitions
valid source mapping
```

---

# 55. Visualization IR Validation

Check:

```text
valid bindings
known components
valid placement references
valid event mappings
valid scene references
```

---

# 56. Animation IR Validation

Check:

```text
unique object IDs
valid targets
valid timeline order
valid durations
supported actions
valid object lifecycle
finite numeric values
```

---

# 57. IR Versioning

Every IR format must use explicit versioning.

Example:

```text
algorithm-ir v1
trace-ir v1
visualization-ir v1
animation-ir v1
```

Breaking schema changes require a version change.

---

# 58. Migration

If serialized IR must survive between versions, provide migration.

Example:

```text
Animation IR v1
      ↓
migration
      ↓
Animation IR v2
```

Do not silently reinterpret old data.

---

# 59. Serialization

Default serialization format:

```text
JSON
```

JSON is useful because it is:

- easy for LLMs
- easy to debug
- language-independent
- human-readable

Binary formats may be added later for performance.

---

# 60. Optional Binary Representation

If Animation IR becomes large, internal representation may use:

```text
MessagePack
CBOR
bincode
protobuf
```

However, JSON should remain available for debugging and interchange.

---

# 61. Canonical Serialization

For testing and caching, serialized IR should use stable ordering when possible.

Benefits:

```text
stable hashes
snapshot tests
cache keys
clean diffs
```

---

# 62. Debug Artifacts

Each generation run may emit:

```text
debug/
├── algorithm.json
├── trace.json
├── visualization.json
└── animation.json
```

These files make pipeline failures easy to inspect.

---

# 63. Caching

IRs are natural cache boundaries.

Example:

```text
Problem
 ↓
Algorithm IR      ← cache

Execution
 ↓
Trace IR          ← cache

Planning
 ↓
Visualization IR  ← cache

Compile
 ↓
Animation IR      ← cache
```

If only rendering style changes, previous semantic IRs should remain reusable.

---

# 64. Hashing

IR cache keys may include:

```text
input hash
schema version
compiler version
model version
prompt version
configuration hash
```

---

# 65. Determinism

Whenever deterministic output is possible, prefer it.

Example:

Trace IR from runtime should be deterministic for:

```text
same code
same input
same environment
```

Visualization planning may be AI-generated, but its output becomes deterministic once serialized.

---

# 66. AI Boundaries

AI may generate:

```text
Algorithm IR
Visualization IR
explanations
semantic mappings
```

AI should generally not generate:

```text
raw frame pixels
precise runtime state when execution is available
video frames
renderer resources
```

Animation IR may be AI-assisted but should preferably be generated by a deterministic compiler from Visualization IR.

---

# 67. Preferred Generation Model

Recommended pipeline:

```text
LLM
 ↓
Algorithm IR

Runtime
 ↓
Trace IR

LLM / Rules / Templates
 ↓
Visualization IR

Deterministic Compiler
 ↓
Animation IR
```

This keeps the model away from low-level animation details.

---

# 68. Templates

Visualization templates should operate on semantic IR.

Example:

```text
Binary Search Trace
        ↓
BinarySearchTemplate
        ↓
Visualization IR
```

Templates may define:

```text
preferred components
layout relationships
event mappings
annotation strategy
```

---

# 69. Generic Fallback

When no specialized visualization exists:

```text
Trace IR
   ↓
Generic Trace Visualizer
   ↓
Visualization IR
```

Generic visualization may show:

```text
variables
current event
data structures
code
explanation
```

This ensures unsupported algorithms can still be visualized.

---

# 70. Example: Binary Search

Algorithm IR:

```text
entities:
array
left
right
mid
target
```

Trace IR:

```text
left = 0
right = 7
mid = 3

compare nums[3] with target

right = 2
...
```

Visualization IR:

```text
ArrayView
RangeView(left, right)
PointerView(mid)
ComparisonText
```

Animation IR:

```text
Move left marker
Move right marker
Move mid pointer
Highlight nums[mid]
Update comparison text
```

Each layer becomes more visual and less semantic.

---

# 71. Example: BFS

Algorithm IR:

```text
graph
queue
visited
current_node
```

Trace IR:

```text
enqueue A
dequeue A
visit A
inspect edge A-B
enqueue B
```

Visualization IR:

```text
GraphView
QueueView
CurrentNodeIndicator
VisitedState
```

Animation IR:

```text
Highlight A
Fade visited state
Move B into queue
Highlight edge A-B
```

---

# 72. Example: Dynamic Programming

Algorithm IR:

```text
input
dp
transition
answer
```

Trace IR:

```text
dp[0] = ...
dp[1] = ...
dp[2] = dp[1] + dp[0]
```

Visualization IR:

```text
DPTableView
FormulaView
DependencyArrows
```

Animation IR:

```text
Highlight dp[1]
Highlight dp[0]
Show dependency arrows
Update dp[2]
```

---

# 73. Expression IR

Expressions may use a small structured format.

Example:

```json
{
  "type": "binary",
  "operator": "-",
  "left": {
    "type": "ref",
    "entity": "target"
  },
  "right": {
    "type": "index",
    "entity": "nums",
    "index": {
      "type": "ref",
      "entity": "i"
    }
  }
}
```

This is better than relying only on raw text:

```text
target - nums[i]
```

Raw display text may still be included.

---

# 74. Value Types

Common values:

```text
Integer
Float
Boolean
String
Null

Array
Map
Set
Tuple

NodeRef
EdgeRef

Custom
```

Values must remain serializable.

---

# 75. References

Object references should use IDs.

Example:

```json
{
  "entity": "nums"
}
```

Do not duplicate entire objects when a reference is sufficient.

---

# 76. Extensibility

Every IR should allow optional extension fields.

Example:

```json
{
  "extensions": {
    "my-feature": {}
  }
}
```

Core renderers should ignore unknown optional extensions where safe.

---

# 77. Unknown Fields

Readers should preferably tolerate unknown optional fields.

Readers must reject:

- unknown required schema versions
- invalid required fields
- invalid enum values where behavior would be ambiguous

---

# 78. Renderer Independence

Animation IR must not assume a specific backend such as:

```text
Skia
Vello
wgpu
Canvas
SVG
```

Avoid backend-specific types in core Animation IR.

Bad:

```text
SkiaPaint
VkTexture
HTMLCanvasGradient
```

Good:

```text
FillStyle
StrokeStyle
TextStyle
```

---

# 79. Language Independence

Algorithm IR and Trace IR must not depend on one source language.

Rust:

```text
Vec<i32>
```

Go:

```text
[]int
```

Python:

```text
list[int]
```

should all map to a semantic type such as:

```text
Array<Integer>
```

Source-specific metadata may be attached separately.

---

# 80. No Presentation Leakage

Algorithm IR should never contain:

```text
color
font
x
y
width
height
animation duration
```

Trace IR should never contain:

```text
screen coordinates
camera movement
render styles
```

Visualization IR may contain semantic layout, but should avoid final pixel geometry when possible.

Animation IR is the correct layer for concrete presentation behavior.

---

# 81. No Execution Leakage

Visualization IR should not invent runtime events.

Animation IR must not alter algorithm state.

Visual state and algorithm state are separate.

---

# 82. Round-Trip Debugging

Developers should be able to inspect every stage independently.

Example:

```text
algorithm.json
```

correct?

Then:

```text
trace.json
```

correct?

Then:

```text
visualization.json
```

correct?

Then:

```text
animation.json
```

correct?

This is the primary debugging strategy.

---

# 83. Testing Strategy

Each compiler boundary should have tests.

Example:

```text
Trace IR
    ↓
Visualization Compiler
    ↓
Expected Visualization IR
```

Use snapshot tests where appropriate.

---

# 84. Golden Tests

Maintain small canonical examples.

Recommended cases:

```text
linear array scan
two pointers
binary search
bubble sort
stack
queue
BFS
DFS
simple DP
```

Store expected IR artifacts.

Example:

```text
tests/golden/two_sum/
├── algorithm.json
├── trace.json
├── visualization.json
└── animation.json
```

---

# 85. Minimal IR Philosophy

Do not make the IR more complex than necessary.

When deciding whether to add a new field or action, ask:

> Can the existing model express this clearly?

Prefer composition over adding special cases.

Example:

Instead of:

```text
SpecialFancySwapAndHighlightOperation
```

prefer:

```text
Parallel
├── Swap
└── Highlight
```

---

# 86. Semantic First

Prefer semantic descriptions over visual tricks.

Good:

```text
state: "visited"
```

Less ideal:

```text
fill: "#729B79"
```

Good:

```text
pointer targets array index 3
```

Less ideal:

```text
pointer x = 417
```

Semantic information survives layout and theme changes.

---

# 87. IR Ownership

Recommended ownership:

```text
Agent Layer
    owns Algorithm IR generation

Runtime Layer
    owns Trace IR

Visualization Layer
    owns Visualization IR

Animation Compiler
    owns Animation IR

Renderer
    consumes Animation IR
```

No single module should own every representation.

---

# 88. Error Handling

Invalid IR must return structured validation errors.

Example:

```text
IRValidationError {
    ir: Trace
    path: events[14].entity
    code: UNKNOWN_ENTITY
    message: "Unknown entity 'left_ptr'"
}
```

The original IR should be preserved for debugging where safe.

Detailed error handling follows `EXCEPTION.md`.

---

# 89. Recommended Repository Structure

```text
src/
├── ir/
│   ├── algorithm/
│   ├── trace/
│   ├── visualization/
│   ├── animation/
│   ├── validation/
│   └── migration/
│
├── compiler/
│   ├── trace_to_visualization/
│   └── visualization_to_animation/
│
└── renderer/
```

Possible Rust form:

```text
src/
├── ir.rs
└── ir/
    ├── algorithm.rs
    ├── trace.rs
    ├── visualization.rs
    ├── animation.rs
    └── error.rs
```

---

# 90. Recommended Rust Shape

Conceptually:

```rust
pub struct AlgorithmIr {
    pub version: u32,
    pub entities: Vec<AlgorithmEntity>,
    pub phases: Vec<AlgorithmPhase>,
}

pub struct TraceIr {
    pub version: u32,
    pub events: Vec<TraceEvent>,
}

pub struct VisualizationIr {
    pub version: u32,
    pub scenes: Vec<VisualScene>,
}

pub struct AnimationIr {
    pub version: u32,
    pub scene: SceneGraph,
    pub timeline: Vec<AnimationAction>,
}
```

Exact Rust types should evolve with implementation.

This document defines architecture, not a frozen Rust API.

---

# 91. MVP Recommendation

For the first version, keep the IR intentionally small.

## Algorithm IR

Support:

```text
Scalar
Array
Map
Stack
Queue
Tree
Graph
Table
Pointer
```

## Trace IR

Support:

```text
Read
Write
Compare
Insert
Remove
Swap
Push
Pop
Enqueue
Dequeue
Visit
Branch
Return
```

## Visualization IR

Support:

```text
ArrayView
MapView
StackView
QueueView
TreeView
GraphView
TableView
PointerView
TextView
CodeView
```

## Animation IR

Support:

```text
Show
Hide
Move
Highlight
Unhighlight
UpdateText
UpdateValue
FadeIn
FadeOut
Wait
Sequence
Parallel
```

Do not attempt to support every possible algorithm in v1.

---

# 92. Evolution Strategy

Add new concepts only when a real visualization requires them.

Suggested order:

```text
v1
arrays + pointers + maps

v2
stack + queue + sorting

v3
trees + graphs

v4
DP + matrix

v5
advanced custom structures
```

This keeps the system manageable.

---

# 93. Final Architecture

The intended system is:

```text
                    ┌───────────────┐
                    │ OJ Problem    │
                    └───────┬───────┘
                            ↓
                    ┌───────────────┐
                    │ Algorithm IR  │
                    └───────┬───────┘
                            ↓
Source Code ─────→ Runtime / Instrumentation
                            ↓
                    ┌───────────────┐
                    │   Trace IR    │
                    └───────┬───────┘
                            ↓
                    ┌───────────────┐
                    │Visualization  │
                    │      IR       │
                    └───────┬───────┘
                            ↓
                    ┌───────────────┐
                    │ Animation IR  │
                    └───────┬───────┘
                            ↓
             ┌──────────────┼──────────────┐
             ↓              ↓              ↓
           Preview         GIF            MP4
```

---

# 94. Final Principle

The IR system exists to prevent this:

```text
LLM
 ↓
large block of animation code
 ↓
unpredictable behavior
 ↓
difficult debugging
```

The desired architecture is:

```text
LLM / Runtime
      ↓
Structured IR
      ↓
Validation
      ↓
Deterministic Compilation
      ↓
Deterministic Rendering
```

If a generated animation is wrong, the developer should be able to determine whether the problem came from:

```text
Algorithm understanding
Trace execution
Visualization planning
Animation compilation
Rendering
```

without guessing.

That separation is the primary purpose of the IR architecture.

---

# Structured animation extension (version 2, backward compatible)

The chat animation compiler can now emit non-array visual structures without
creating a second animation system. Existing `values`, `indices`, `edges`,
`directed`, `text` and action fields remain valid; all new fields are optional
and default to empty.

`SceneObject` and `AnimationAction` may carry:

- `labels`, `states`: per-node or per-cell display labels and semantic states
  (`normal`, `active`/`current`/`visiting`, `done`/`visited`/`success`,
  `error`, `hidden`, `empty`).
- `positions`: normalized `[x, y]` coordinates in `0..1` for graph nodes.
- `edges`, `edge_labels`, `directed`: node or grid-cell connections; indices
  refer to the object's values in row-major grid order.
- `pointers`: a pointer name and target index. For arrays the target is a value
  index; for trees/graphs/linked lists it is a node index; for grids it is a
  row-major cell index.
- `parents`: optional explicit parent indices for tree consumers.
- `rows`, `cols`: required for `grid` objects, with
  `rows * cols == values.len()`.

`AnimationAction::UpdateValue` carries a full `structure` snapshot when
connectivity, labels, states, positions or pointers change. The timeline
compiles structure keyframes alongside value keyframes, so seeking remains
deterministic. `tree`, `graph`, `linked-list` and `grid` object kinds are
rendered by the existing Skia renderer; values-only objects still use the
array/table/stack/queue layouts.
