# SESSION.md

## Current conversation storage

`chat_messages` adds ordered Markdown messages and attachment snapshots to the
existing SQLite sessions. Tool requests, verification reports and `chat-animation`
bundles remain immutable artifacts with dependencies. Each bundle contains its
named case timelines and verification basis. Application-local links resolve
only inside their owning conversation. Existing session/IR records are retained.
See [current chat workflow](docs/CHAT.md).

The additive `token_usage` table stores one UUID-keyed record per returned chat
model response, scoped to its conversation. Records contain the originating user
message ID, provider, model and optional normalized token counters; no credentials
or raw provider responses are stored. Repeated record IDs are ignored. Totals are
computed from these records and survive restart. Deleting a conversation cascades
to its usage; late cancelled workers cannot recreate deleted records. Existing
messages are preserved, and historical usage cannot be backfilled.

# Session Model Specification

## 1. Purpose

This document defines the session model used by the OJ Resolve Animation Agent.

A session represents one continuous piece of work performed by the Agent.

A session may include:

- problem understanding
- source code analysis
- code execution
- trace generation
- visualization planning
- animation generation
- preview
- revision
- export

The session exists to preserve context and intermediate artifacts across multiple Agent actions.

The core principle is:

> A session is the durable context of one animation-generation workflow.

---

# 2. Session Responsibilities

A session is responsible for tracking:

- user intent
- problem input
- source code
- selected algorithm
- Agent decisions
- tool calls
- generated IR
- validation results
- sandbox executions
- previews
- exports
- warnings
- errors
- revision history

A session should make the entire generation workflow reproducible and inspectable.

---

# 3. Session Is Not

A session is not:

- the Agent itself
- the sandbox
- the renderer
- the animation engine
- a single LLM request
- a single tool invocation

A session coordinates and records interactions between these systems.

---

# 4. High-Level Model

```text
User
  ↓
Session
  ↓
Agent
  ↓
Tools
  ↓
Artifacts
  ↓
Session State
```

More specifically:

```text
Session
├── Input
├── Context
├── Agent State
├── Tool History
├── IR Artifacts
├── Sandbox Jobs
├── Preview Artifacts
├── Export Artifacts
└── Errors / Warnings
```

---

# 5. Session Lifecycle

A session follows this lifecycle:

```text
Created
   ↓
Active
   ↓
Processing
   ↓
Ready
   ↓
Completed
```

Additional terminal or temporary states:

```text
Failed
Cancelled
Paused
```

Recommended enum:

```text
Created
Active
Processing
Ready
Completed
Failed
Cancelled
```

---

# 6. Session State

Conceptual model:

```text
Session {
    id
    status

    input
    context

    current_stage

    artifacts
    tool_calls
    sandbox_jobs

    warnings
    errors

    created_at
    updated_at
}
```

---

# 7. Session ID

Every session must have a unique stable ID.

Example:

```text
session_01JXYZ...
```

The ID should not change during the lifetime of the session.

All generated artifacts should reference the session ID when appropriate.

---

# 8. Session Input

Session input represents the original user request.

Example:

```json
{
  "problem": "...",
  "source_code": "...",
  "language": "rust",
  "sample_input": "...",
  "request": "Generate an animation explaining this solution."
}
```

Possible input fields:

```text
problem
problem_url
source_code
source_language
sample_input
sample_output
solution_text
user_instruction
visual_preferences
```

---

# 9. Input Immutability

Original session input should be preserved.

Do not silently overwrite it during later revisions.

If the user provides updated input, store it as a revision.

Example:

```text
input_v1
input_v2
```

or maintain an explicit change history.

---

# 10. Session Context

Session context contains information derived from the original request.

Example:

```text
problem category
selected algorithm
detected language
selected template
current visualization strategy
user preferences
```

Context should contain useful high-level information, not duplicate every artifact.

---

# 11. Current Stage

The session should track its current pipeline stage.

Recommended values:

```text
Input
ProblemAnalysis
CodeAnalysis
Compilation
Execution
TraceGeneration
VisualizationPlanning
AnimationCompilation
Validation
Preview
Export
Completed
```

Example:

```json
{
  "current_stage": "VisualizationPlanning"
}
```

---

# 12. Session and Pipeline

A typical session progresses through:

```text
Input
 ↓
Problem Analysis
 ↓
Algorithm IR
 ↓
Execution
 ↓
Trace IR
 ↓
Visualization IR
 ↓
Animation IR
 ↓
Preview
 ↓
Export
```

A session should preserve the result of every successfully completed stage.

---

# 13. Artifact Model

All meaningful generated outputs should be stored as artifacts.

Examples:

```text
problem model
algorithm IR
trace IR
visualization IR
animation IR
preview image
video
logs
```

Conceptual artifact:

```text
Artifact {
    id
    kind
    version
    path?
    content_hash
    created_at
    metadata
}
```

---

# 14. Artifact Kinds

Recommended artifact types:

```text
ProblemIR
AlgorithmIR
TraceIR
VisualizationIR
AnimationIR

SourceCode
InstrumentedSource

CompileLog
RuntimeLog

Preview
Frame

PNG
GIF
MP4
WebM

ValidationReport
```

---

# 15. Artifact Identity

Artifacts should have stable IDs.

Example:

```text
artifact_trace_001
artifact_animation_003
```

Do not reference artifacts only through temporary filesystem paths.

Paths may change.

Artifact IDs should remain stable.

---

# 16. Artifact Versioning

Artifacts may have multiple versions during revision.

Example:

```text
visualization_v1
visualization_v2
visualization_v3
```

The session should know which version is active.

Example:

```json
{
  "active_visualization_ir": "artifact_visualization_003"
}
```

---

# 17. Immutable Artifact Principle

Prefer treating generated artifacts as immutable.

Instead of modifying:

```text
animation.json
```

in place, create:

```text
animation_v2.json
```

and update the active reference.

Benefits:

- rollback
- debugging
- comparison
- reproducibility
- auditability

---

# 18. Derived Artifacts

Artifacts should record what they were derived from.

Example:

```text
Animation IR
    derived from
Visualization IR v3
+
Trace IR v1
```

Metadata example:

```json
{
  "derived_from": [
    "trace_001",
    "visualization_003"
  ]
}
```

---

# 19. Dependency Graph

Artifact dependencies form a DAG.

Example:

```text
Problem
   ↓
Algorithm IR

Source Code
   ↓
Trace IR

Algorithm IR + Trace IR
         ↓
Visualization IR
         ↓
Animation IR
         ↓
Preview
         ↓
MP4
```

This dependency graph helps invalidate downstream artifacts when upstream data changes.

---

# 20. Artifact Invalidation

If an upstream artifact changes, dependent artifacts may become stale.

Example:

```text
Trace IR changed
      ↓
Visualization IR becomes stale
      ↓
Animation IR becomes stale
      ↓
Preview becomes stale
```

The session should track this explicitly.

Possible artifact state:

```text
valid
stale
invalid
```

---

# 21. Agent State

The session may store limited Agent state.

Useful examples:

```text
selected template
current plan
pending repair
current revision target
previous validation errors
```

Do not store hidden chain-of-thought.

Store decisions and structured reasoning summaries instead.

---

# 22. Decision Records

Important Agent decisions should be recorded explicitly.

Example:

```json
{
  "decision": "use_template",
  "value": "binary-search",
  "reason": "Problem classified as binary search with array + left/right/mid pointers."
}
```

Decision records should be concise and inspectable.

---

# 23. Tool Call History

Every important tool invocation should be recorded.

Conceptual record:

```text
ToolCall {
    id
    tool
    input_ref
    output_ref
    status
    duration
    started_at
    finished_at
}
```

---

# 24. Tool Call Example

```json
{
  "id": "tool_call_014",
  "tool": "code.trace",
  "status": "completed",
  "input_artifact": "source_001",
  "output_artifact": "trace_001",
  "duration_ms": 137
}
```

---

# 25. Avoid Storing Large Tool Payloads Inline

Large data should be referenced by artifact ID.

Prefer:

```json
{
  "output_artifact": "trace_001"
}
```

instead of embedding a 10 MB trace into session metadata.

---

# 26. Tool Call Status

Recommended status values:

```text
Pending
Running
Completed
Failed
Cancelled
```

A tool failure does not necessarily mean the entire session failed.

---

# 27. Sandbox Jobs

Sandbox executions should be associated with the session.

Example:

```text
Session
 ├── sandbox_compile_001
 ├── sandbox_run_002
 └── sandbox_trace_003
```

Each sandbox job should record:

```text
job_id
operation
language
status
input artifact
output artifact
resource usage
error
```

---

# 28. Sandbox Isolation

Session state must not weaken sandbox isolation.

A sandbox job should only receive explicitly required artifacts.

Do not expose the entire session directory to untrusted code.

---

# 29. Session Workspace

A session may have a dedicated workspace.

Example:

```text
sessions/
└── session_01JXYZ/
    ├── artifacts/
    ├── logs/
    ├── previews/
    ├── exports/
    └── session.json
```

The exact physical layout is implementation-specific.

---

# 30. Recommended Workspace

Example:

```text
session_01JXYZ/
├── session.json
│
├── artifacts/
│   ├── algorithm/
│   ├── trace/
│   ├── visualization/
│   └── animation/
│
├── previews/
│
├── exports/
│
└── logs/
```

Sandbox temporary files should remain outside this trusted workspace.

---

# 31. Session Persistence

A session should be persistable.

After application restart, the system should be able to restore:

- current status
- active artifacts
- pipeline stage
- previous tool results
- generated previews
- export results

Transient processes do not need to survive restart.

---

# 32. Session Serialization

Recommended metadata format:

```text
JSON
```

Example:

```json
{
  "id": "session_01JXYZ",
  "status": "active",
  "current_stage": "preview",

  "active_artifacts": {
    "algorithm_ir": "algorithm_001",
    "trace_ir": "trace_001",
    "visualization_ir": "visualization_003",
    "animation_ir": "animation_003"
  }
}
```

---

# 33. Session Recovery

When restoring a session:

1. load session metadata
2. verify referenced artifacts exist
3. validate artifact versions
4. mark missing artifacts
5. detect stale dependencies
6. determine resumable stage

Do not blindly assume persisted state is valid.

---

# 34. Resuming Work

A user should be able to continue:

```text
"Make the pointer animation slower."
```

without regenerating:

```text
Problem IR
Trace IR
```

The session should modify the smallest necessary layer.

Example:

```text
Animation timing change
      ↓
Animation IR
      ↓
Preview
```

No need to rerun source code.

---

# 35. Minimal Recalculation Principle

When the user requests a change, determine the earliest affected layer.

Example:

### Change color/style

```text
Theme
 ↓
Render again
```

### Change animation pacing

```text
Animation IR
 ↓
Render again
```

### Change visualization layout

```text
Visualization IR
 ↓
Animation IR
 ↓
Render
```

### Change sample input

```text
Trace IR
 ↓
Visualization IR
 ↓
Animation IR
 ↓
Render
```

### Change source algorithm

```text
Algorithm IR
 ↓
Trace IR
 ↓
Visualization IR
 ↓
Animation IR
```

---

# 36. Revision Model

A session may contain multiple revisions.

Example:

```text
Revision 1
Initial animation

Revision 2
Move code panel to the right

Revision 3
Slow down BFS visitation
```

Conceptual:

```text
Session
├── Revision 1
├── Revision 2
└── Revision 3
```

---

# 37. Revision Record

Recommended record:

```json
{
  "id": "revision_003",
  "instruction": "Slow down pointer movement.",
  "changed_artifacts": [
    "animation_004"
  ],
  "created_at": "..."
}
```

---

# 38. Rollback

Because artifacts are versioned, rollback should be simple.

Example:

```text
active_animation_ir:
animation_004
      ↓ rollback
animation_003
```

Do not require regeneration for rollback.

---

# 39. Branching Revisions

Future implementations may allow branching:

```text
Visualization v2
     ├── Style A
     └── Style B
```

This allows users to compare multiple animation variants.

Branch support is optional for MVP.

---

# 40. Validation State

Every IR artifact should record validation status.

Example:

```json
{
  "schema_valid": true,
  "semantic_valid": true
}
```

Possible values:

```text
unknown
valid
invalid
```

Do not treat unvalidated IR as production-ready.

---

# 41. Validation Reports

Validation reports should be artifacts.

Example:

```text
trace_validation_001
animation_validation_004
```

This makes validation failures inspectable.

---

# 42. Error State

Errors belong to a stage or operation.

Session-level errors should reference:

```text
stage
tool call
artifact
sandbox job
```

Example:

```json
{
  "code": "ANIMATION_INVALID_TARGET",
  "stage": "AnimationValidation",
  "artifact": "animation_003"
}
```

Detailed error behavior follows `EXCEPTION.md`.

---

# 43. Warning State

Warnings should not fail the session.

Examples:

```text
fallback font used
trace was sampled
generic visualization template used
MP4 unavailable, GIF available
```

Warnings should remain attached to the relevant artifact or stage.

---

# 44. Partial Success

A session may be usable even when some operations fail.

Example:

```text
Animation IR     ✓
Preview          ✓
GIF export       ✓
MP4 export       ✗
```

Session state may be:

```text
CompletedWithWarnings
```

if this state is supported.

Alternatively keep:

```text
Completed
```

with warnings.

---

# 45. Cancellation

Users may cancel an active session operation.

Cancellation should stop:

- active Agent work
- running tool calls where possible
- sandbox jobs
- rendering
- export

Previously completed artifacts should remain available.

---

# 46. Session Cancellation vs Tool Cancellation

Cancelling a tool does not necessarily cancel the entire session.

Example:

```text
cancel MP4 export
```

should not destroy:

```text
Animation IR
Preview
Trace IR
```

---

# 47. Session Completion

A session may be considered completed when the user's requested output exists.

Example:

```text
requested output = preview
```

Then MP4 export is not required.

Completion depends on user intent.

---

# 48. Session Reopening

Completed sessions may be reopened for revision.

Example:

```text
Completed
   ↓
User requests change
   ↓
Active
```

Do not require a completely new session for every small modification.

---

# 49. Session Forking

A session may be forked.

Example:

```text
Session A
 ↓ fork
Session B
```

The new session may reuse immutable artifacts from the original.

Useful for:

- alternate animation style
- different input
- different explanation depth
- different renderer

---

# 50. Session and Conversation

A conversation and a session are related but not identical.

One conversation may contain:

```text
Session A
Session B
Session C
```

One session may span multiple user messages.

The system should identify which session a user action applies to.

---

# 51. Active Session

There should normally be one active session per interaction context.

Example:

```text
active_session_id
```

When the user says:

```text
"make it slower"
```

the instruction should apply to the active session.

---

# 52. Ambiguous Session References

If multiple sessions exist and the target cannot be safely inferred, the caller should resolve the intended session.

Do not modify arbitrary sessions.

---

# 53. Session Context Window

Do not send the entire session history to the LLM on every call.

Instead build a compact context.

Example:

```text
Current user goal
Current problem summary
Algorithm IR summary
Relevant Trace segment
Current Visualization IR
Latest validation errors
```

This reduces tokens and irrelevant context.

---

# 54. Context Builder

Recommended component:

```text
SessionContextBuilder
```

Responsibilities:

- select relevant artifacts
- summarize large traces
- include current task
- include required constraints
- exclude irrelevant history

---

# 55. Artifact Retrieval

Large artifacts should be retrieved on demand.

Example:

```text
Agent needs events 30–60
        ↓
read Trace IR range
```

Do not automatically inject the entire trace.

---

# 56. Trace Context

Large Trace IR may contain thousands of events.

Agent context should use:

```text
summary
important events
requested range
```

rather than every event.

The full trace remains available through tools.

---

# 57. Session Memory Boundaries

Session memory should contain project-specific working context.

It should not automatically become global Agent memory.

Example:

```text
selected visualization template
```

belongs to the session.

It should not affect unrelated future sessions.

---

# 58. Session Configuration

A session may contain configuration.

Example:

```json
{
  "render": {
    "width": 1280,
    "height": 720,
    "fps": 60
  },
  "animation": {
    "speed": 1.0
  }
}
```

Configuration should be separated from IR where appropriate.

---

# 59. Default Configuration

Global defaults may be inherited when the session is created.

Example:

```text
Project Defaults
      ↓
Session Configuration
      ↓
Per-Tool Overrides
```

Later layers override earlier ones.

---

# 60. Configuration Snapshot

For reproducibility, the effective configuration used for generation should be recorded.

Do not rely only on current global configuration.

---

# 61. Template Selection

The session should record:

```text
candidate templates
selected template
selection reason
```

Example:

```json
{
  "selected_template": "binary-search",
  "reason": "Detected array + left/right/mid binary search structure."
}
```

---

# 62. Model Metadata

If an LLM participates in artifact generation, optionally record:

```text
provider
model
prompt version
schema version
```

This helps reproduce AI-generated artifacts.

Do not store hidden model reasoning.

---

# 63. Prompt Versioning

Agent-generated structured artifacts should record the prompt/template version when useful.

Example:

```text
visualization-planner-v3
```

Changing prompts may change output.

This matters for reproducibility.

---

# 64. Deterministic Tool Results

Deterministic tool results should be reusable within the session.

Example:

```text
same source
+
same input
+
same runtime environment
=
reuse Trace IR
```

Do not rerun expensive deterministic operations unnecessarily.

---

# 65. Session Cache

A session may maintain local cache references.

Example:

```text
compile cache
trace cache
render frame cache
```

Cache entries should not replace authoritative artifacts.

---

# 66. Session Events

The session may expose an internal event stream.

Examples:

```text
SessionCreated
StageChanged
ArtifactCreated
ArtifactActivated
ToolStarted
ToolCompleted
ToolFailed
WarningAdded
SessionCompleted
```

Useful for:

- UI progress
- logging
- observability
- persistence

---

# 67. Event Example

```json
{
  "type": "ArtifactCreated",
  "session_id": "session_001",
  "artifact_id": "trace_001",
  "artifact_type": "TraceIR"
}
```

---

# 68. Event Ordering

Session events should have a monotonic sequence number.

Example:

```text
seq = 1
seq = 2
seq = 3
```

Do not depend solely on wall-clock timestamps for ordering.

---

# 69. Concurrency

A session may have parallel operations.

Example:

```text
render preview frame
+
generate narration
```

However, operations that modify the same active artifact must be coordinated.

---

# 70. Optimistic Revision Control

Artifact updates may include expected version information.

Example:

```text
modify visualization_v3
expected active version = 3
```

If another operation already created v4, reject or reconcile the change.

This prevents concurrent modifications from silently overwriting each other.

---

# 71. Session Locking

Avoid locking an entire session for long-running work.

Prefer fine-grained coordination.

For example:

- artifact update lock
- export job lock
- sandbox job independent execution

---

# 72. Session Storage Interface

Recommended abstraction:

```text
SessionStore {
    create()
    get()
    update()
    append_event()
    add_artifact()
    activate_artifact()
    list_artifacts()
    delete()
}
```

Storage implementation may be:

```text
memory
filesystem
SQLite
PostgreSQL
```

The Agent should not depend on a specific backend.

---

# 73. Artifact Store Interface

Recommended:

```text
ArtifactStore {
    put()
    get()
    exists()
    metadata()
    delete()
}
```

Large binary files should not be stored directly inside session metadata.

---

# 74. MVP Storage

For an initial local application, use:

```text
filesystem
+
session.json
```

Example:

```text
sessions/
└── <session-id>/
```

This is simple and debuggable.

A database can be added later.

---

# 75. SQLite Option

For more structured local persistence:

```text
SQLite
```

can store:

```text
sessions
artifacts metadata
tool calls
events
errors
```

Large files may remain on disk.

---

# 76. Cleanup Policy

Sessions may generate significant temporary data.

Separate:

```text
persistent artifacts
```

from:

```text
temporary artifacts
```

Temporary artifacts may include:

```text
intermediate frames
compiler temporary files
sandbox files
render caches
```

They may be cleaned automatically.

---

# 77. Persistent Artifacts

Recommended persistent artifacts:

```text
Algorithm IR
Trace IR
Visualization IR
Animation IR
final preview
exports
session metadata
```

Retention policy may be configurable.

---

# 78. Session Deletion

Deleting a session should remove:

- session metadata
- owned artifacts
- previews
- exports if owned exclusively
- caches associated only with the session

Shared immutable cached artifacts should use reference counting or independent ownership.

---

# 79. Security

Session metadata must never store secrets unnecessarily.

Avoid storing:

```text
API keys
access tokens
sandbox host credentials
```

Session IDs should not themselves contain sensitive data.

---

# 80. User Source Code

Source code may be sensitive.

Its persistence policy should be explicit.

Possible policy:

```text
persist source: configurable
```

At minimum, avoid unnecessary logging duplication.

---

# 81. Export References

Exports should be associated with the exact Animation IR version.

Example:

```text
animation_v3
   ↓
demo_v3.mp4
```

Do not lose this relationship.

---

# 82. Preview References

A preview should also record:

```text
Animation IR version
render configuration
theme version
```

This makes screenshots and previews reproducible.

---

# 83. Theme Changes

A visual style change may not require semantic regeneration.

Example:

```text
DESIGN/theme changes
      ↓
rerender
```

The session should preserve reusable Animation IR where possible.

---

# 84. Session Summary

A session should be able to produce a compact summary.

Example:

```text
Problem:
Binary Search

Language:
Rust

Algorithm:
Binary search

Trace:
47 events, runtime verified

Visualization:
binary-search template

Animation:
12.8 seconds

Output:
preview available
MP4 exported
```

Useful for Agent context and UI display.

---

# 85. Debug Summary

Developer-facing summaries may additionally include:

```text
artifact versions
tool calls
sandbox jobs
validation warnings
cache hits
timings
```

---

# 86. Provenance

Each generated artifact should make its provenance inspectable.

Example:

```text
Visualization IR v3

Generated by:
visualization.plan

Inputs:
Algorithm IR v1
Trace IR v1

Model:
...

Validated:
yes
```

---

# 87. Session Reproducibility

To reproduce a session, the system should ideally preserve:

```text
original input
source code
sample input
IR versions
runtime version
tool versions
schema versions
configuration
template version
model metadata
```

Perfect AI reproducibility may not always be possible.

Deterministic portions should remain reproducible.

---

# 88. Failure Recovery

If a session stops unexpectedly:

```text
AnimationCompilation
        ↓
process crashes
```

After restart:

```text
load session
 ↓
find last valid artifact
 ↓
resume from AnimationCompilation
```

Do not regenerate earlier valid stages unnecessarily.

---

# 89. Stage Checkpointing

Persist session state after meaningful stages.

Recommended checkpoints:

```text
Algorithm IR generated
Trace IR generated
Visualization IR generated
Animation IR generated
Export completed
```

Avoid persisting only at the very end.

---

# 90. Transactional Updates

When possible:

```text
write artifact
 ↓
validate artifact
 ↓
persist metadata
 ↓
activate artifact
```

Do not activate partially written artifacts.

---

# 91. Atomic Activation

Changing the active artifact should be atomic.

Example:

```text
active_animation:
v3 → v4
```

Readers should never see an incomplete transition.

---

# 92. Invalid Artifacts

Invalid artifacts may be retained for debugging.

However, they must never become active production artifacts.

Example:

```text
animation_v4
status = invalid
```

Active remains:

```text
animation_v3
```

---

# 93. Agent Repair

When validation fails:

```text
invalid Visualization IR
       ↓
store invalid artifact
       ↓
store validation report
       ↓
Agent repair
       ↓
new Visualization IR version
```

Do not destroy the failed artifact before repair.

It may be useful for debugging.

---

# 94. Example Full Session

```text
Create Session
   ↓
Store user problem
   ↓
problem.parse
   ↓
Algorithm IR v1
   ↓
code.compile
   ↓
code.trace
   ↓
Trace IR v1
   ↓
visualization.apply_template
   ↓
Visualization IR v1
   ↓
validation fails
   ↓
Visualization IR v2
   ↓
validation succeeds
   ↓
animation.compile
   ↓
Animation IR v1
   ↓
render.preview
   ↓
User: "Move explanation to the bottom"
   ↓
Visualization IR v3
   ↓
Animation IR v2
   ↓
Preview v2
   ↓
render.export
   ↓
MP4
   ↓
Completed
```

---

# 95. Example Session JSON

```json
{
  "id": "session_01JXYZ",
  "status": "active",
  "current_stage": "preview",

  "input": {
    "language": "rust"
  },

  "active_artifacts": {
    "algorithm_ir": "algorithm_001",
    "trace_ir": "trace_001",
    "visualization_ir": "visualization_003",
    "animation_ir": "animation_002"
  },

  "warnings": [],

  "created_at": "2026-01-01T10:00:00Z",
  "updated_at": "2026-01-01T10:05:00Z"
}
```

---

# 96. Rust Model Example

A possible Rust model:

```rust
pub struct Session {
    pub id: SessionId,
    pub status: SessionStatus,
    pub current_stage: SessionStage,

    pub input: SessionInput,
    pub context: SessionContext,

    pub active_artifacts: ActiveArtifacts,

    pub warnings: Vec<SessionWarning>,
    pub errors: Vec<SessionError>,
}
```

Statuses:

```rust
pub enum SessionStatus {
    Created,
    Active,
    Processing,
    Ready,
    Completed,
    Failed,
    Cancelled,
}
```

Stages:

```rust
pub enum SessionStage {
    Input,
    ProblemAnalysis,
    CodeAnalysis,
    Compilation,
    Execution,
    TraceGeneration,
    VisualizationPlanning,
    AnimationCompilation,
    Validation,
    Preview,
    Export,
    Completed,
}
```

---

# 97. Recommended Modules

```text
src/
├── session.rs
└── session/
    ├── model.rs
    ├── state.rs
    ├── artifact.rs
    ├── revision.rs
    ├── event.rs
    ├── store.rs
    └── context.rs
```

Possible responsibilities:

```text
model.rs
    core session model

artifact.rs
    artifact references and metadata

revision.rs
    revision history

event.rs
    session event log

store.rs
    persistence abstraction

context.rs
    Agent context builder
```

---

# 98. Relationship with ARCHITECTURE.md

`ARCHITECTURE.md` defines:

> What system components exist.

`SESSION.md` defines:

> How one piece of user work moves through those components over time.

---

# 99. Relationship with IR.md

`IR.md` defines:

> What Algorithm / Trace / Visualization / Animation data look like.

`SESSION.md` defines:

> Which versions of those IRs belong to the current workflow.

---

# 100. Relationship with TOOLS.md

`TOOLS.md` defines:

> What operations the Agent can invoke.

`SESSION.md` records:

> Which operations were invoked and which artifacts they produced.

---

# 101. Relationship with SANDBOX.md

`SANDBOX.md` defines:

> How untrusted code executes safely.

`SESSION.md` tracks:

```text
which sandbox job
used which source
used which input
produced which Trace IR
```

---

# 102. Relationship with EXCEPTION.md

`EXCEPTION.md` defines error behavior.

`SESSION.md` records:

```text
which error occurred
at which stage
on which artifact/tool call
whether the session can continue
```

---

# 103. MVP Session Model

For the first version, a session only needs:

```text
id
status
stage

input

algorithm_ir
trace_ir
visualization_ir
animation_ir

preview
export

errors
warnings
```

Do not build a complex distributed session system before the core pipeline works.

---

# 104. MVP Persistence

Recommended MVP:

```text
session.json
+
versioned IR files
+
output files
```

Example:

```text
sessions/
└── abc123/
    ├── session.json
    ├── algorithm-v1.json
    ├── trace-v1.json
    ├── visualization-v1.json
    ├── visualization-v2.json
    ├── animation-v1.json
    ├── preview.png
    └── output.mp4
```

This is sufficient for initial development.

---

# 105. Future Extensions

Future versions may add:

```text
multi-user sessions
cloud persistence
collaborative editing
session branching
distributed execution
remote workers
automatic checkpoints
undo/redo
session sharing
artifact comparison
```

These should extend the model without changing its basic principles.

---

# 106. Core Session Principle

A session should make it possible to answer:

```text
What did the user ask for?

What stage are we at?

What did the Agent decide?

Which tools were called?

Which code was executed?

Which Trace IR was produced?

Which Visualization IR is active?

Which Animation IR generated this preview?

Why did something fail?

What can be reused if the user changes one thing?
```

If the system cannot answer these questions, the session model is insufficient.

---

# 107. Final Principle

Do not treat an Agent session as:

```text
chat messages
+
some temporary variables
```

Treat it as:

```text
User Intent
     ↓
Versioned State
     ↓
Tool Execution
     ↓
Validated Artifacts
     ↓
Revisions
     ↓
Final Output
```

The session is the coordination and persistence boundary of the entire OJ animation generation workflow.
