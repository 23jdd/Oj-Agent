# EXCEPTION.md

# Error and Exception Handling

## 1. Purpose

This document defines how errors are represented, propagated, recovered from, logged, and presented in the OJ Resolve Animation Agent.

The system contains multiple independent subsystems:

- user input
- AI agents
- parsers
- code execution
- trace generation
- visualization planning
- animation compilation
- rendering
- export

Errors must retain enough context to identify which stage failed and why.

---

# 2. Core Principle

Errors should be:

- explicit
- structured
- contextual
- recoverable where possible
- understandable by developers
- understandable by users when surfaced

Never silently ignore meaningful errors.

Never convert every error into a generic:

```text
Something went wrong.
```

---

# 3. Error Model

All application errors should share a common structure.

Conceptually:

```text
AppError {
    code
    kind
    stage
    message
    source?
    context
    recoverable
}
```

Example:

```json
{
  "code": "TRACE_INVALID_ENTITY",
  "kind": "Validation",
  "stage": "TraceValidation",
  "message": "Trace references unknown entity 'left_ptr'",
  "recoverable": true
}
```

---

# 4. Error Categories

Use stable categories.

```text
Input
Parse
Validation
Agent
ModelProvider
Execution
Trace
Visualization
Animation
Render
Export
IO
Configuration
Internal
Cancelled
Timeout
```

Do not create a unique error category for every individual failure.

---

# 5. Stage Information

Every pipeline error should identify its stage.

Recommended stages:

```text
InputNormalization

ProblemAnalysis
CodeAnalysis

AlgorithmModelGeneration

Execution
Instrumentation
TraceGeneration
TraceValidation

VisualizationPlanning
VisualizationValidation

AnimationCompilation
AnimationValidation

Rendering
Preview

Encoding
Export
```

This makes failures easy to diagnose.

---

# 6. Error Codes

Errors should use stable machine-readable codes.

Examples:

```text
INPUT_EMPTY

PROBLEM_PARSE_FAILED
CODE_LANGUAGE_UNSUPPORTED

AGENT_OUTPUT_INVALID
AGENT_RESPONSE_EMPTY
AGENT_REPAIR_FAILED

MODEL_RATE_LIMITED
MODEL_TIMEOUT
MODEL_UNAVAILABLE

EXECUTION_COMPILE_FAILED
EXECUTION_RUNTIME_FAILED
EXECUTION_TIMEOUT

TRACE_INVALID
TRACE_UNKNOWN_ENTITY
TRACE_INVALID_EVENT

VISUALIZATION_INVALID
VISUALIZATION_UNKNOWN_TARGET

ANIMATION_INVALID
ANIMATION_DUPLICATE_ID
ANIMATION_INVALID_DURATION

RENDER_FONT_MISSING
RENDER_RESOURCE_FAILED

EXPORT_ENCODER_FAILED
EXPORT_PATH_INVALID

IO_READ_FAILED
IO_WRITE_FAILED
```

Codes should remain stable even if human-readable messages change.

---

# 7. User Errors vs System Errors

Errors should be separated into:

## User-caused

Examples:

```text
empty input
invalid source code
unsupported language
invalid custom test case
```

These should produce clear actionable messages.

Example:

```text
The provided C++ source code could not be compiled.
Check the syntax or select the correct language.
```

---

## System-caused

Examples:

```text
model provider unavailable
renderer crash
invalid internal IR
encoder failure
```

These should preserve debugging information internally while presenting a concise message to users.

---

# 8. AI Output Errors

LLM output must never be assumed valid.

Possible failures:

```text
invalid JSON
missing required fields
unknown enum values
invalid IDs
references to nonexistent objects
hallucinated variables
inconsistent algorithm description
```

Flow:

```text
Model Output
    ↓
Parse
    ↓
Validate
    ↓
Valid?
 ┌──┴──┐
Yes   No
 │      │
Use   Repair
        ↓
     Validate
        ↓
      Accept
       or
      Fail
```

---

# 9. AI Repair Strategy

If structured AI output is invalid:

1. keep the original response
2. collect validation errors
3. send only relevant errors back to the model
4. request corrected structured output
5. validate again

Do not repeatedly regenerate from scratch unless necessary.

Recommended maximum automatic repair attempts:

```text
2
```

After repair attempts are exhausted, return an explicit Agent error.

---

# 10. LLM Retry Rules

Retry only failures that may be temporary.

Retryable:

```text
timeout
connection failure
rate limiting
temporary provider error
HTTP 5xx
```

Normally not retryable:

```text
authentication failure
invalid API key
invalid request
unsupported model
schema validation failure
```

Schema validation should use the repair flow instead.

---

# 11. Retry Policy

Use bounded retries.

Example:

```text
Attempt 1
↓
short backoff
Attempt 2
↓
longer backoff
Attempt 3
↓
fail
```

Do not retry indefinitely.

Recommended:

```text
maximum network attempts: 3
```

Use exponential backoff where appropriate.

---

# 12. Timeout

Every external or potentially unbounded operation should have a timeout.

Examples:

```text
LLM request
source code execution
compilation
renderer operation
video encoding
network access
```

Timeouts should produce explicit errors.

Example:

```text
EXECUTION_TIMEOUT
```

not:

```text
EXECUTION_FAILED
```

---

# 13. Code Execution Safety

OJ source code must be treated as untrusted.

Execution should occur in a sandbox.

Restrictions should include:

```text
CPU time
memory
filesystem access
network access
process spawning
output size
execution duration
```

Never execute arbitrary submitted code directly in the main application process.

---

# 14. Compile Errors

Compile failures should preserve compiler output.

Internal error:

```text
ExecutionError {
    code: EXECUTION_COMPILE_FAILED
    compiler_output
    language
}
```

User-facing message should include useful compiler diagnostics when appropriate.

---

# 15. Runtime Errors

Runtime failures include:

```text
panic
exception
segmentation fault
invalid memory access
divide by zero
assertion failure
```

The runtime adapter should convert these into a common error representation.

Example:

```text
EXECUTION_RUNTIME_FAILED
```

with:

```text
exit_code
stderr
source_location?
```

---

# 16. Execution Output Limits

Execution must limit:

```text
stdout
stderr
trace size
event count
```

If limits are exceeded, terminate execution.

Examples:

```text
EXECUTION_OUTPUT_LIMIT
TRACE_EVENT_LIMIT
```

This prevents infinite loops or huge traces from exhausting memory.

---

# 17. Trace Validation Errors

Trace events must be validated before visualization.

Examples:

```text
unknown entity
invalid index
invalid graph node
event referencing deleted object
invalid stack operation
invalid call-return pairing
```

Do not pass invalid traces into visualization.

---

# 18. Trace Recovery

Some trace errors may be recoverable.

Examples:

```text
missing optional source location
unknown display label
nonessential metadata absent
```

Recover by supplying defaults.

Do not recover from semantic inconsistencies such as:

```text
swap indexes outside array bounds
visit nonexistent graph node
pop empty stack
```

unless the trace explicitly represents an algorithm error.

---

# 19. Visualization Errors

Visualization planning errors include:

```text
unknown visual component
unsupported algorithm visualization
layout constraint failure
reference to missing trace entity
```

Fallback strategies may be used.

Example:

```text
Specialized Tree Visualization
            ↓ fails
Generic Event Visualization
```

---

# 20. Visualization Fallback

When specialized visualization is unavailable:

```text
specialized visualization
        ↓
generic data structure view
        ↓
generic trace view
        ↓
text explanation
```

The system should prefer degraded output over total failure when meaningful output is still possible.

---

# 21. Animation Validation

Before rendering, validate:

```text
unique object IDs
valid references
valid durations
valid timestamps
nonnegative sizes
valid timeline ordering
supported actions
valid object lifecycle
```

Never allow malformed animation data to reach the renderer.

---

# 22. Invalid Duration

Invalid:

```text
duration < 0
NaN
Infinity
```

These must fail validation.

Zero duration may be allowed only for instantaneous state updates.

---

# 23. Object Lifecycle Errors

Example invalid sequence:

```text
Delete object A
↓
Move object A
```

This should produce:

```text
ANIMATION_INVALID_OBJECT_LIFECYCLE
```

Likewise:

```text
Create A
Create A
```

should fail unless replacement semantics are explicitly supported.

---

# 24. Renderer Errors

Rendering errors should not affect algorithm or trace state.

Typical errors:

```text
missing font
image decode failed
GPU initialization failed
invalid texture
resource allocation failed
```

Where possible, use fallbacks.

Example:

```text
Preferred Font
   ↓ unavailable
Fallback Font
```

---

# 25. Renderer Fallback

If GPU rendering fails and a software renderer exists:

```text
GPU Renderer
    ↓ failure
Software Renderer
```

Fallback should be explicit in logs.

Do not silently change renderers without recording the event.

---

# 26. Export Errors

Export can fail independently from preview.

Examples:

```text
FFmpeg unavailable
encoder initialization failed
invalid output directory
permission denied
disk full
```

A failed export must not invalidate a successfully generated animation.

The user should still be able to preview or save the Animation IR.

---

# 27. Partial Success

The system should support partial success.

Example:

```text
Problem analysis       ✓
Trace generation       ✓
Animation generation   ✓
Preview                ✓
MP4 export             ✗
```

Result:

```text
Generation succeeded.
MP4 export failed.
```

Do not report the entire generation as failed.

---

# 28. Recoverable Errors

Examples:

```text
temporary API failure
missing optional font
unsupported specialized visualization
invalid first AI response
```

These may use:

```text
retry
repair
fallback
default
```

---

# 29. Fatal Errors

Examples:

```text
corrupted internal state
invalid core Animation IR after validation
missing required configuration
sandbox unavailable when executing untrusted code
```

Fatal errors should stop the relevant job immediately.

---

# 30. Error Propagation

Errors should be handled at the layer that can make a meaningful decision.

Lower layers should provide context.

Higher layers decide:

```text
retry?
repair?
fallback?
abort?
show user?
```

Do not handle an error too early if the layer cannot recover meaningfully.

---

# 31. Context Wrapping

When propagating errors, preserve the original cause.

Example:

```text
IO error:
permission denied
```

wrapped as:

```text
Failed to export MP4 to output/demo.mp4:
permission denied
```

Do not replace the original error.

---

# 32. Logging

Logs should contain useful context.

Example:

```text
generation_id=abc123
stage=VisualizationPlanning
error=VISUALIZATION_UNKNOWN_TARGET
target=left_pointer
```

Avoid logs such as:

```text
error happened
```

---

# 33. Sensitive Data

Do not log:

```text
API keys
authentication tokens
passwords
private environment variables
```

Be cautious when logging user-submitted source code.

Debug logging may include source snippets only when explicitly allowed.

---

# 34. Error Reports

A developer-facing error report should ideally contain:

```text
generation ID
timestamp
pipeline stage
error code
message
source cause
model/provider if relevant
input artifact IDs
retry count
repair count
```

---

# 35. User-Facing Errors

User-facing errors should answer:

1. What failed?
2. Why, if known?
3. Can the user do anything?
4. Was any partial result preserved?

Example:

```text
The animation was generated successfully,
but MP4 export failed because FFmpeg was not found.

You can still preview the animation or export it as GIF.
```

---

# 36. Internal Errors

Unexpected internal conditions should be classified as:

```text
INTERNAL_ERROR
```

but internal logs must preserve the underlying details.

Do not expose stack traces to normal users by default.

---

# 37. Cancellation

User cancellation is not a failure.

Represent it separately:

```text
CANCELLED
```

When cancellation occurs:

- stop unnecessary downstream work
- release resources
- preserve completed reusable artifacts when safe
- do not display alarming error UI

---

# 38. Job State

Generation jobs should have explicit states.

```text
Pending
Analyzing
Tracing
Planning
CompilingAnimation
Rendering
Exporting
Completed
Failed
Cancelled
```

Optional:

```text
CompletedWithWarnings
```

This makes partial failure easier to represent.

---

# 39. Warnings

Not every issue should become an error.

Examples:

```text
fallback font used
specialized visualization unavailable
trace metadata incomplete
video export skipped
```

Warnings should not stop generation.

---

# 40. Panic / Unhandled Exception

Unexpected panics or exceptions should be caught at process or job boundaries where possible.

The system should:

1. record the error
2. mark the job failed
3. release resources
4. avoid corrupting other jobs
5. keep the main service alive when possible

---

# 41. Resource Cleanup

Failures must not leak:

```text
temporary files
child processes
sandbox containers
GPU resources
open file handles
encoder processes
```

Cleanup must happen on:

```text
success
failure
timeout
cancellation
```

---

# 42. Temporary Files

Temporary resources should belong to a generation job.

Example:

```text
tmp/
└── generation-abc123/
```

The job owns this directory.

Cleanup should be easy and deterministic.

---

# 43. Testing Error Paths

Error handling must be tested.

Tests should include:

```text
invalid model JSON
model timeout
compiler failure
runtime timeout
infinite loop
trace validation failure
missing font
renderer failure
missing FFmpeg
disk write error
user cancellation
```

Successful-path testing alone is insufficient.

---

# 44. Error Injection

Where useful, development builds may support controlled error injection.

Examples:

```text
simulate model timeout
simulate render failure
simulate encoder failure
```

This helps test fallback and recovery logic.

---

# 45. Agent Behavior on Failure

The Agent must not hide failures by inventing data.

For example:

If execution tracing fails, the Agent must not silently fabricate an exact runtime trace.

It may instead generate:

```text
conceptual visualization
```

if clearly marked as conceptual.

The system must distinguish:

```text
Observed execution trace
```

from:

```text
AI-generated conceptual trace
```

---

# 46. Trust Levels

Generated artifacts may optionally carry trust metadata.

Example:

```text
trace_source:
    runtime

trace_confidence:
    verified
```

or:

```text
trace_source:
    model

trace_confidence:
    inferred
```

This prevents inferred runtime behavior from being treated as verified execution.

---

# 47. Fallback Order

Default recovery priority:

```text
Retry transient failure
        ↓
Repair invalid structured output
        ↓
Use deterministic fallback
        ↓
Use simpler visualization
        ↓
Return partial result
        ↓
Fail explicitly
```

Never silently produce incorrect output.

---

# 48. Final Rule

When choosing between:

```text
pretty but incorrect animation
```

and:

```text
simpler but correct animation
```

always choose the simpler correct animation.

Correctness takes priority over visual quality.

---

# Error Handling Principle

The system should aim for:

```text
failure
   ↓
understand
   ↓
recover if safe
   ↓
degrade if useful
   ↓
report clearly
```

not:

```text
failure
   ↓
ignore
   ↓
continue with invalid state
```