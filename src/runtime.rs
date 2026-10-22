use crate::{
    error::{AppError, Result},
    ir::*,
};
pub use oj_runtime::Request;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{process::Command, sync::atomic::AtomicBool, time::Duration};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u128,
}
pub struct SandboxRuntime;
impl SandboxRuntime {
    pub fn available() -> bool {
        cfg!(any(target_os = "linux", windows)) || std::env::var_os("OJ_WORKER_PROGRAM").is_some()
    }
    pub fn request(request: Request, cancel: &AtomicBool) -> Result<Value> {
        let payload = serde_json::to_vec(&request).map_err(|e| AppError::io("Execution", e))?;
        if let Some(program) = std::env::var_os("OJ_WORKER_PROGRAM") {
            let args: Vec<String> = std::env::var("OJ_WORKER_ARGS")
                .ok()
                .map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| {
                    AppError::new(
                        "WORKER_CONFIG_INVALID",
                        "Configuration",
                        "Execution",
                        e.to_string(),
                    )
                })?
                .unwrap_or_default();
            let mut command = Command::new(program);
            command.args(args);
            return invoke_worker(&mut command, payload, cancel);
        }
        #[cfg(windows)]
        if let Ok(distro) = std::env::var("OJ_WSL_DISTRO") {
            if distro.to_ascii_lowercase().contains("docker") {
                return Err(AppError::new(
                    "WORKER_CONFIG_INVALID",
                    "Configuration",
                    "Execution",
                    "Use an independent Linux distribution.",
                ));
            }
            let program = std::env::var("OJ_WSL_WORKER")
                .unwrap_or_else(|_| "/opt/oj-agent/oj-runtime".into());
            let mut command = Command::new("wsl.exe");
            command.args(["-d", &distro, "--exec", &program]);
            return invoke_worker(&mut command, payload, cancel);
        }
        #[cfg(target_os = "linux")]
        {
            oj_runtime::handle(request).map_err(|e| {
                AppError::new(&e.code, &e.kind, &e.stage, e.message).context(
                    "worker",
                    serde_json::to_string(&e.context).unwrap_or_default(),
                )
            })
        }
        #[cfg(windows)]
        {
            let program = std::env::current_exe()
                .map_err(|e| AppError::io("Execution", e))?
                .parent()
                .ok_or_else(|| {
                    AppError::validation("Execution", "Cannot locate worker directory.")
                })?
                .join("oj-runtime.exe");
            if !program.is_file() {
                return Err(AppError::new(
                    "SANDBOX_UNAVAILABLE",
                    "Configuration",
                    "Execution",
                    "Build the native worker with cargo build --workspace, then restart the application.",
                ));
            }
            invoke_worker(&mut Command::new(program), payload, cancel)
        }
        #[cfg(not(any(target_os = "linux", windows)))]
        {
            let _ = (payload, cancel);
            Err(AppError::new(
                "SANDBOX_UNAVAILABLE",
                "Configuration",
                "Execution",
                "Configure OJ_WSL_DISTRO and OJ_WSL_WORKER, or OJ_WORKER_PROGRAM plus JSON OJ_WORKER_ARGS for a native Linux worker.",
            ))
        }
    }
    pub fn run_python(source: &str, stdin: &str) -> Result<Execution> {
        Self::run("python", source, stdin, &AtomicBool::new(false))
    }
    pub fn run(
        language: &str,
        source: &str,
        stdin: &str,
        cancel: &AtomicBool,
    ) -> Result<Execution> {
        serde_json::from_value(Self::request(
            request("run", language, source, stdin),
            cancel,
        )?)
        .map_err(|e| AppError::validation("Execution", e.to_string()))
    }
    pub fn trace(input: &ProblemInput, cancel: &AtomicBool) -> Result<(AlgorithmIr, TraceIr)> {
        let mut request = request("trace", &input.language, &input.source_code, &input.stdin);
        request.values = input.values.clone();
        let result = Self::request(request, cancel)?;
        let mut algorithm = crate::trace::algorithm_model(input);
        algorithm.name = format!("{} · source observations", input.title);
        let events: Vec<TraceEvent> = serde_json::from_value(result["events"].clone())
            .map_err(|e| AppError::validation("TraceGeneration", e.to_string()))?;
        let trace = TraceIr {
            header: Header::new("trace-ir", "trace_runtime"),
            algorithm: algorithm.header.id.clone(),
            trace_source: "instrumentation".into(),
            confidence: "derived".into(),
            initial_state: std::collections::BTreeMap::from([(
                "values".into(),
                json!(input.values),
            )]),
            final_state: std::collections::BTreeMap::from([(
                "stdout".into(),
                result["stdout"].clone(),
            )]),
            events,
        };
        trace.validate(&algorithm)?;
        Ok((algorithm, trace))
    }
}
pub fn request(operation: &str, language: &str, source: &str, stdin: &str) -> Request {
    Request {
        operation: operation.into(),
        language: language.into(),
        source: source.into(),
        stdin: stdin.into(),
        artifact_id: String::new(),
        values: vec![],
        max_events: 1000,
        instrument: false,
        tools: Default::default(),
        work_dir: String::new(),
    }
}
fn invoke_worker(command: &mut Command, payload: Vec<u8>, cancel: &AtomicBool) -> Result<Value> {
    let output = crate::process::run(
        command,
        payload,
        Duration::from_secs(120),
        16 * 1024 * 1024,
        cancel,
    )?;
    if !output.status.success() {
        return Err(AppError::new(
            "WORKER_FAILED",
            "Execution",
            "Execution",
            String::from_utf8_lossy(&output.stderr),
        )
        .context("exit_code", output.status));
    }
    let response: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| AppError::validation("WorkerProtocol", e.to_string()))?;
    if response["ok"] == true {
        Ok(response["data"].clone())
    } else {
        Err(serde_json::from_value(response["error"].clone())
            .map_err(|e| AppError::validation("WorkerProtocol", e.to_string()))?)
    }
}
