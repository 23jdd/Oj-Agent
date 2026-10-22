use super::*;
use nanosandbox::{MB, Permission, Sandbox};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

fn io(e: impl std::fmt::Display) -> Failure {
    Failure::new("IO_FAILED", "Execution", e.to_string())
}
fn rootfs() -> Result<PathBuf> {
    let path = std::env::var_os("OJ_SANDBOX_ROOTFS")
        .map(PathBuf::from)
        .ok_or_else(|| {
            Failure::new(
                "SANDBOX_ROOTFS_MISSING",
                "Execution",
                "Set OJ_SANDBOX_ROOTFS to a dedicated Linux toolchain root filesystem.",
            )
        })?;
    let path = path.canonicalize().map_err(io)?;
    if path == Path::new("/") {
        return Err(Failure::new(
            "SANDBOX_ROOTFS_INVALID",
            "Execution",
            "The host root filesystem cannot be a sandbox rootfs.",
        ));
    }
    Ok(path)
}
fn store() -> Result<PathBuf> {
    let path = std::env::var_os("OJ_RUNTIME_STORE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/tmp/oj-agent-artifacts"));
    fs::create_dir_all(&path).map_err(io)?;
    path.canonicalize().map_err(io)
}
fn sandbox(job: &Path, compiling: bool) -> Result<Sandbox> {
    let mut builder = Sandbox::builder()
        .rootfs(rootfs()?)
        .clear_env(true)
        .no_network()
        .memory_limit(if compiling { 768 * MB } else { 256 * MB })
        .cpu_time_limit(Duration::from_secs(if compiling { 20 } else { 3 }))
        .wall_time_limit(Duration::from_secs(if compiling { 30 } else { 5 }))
        .max_pids(32)
        .max_output(if compiling { MB } else { 8 * MB })
        .max_file_size(64 * MB)
        .max_open_files(128)
        .private_tmp(128 * MB)
        .working_dir("/job")
        .env("PATH", "/usr/bin:/bin")
        .env("LANG", "C.UTF-8");
    builder = builder.bind(
        job,
        "/job",
        if compiling {
            Permission::ReadWrite
        } else {
            Permission::ReadOnly
        },
    );
    builder
        .build()
        .map_err(|e| Failure::new("SANDBOX_UNAVAILABLE", "Execution", e.to_string()))
}
fn execute(
    sandbox: &Sandbox,
    program: &str,
    args: &[&str],
    stdin: &[u8],
    compiling: bool,
) -> Result<nanosandbox::ExecutionResult> {
    let result = sandbox
        .run_with_input(program, args, Some(stdin))
        .map_err(|e| Failure::new("EXECUTION_FAILED", "Execution", e.to_string()))?;
    let code = if result.killed_by_timeout || result.killed_by_cpu_limit {
        Some("EXECUTION_TIMEOUT")
    } else if result.output_truncated {
        Some("EXECUTION_OUTPUT_LIMIT")
    } else if result.killed_by_oom {
        Some("EXECUTION_MEMORY_LIMIT")
    } else if !result.success() {
        Some(if compiling {
            "EXECUTION_COMPILE_FAILED"
        } else {
            "EXECUTION_RUNTIME_FAILED"
        })
    } else {
        None
    };
    if let Some(code) = code {
        let mut error = Failure::new(
            code,
            if compiling {
                "Compilation"
            } else {
                "Execution"
            },
            result.failure_reason().unwrap_or_else(|| code.into()),
        );
        error.context.insert("stderr".into(), result.stderr);
        error.context.insert("stdout".into(), result.stdout);
        error
            .context
            .insert("exit_code".into(), result.exit_code.to_string());
        return Err(error);
    }
    Ok(result)
}
#[derive(Serialize, Deserialize)]
struct Compiled {
    language: String,
    instrument: bool,
    source: String,
}

fn compile(request: &Request) -> Result<Value> {
    if !["python", "cpp", "rust"].contains(&request.language.as_str()) {
        return Err(Failure::new(
            "CODE_LANGUAGE_UNSUPPORTED",
            "Compilation",
            "Supported languages: Python, Rust, C++.",
        ));
    }
    let id = format!(
        "{:x}",
        Sha256::digest(format!(
            "runtime-v2:{}:{}:{}",
            request.language, request.instrument, request.source
        ))
    );
    let base = store()?;
    let destination = base.join(&id);
    if destination.join("metadata.json").exists() {
        return Ok(json!({"artifact_id":id,"language":request.language,"cache_hit":true}));
    }
    let temp = tempfile::tempdir().map_err(io)?;
    // Root callers run sandbox children as nobody; only this per-job directory is writable.
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o777)).map_err(io)?;
    let source = if request.instrument && request.language == "rust" {
        instrument::rust(&request.source)
            .map_err(|e| Failure::new("EXECUTION_COMPILE_FAILED", "Compilation", e))?
    } else if request.instrument && request.language == "cpp" {
        format!("{}{}", instrument::CPP_PRELUDE, request.source)
    } else {
        request.source.clone()
    };
    let extension = match request.language.as_str() {
        "python" => "py",
        "cpp" => "cpp",
        _ => "rs",
    };
    fs::write(temp.path().join(format!("solution.{extension}")), source).map_err(io)?;
    let sb = sandbox(temp.path(), true)?;
    match request.language.as_str() {
        "cpp" => {
            let mut args = vec![
                "/job/solution.cpp",
                "-std=c++20",
                "-O0",
                "-g",
                "-fno-pie",
                "-no-pie",
                "-o",
                "/job/program",
            ];
            if request.instrument {
                fs::write(temp.path().join("hooks.cpp"), instrument::CPP_HOOKS).map_err(io)?;
                execute(
                    &sb,
                    "/usr/bin/g++",
                    &["-c", "/job/hooks.cpp", "-o", "/job/hooks.o"],
                    b"",
                    true,
                )?;
                args.extend([
                    "-finstrument-functions",
                    "-finstrument-functions-exclude-file-list=/usr/include",
                    "/job/hooks.o",
                ]);
            }
            execute(&sb, "/usr/bin/g++", &args, b"", true)?;
        }
        "rust" => {
            execute(
                &sb,
                "/usr/bin/rustc",
                &[
                    "--edition=2021",
                    "-g",
                    "-C",
                    "opt-level=0",
                    "/job/solution.rs",
                    "-o",
                    "/job/program",
                ],
                b"",
                true,
            )?;
        }
        _ => {
            execute(
                &sb,
                "/usr/bin/python3",
                &[
                    "-I",
                    "-c",
                    "import ast; ast.parse(open('/job/solution.py').read())",
                ],
                b"",
                true,
            )?;
        }
    }
    fs::write(
        temp.path().join("metadata.json"),
        serde_json::to_vec(&Compiled {
            language: request.language.clone(),
            instrument: request.instrument,
            source: request.source.clone(),
        })
        .map_err(io)?,
    )
    .map_err(io)?;
    // Publish only complete artifacts; competing identical compilations reuse the winner.
    match fs::rename(temp.path(), &destination) {
        Ok(()) => {}
        Err(_) if destination.join("metadata.json").exists() => {}
        Err(e) => return Err(io(e)),
    }
    Ok(json!({"artifact_id":id,"language":request.language,"cache_hit":false}))
}
pub fn handle(mut request: Request) -> Result<Value> {
    if request.operation == "compile" {
        return compile(&request);
    }
    if !["run", "trace"].contains(&request.operation.as_str()) {
        return Err(Failure::new(
            "TOOL_UNKNOWN",
            "Execution",
            "Unknown worker operation.",
        ));
    }
    if request.artifact_id.is_empty() {
        request.instrument = request.operation == "trace";
        request.artifact_id = compile(&request)?["artifact_id"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
    }
    if request.artifact_id.len() != 64
        || !request.artifact_id.bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err(Failure::new(
            "EXECUTION_ARTIFACT_INVALID",
            "Execution",
            "Invalid compiled artifact ID.",
        ));
    }
    let job = store()?.join(&request.artifact_id);
    let metadata: Compiled =
        serde_json::from_slice(&fs::read(job.join("metadata.json")).map_err(io)?).map_err(io)?;
    if request.operation == "trace" && !metadata.instrument {
        return Err(Failure::new(
            "TRACE_NOT_INSTRUMENTED",
            "TraceGeneration",
            "Compile with instrument=true before requesting a trace.",
        ));
    }
    let sb = sandbox(&job, false)?;
    let is_trace = request.operation == "trace";
    let result = if metadata.language == "python" && is_trace {
        let payload = json!({"source":metadata.source,"stdin":request.stdin,"values":request.values,"max_events":request.max_events});
        execute(
            &sb,
            "/usr/bin/python3",
            &["-I", "-c", instrument::PYTHON],
            payload.to_string().as_bytes(),
            false,
        )?
    } else if metadata.language == "python" {
        execute(
            &sb,
            "/usr/bin/python3",
            &["-I", "/job/solution.py"],
            request.stdin.as_bytes(),
            false,
        )?
    } else {
        execute(&sb, "/job/program", &[], request.stdin.as_bytes(), false)?
    };
    let mut output = json!({"stdout":result.stdout,"stderr":result.stderr,"exit_code":result.exit_code,"duration_ms":result.duration.as_millis(),"artifact_id":request.artifact_id});
    if is_trace {
        if metadata.language == "python" {
            let payload: Value = serde_json::from_str(output["stdout"].as_str().unwrap_or(""))
                .map_err(|e| Failure::new("TRACE_INVALID", "TraceGeneration", e.to_string()))?;
            output["stdout"] = payload["stdout"].clone();
            output["events"] = payload["events"].clone();
        } else {
            let stderr = output["stderr"].as_str().unwrap_or("");
            let mut events = Vec::new();
            let mut values = request.values.clone();
            let mut diagnostics = Vec::new();
            for line in stderr.lines() {
                let (line_number, function) = if let Some(raw) = line.strip_prefix("@OJLINE|") {
                    (raw.parse::<usize>().unwrap_or(1), "statement".to_string())
                } else if let Some(raw) = line.strip_prefix("@OJADDR|") {
                    let address = format!("0x{raw}");
                    let resolved = execute(
                        &sb,
                        "/usr/bin/addr2line",
                        &["-e", "/job/program", "-f", "-C", &address],
                        b"",
                        false,
                    )?;
                    let mut lines = resolved.stdout.lines();
                    let function = lines.next().unwrap_or("function").to_owned();
                    let location = lines.next().unwrap_or("solution.cpp:1");
                    let number = location
                        .rsplit(':')
                        .next()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1);
                    (number, function)
                } else if let Some(raw) = line.strip_prefix("@OJVALUES|") {
                    values = serde_json::from_str::<Vec<i64>>(raw).map_err(|e| {
                        Failure::new("TRACE_INVALID", "TraceGeneration", e.to_string())
                    })?;
                    if values.len() > 64 {
                        return Err(Failure::new(
                            "TRACE_EVENT_LIMIT",
                            "TraceGeneration",
                            "Observed arrays exceed 64 values.",
                        ));
                    }
                    (
                        events
                            .last()
                            .and_then(|v: &Value| v["source"]["line"].as_u64())
                            .unwrap_or(1) as usize,
                        "values".into(),
                    )
                } else {
                    diagnostics.push(line);
                    continue;
                };
                if events.len() >= request.max_events {
                    return Err(Failure::new(
                        "TRACE_EVENT_LIMIT",
                        "TraceGeneration",
                        "Trace exceeds event limit.",
                    ));
                }
                let seq = events.len();
                events.push(json!({"id":format!("event_{seq}"),"seq":seq,"kind":"read","entity":"values","indices":[],"values":values,"explanation":format!("{function}: line {line_number}"),"state":{},"source":{"file":if metadata.language=="rust"{"solution.rs"}else{"solution.cpp"},"line":line_number,"function":function}}));
            }
            output["events"] = json!(events);
            output["stderr"] = json!(diagnostics.join("\n"));
        }
        if output["events"]
            .as_array()
            .is_none_or(|events| events.is_empty())
        {
            return Err(Failure::new(
                "TRACE_EMPTY",
                "TraceGeneration",
                "No source observations were produced.",
            ));
        }
    }
    Ok(output)
}
