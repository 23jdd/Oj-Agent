use crate::{
    agent::check_cancelled,
    error::{AppError, Result},
};
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub struct Output {
    pub status: std::process::ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
/// Bounds output and lifetime, drains both pipes concurrently and always reaps the child.
pub fn run(
    command: &mut Command,
    input: Vec<u8>,
    timeout: Duration,
    limit: usize,
    cancel: &AtomicBool,
) -> Result<Output> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AppError::io("ProcessStart", e))?;
    let stdin = child.stdin.take();
    let writer = thread::spawn(move || {
        if let Some(mut pipe) = stdin {
            pipe.write_all(&input)
        } else {
            Ok(())
        }
    });
    let exceeded = Arc::new(AtomicBool::new(false));
    fn drain(
        pipe: impl Read + Send + 'static,
        limit: usize,
        exceeded: Arc<AtomicBool>,
    ) -> thread::JoinHandle<std::io::Result<Vec<u8>>> {
        thread::spawn(move || {
            let mut reader = pipe;
            let mut result = Vec::new();
            let mut chunk = [0; 8192];
            loop {
                let n = reader.read(&mut chunk)?;
                if n == 0 {
                    break;
                }
                if result.len() + n > limit {
                    exceeded.store(true, Ordering::Relaxed);
                }
                let take = n.min(limit.saturating_sub(result.len()));
                result.extend_from_slice(&chunk[..take]);
            }
            Ok(result)
        })
    }
    let stdout = drain(
        child.stdout.take().expect("stdout pipe"),
        limit,
        exceeded.clone(),
    );
    let stderr = drain(
        child.stderr.take().expect("stderr pipe"),
        limit,
        exceeded.clone(),
    );
    let start = Instant::now();
    let status = loop {
        let failure = if let Err(e) = check_cancelled(cancel) {
            Some(e)
        } else if exceeded.load(Ordering::Relaxed) {
            Some(AppError::new(
                "PROCESS_OUTPUT_LIMIT",
                "Execution",
                "Process",
                "Process output exceeded its limit.",
            ))
        } else if start.elapsed() > timeout {
            Some(AppError::new(
                "PROCESS_TIMEOUT",
                "Timeout",
                "Process",
                "Process exceeded its wall time limit.",
            ))
        } else {
            None
        };
        if let Some(error) = failure {
            let _ = child.kill();
            let _ = child.wait();
            let _ = writer.join();
            let _ = stdout.join();
            let _ = stderr.join();
            return Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::io("ProcessWait", e));
            }
        }
    };
    let _ = writer.join();
    let stdout = stdout
        .join()
        .map_err(|_| {
            AppError::new(
                "PROCESS_READER_FAILED",
                "Internal",
                "Process",
                "Output reader panicked.",
            )
        })?
        .map_err(|e| AppError::io("ProcessRead", e))?;
    let stderr = stderr
        .join()
        .map_err(|_| {
            AppError::new(
                "PROCESS_READER_FAILED",
                "Internal",
                "Process",
                "Error reader panicked.",
            )
        })?
        .map_err(|e| AppError::io("ProcessRead", e))?;
    if exceeded.load(Ordering::Relaxed) {
        return Err(AppError::new(
            "PROCESS_OUTPUT_LIMIT",
            "Execution",
            "Process",
            "Process output exceeded its limit.",
        ));
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}
