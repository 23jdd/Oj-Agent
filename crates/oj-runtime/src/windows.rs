//! A capability-free AppContainer is the isolation boundary. Nanosandbox supplies
//! nested Job Object memory/wall limits; its Windows backend alone is insufficient.
use super::*;
use nanosandbox::{MB, Sandbox};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    os::windows::{
        ffi::OsStrExt,
        io::{FromRawHandle, RawHandle},
    },
    path::{Path, PathBuf},
    process::Command,
    ptr::{null, null_mut},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::*,
    Security::{Authorization::*, Isolation::*, *},
    Storage::FileSystem::*,
    System::{JobObjects::*, Pipes::*, Threading::*},
};

fn wide(value: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}
fn failure(stage: &str) -> Failure {
    Failure::new(
        "SANDBOX_UNAVAILABLE",
        stage,
        std::io::Error::last_os_error().to_string(),
    )
}
fn io(e: impl std::fmt::Display) -> Failure {
    Failure::new("IO_FAILED", "Execution", e.to_string())
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                CloseHandle(self.0);
            }
        }
    }
}
fn is_container() -> bool {
    unsafe {
        let mut token = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        let token = Handle(token);
        let mut is_container = 0u32;
        let mut len = 0;
        GetTokenInformation(
            token.0,
            TokenIsAppContainer,
            &mut is_container as *mut _ as _,
            4,
            &mut len,
        ) != 0
            && is_container != 0
    }
}
struct Profile {
    name: Vec<u16>,
    sid: PSID,
}
impl Drop for Profile {
    fn drop(&mut self) {
        unsafe {
            DeleteAppContainerProfile(self.name.as_ptr());
            FreeSid(self.sid);
        }
    }
}
struct AclGrant {
    path: Vec<u16>,
    descriptor: PSECURITY_DESCRIPTOR,
    original: *mut ACL,
}
impl Drop for AclGrant {
    fn drop(&mut self) {
        unsafe {
            SetNamedSecurityInfoW(
                self.path.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                null_mut(),
                null_mut(),
                self.original,
                null(),
            );
            LocalFree(self.descriptor);
        }
    }
}
fn grant(path: &Path, sid: PSID, write: bool) -> Result<AclGrant> {
    unsafe {
        let path = wide(path);
        let mut old = null_mut();
        let mut descriptor = null_mut();
        if GetNamedSecurityInfoW(
            path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            &mut old,
            null_mut(),
            &mut descriptor,
        ) != 0
        {
            return Err(failure("SandboxACL"));
        }
        let restore = AclGrant {
            path,
            descriptor,
            original: old,
        };
        let access = EXPLICIT_ACCESS_W {
            grfAccessPermissions: if write {
                FILE_ALL_ACCESS
            } else {
                FILE_GENERIC_READ | FILE_GENERIC_EXECUTE
            },
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT as u32,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: TRUSTEE_IS_UNKNOWN,
                ptstrName: sid as _,
            },
        };
        let mut acl = null_mut();
        if SetEntriesInAclW(1, &access, old, &mut acl) != 0 {
            return Err(failure("SandboxACL"));
        }
        let result = SetNamedSecurityInfoW(
            restore.path.as_ptr(),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            acl,
            null(),
        );
        LocalFree(acl as _);
        if result != 0 {
            return Err(failure("SandboxACL"));
        }
        Ok(restore)
    }
}
fn low_integrity(path: &Path) -> Result<()> {
    unsafe {
        let mut descriptor = null_mut();
        let sddl = wide("S:(ML;;NW;;;LW)");
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            null_mut(),
        ) == 0
        {
            return Err(failure("SandboxLabel"));
        }
        let mut present = 0;
        let mut defaulted = 0;
        let mut sacl = null_mut();
        GetSecurityDescriptorSacl(descriptor, &mut present, &mut sacl, &mut defaulted);
        let result = SetNamedSecurityInfoW(
            wide(path).as_ptr(),
            SE_FILE_OBJECT,
            LABEL_SECURITY_INFORMATION,
            null_mut(),
            null_mut(),
            null(),
            sacl,
        );
        LocalFree(descriptor);
        if result != 0 {
            return Err(failure("SandboxLabel"));
        }
        Ok(())
    }
}
fn locate(name: &str) -> Result<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|dir| dir.join(name))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            Failure::new(
                "TOOLCHAIN_MISSING",
                "Compilation",
                format!("Install {name} or configure the toolchain on PATH."),
            )
        })
}
fn discover(language: &str) -> Result<BTreeMap<String, String>> {
    let mut tools = BTreeMap::new();
    let (key, name) = match language {
        "python" => ("python", "python.exe"),
        "cpp" => ("cpp", "g++.exe"),
        "rust" => ("rust", "rustc.exe"),
        _ => {
            return Err(Failure::new(
                "CODE_LANGUAGE_UNSUPPORTED",
                "Compilation",
                "Supported languages: Python, C++, Rust.",
            ));
        }
    };
    let mut program = locate(name)?;
    if language == "rust" {
        let result = Command::new("rustup.exe")
            .args(["which", "rustc"])
            .output()
            .map_err(io)?;
        if result.status.success() {
            program = PathBuf::from(String::from_utf8_lossy(&result.stdout).trim());
        }
        // MSVC's linker/SDK are discovered by rustc from the installed toolchain.
    }
    tools.insert(key.into(), program.to_string_lossy().into());
    Ok(tools)
}
fn cache() -> Result<PathBuf> {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| io("LOCALAPPDATA is unavailable"))?
        .join("OjAgent/runtime-artifacts");
    fs::create_dir_all(&root).map_err(io)?;
    Ok(root)
}
fn valid_id(id: &str) -> bool {
    id.len() == 64 && id.bytes().all(|c| c.is_ascii_hexdigit())
}

pub fn handle(mut request: Request) -> Result<Value> {
    if is_container() {
        return inner(request);
    }
    if request.operation == "trace" && request.artifact_id.is_empty() {
        request.instrument = true;
    }
    // ACL changes are scoped and restored; serialize overlapping toolchain grants.
    let mutex = unsafe {
        Handle(CreateMutexW(
            null(),
            0,
            wide("Local\\OjAgentSandboxAcl").as_ptr(),
        ))
    };
    if mutex.0.is_null() || unsafe { WaitForSingleObject(mutex.0, 60_000) } > WAIT_ABANDONED {
        return Err(failure("SandboxLock"));
    }
    struct Unlock(HANDLE);
    impl Drop for Unlock {
        fn drop(&mut self) {
            unsafe {
                ReleaseMutex(self.0);
            }
        }
    }
    let _unlock = Unlock(mutex.0);
    let job = tempfile::tempdir().map_err(io)?;
    let mut cached = None;
    if !request.artifact_id.is_empty() {
        if !valid_id(&request.artifact_id) {
            return Err(Failure::new(
                "EXECUTION_ARTIFACT_INVALID",
                "Execution",
                "Invalid artifact ID.",
            ));
        }
        let source = cache()?.join(&request.artifact_id);
        let metadata: Request =
            serde_json::from_slice(&fs::read(source.join("metadata.json")).map_err(io)?)
                .map_err(io)?;
        request.language = metadata.language;
        request.source = metadata.source;
        request.instrument = metadata.instrument;
        if source.join("program.exe").is_file() {
            fs::copy(source.join("program.exe"), job.path().join("program.exe")).map_err(io)?;
        }
        cached = Some(request.artifact_id.clone());
    }
    request.tools = if request.operation == "probe" {
        BTreeMap::new()
    } else {
        discover(&request.language)?
    };
    request.work_dir = job.path().to_string_lossy().into();
    let name = wide(format!(
        "OjAgent.Job.{}.{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io)?
            .as_nanos()
    ));
    let mut sid = null_mut();
    if unsafe {
        CreateAppContainerProfile(
            name.as_ptr(),
            name.as_ptr(),
            name.as_ptr(),
            null(),
            0,
            &mut sid,
        )
    } < 0
    {
        return Err(failure("AppContainerCreate"));
    }
    let profile = Profile { name, sid };
    let mut grants = vec![grant(job.path(), profile.sid, true)?];
    low_integrity(job.path())?;
    let worker = job.path().join("oj-runtime.exe");
    fs::copy(std::env::current_exe().map_err(io)?, &worker).map_err(io)?;
    for program in request.tools.values() {
        let tool = PathBuf::from(program);
        let root = tool
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| io("Invalid toolchain path"))?;
        grants.push(grant(root, profile.sid, false)?);
    }
    let output = run_container(
        &worker,
        job.path(),
        profile.sid,
        &serde_json::to_vec(&request).map_err(io)?,
    )?;
    drop(grants);
    drop(profile);
    if output["ok"] != true {
        return Err(serde_json::from_value(output["error"].clone()).map_err(io)?);
    }
    let mut data = output["data"].clone();
    if request.operation == "compile" {
        let tool = request.tools.values().next().cloned().unwrap_or_default();
        let id = format!(
            "{:x}",
            Sha256::digest(format!(
                "win-appcontainer-v2:{}:{}:{}:{tool}",
                request.language, request.instrument, request.source
            ))
        );
        let destination = cache()?.join(&id);
        fs::create_dir_all(&destination).map_err(io)?;
        if job.path().join("program.exe").is_file() {
            let file = job.path().join("program.exe");
            if fs::symlink_metadata(&file)
                .map_err(io)?
                .file_type()
                .is_symlink()
            {
                return Err(io("Compiler output is a reparse point"));
            }
            fs::copy(file, destination.join("program.exe")).map_err(io)?;
        }
        fs::write(
            destination.join("metadata.json"),
            serde_json::to_vec(&request).map_err(io)?,
        )
        .map_err(io)?;
        data["artifact_id"] = json!(id);
    } else if let Some(id) = cached {
        data["artifact_id"] = json!(id);
    }
    Ok(data)
}

fn run_container(program: &Path, dir: &Path, sid: PSID, input: &[u8]) -> Result<Value> {
    unsafe {
        let mut sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: null_mut(),
            bInheritHandle: 1,
        };
        let (
            mut input_read,
            mut input_write,
            mut output_read,
            mut output_write,
            mut error_read,
            mut error_write,
        ) = (
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
        );
        if CreatePipe(&mut input_read, &mut input_write, &mut sa, 0) == 0
            || CreatePipe(&mut output_read, &mut output_write, &mut sa, 0) == 0
            || CreatePipe(&mut error_read, &mut error_write, &mut sa, 0) == 0
        {
            return Err(failure("AppContainerPipes"));
        }
        for handle in [input_write, output_read, error_read] {
            SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0);
        }
        let input_read = Handle(input_read);
        let output_write = Handle(output_write);
        let error_write = Handle(error_write);
        let mut size = 0;
        InitializeProcThreadAttributeList(null_mut(), 2, 0, &mut size);
        let mut attributes = vec![0usize; size.div_ceil(std::mem::size_of::<usize>())];
        let list = attributes.as_mut_ptr() as _;
        if InitializeProcThreadAttributeList(list, 2, 0, &mut size) == 0 {
            return Err(failure("AppContainerAttributes"));
        }
        let capabilities = SECURITY_CAPABILITIES {
            AppContainerSid: sid,
            Capabilities: null_mut(),
            CapabilityCount: 0,
            Reserved: 0,
        };
        let handles = [input_read.0, output_write.0, error_write.0];
        if UpdateProcThreadAttribute(
            list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            &capabilities as *const _ as _,
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
            null_mut(),
            null(),
        ) == 0
            || UpdateProcThreadAttribute(
                list,
                0,
                PROC_THREAD_ATTRIBUTE_HANDLE_LIST as usize,
                handles.as_ptr() as _,
                std::mem::size_of_val(&handles),
                null_mut(),
                null(),
            ) == 0
        {
            DeleteProcThreadAttributeList(list);
            return Err(failure("AppContainerAttributes"));
        }
        let mut startup: STARTUPINFOEXW = std::mem::zeroed();
        startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
        startup.lpAttributeList = list;
        startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        startup.StartupInfo.hStdInput = input_read.0;
        startup.StartupInfo.hStdOutput = output_write.0;
        startup.StartupInfo.hStdError = error_write.0;
        let system = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let environment = wide(format!(
            "PATH={system}\\System32\0SystemRoot={system}\0TEMP={}\0TMP={}\0",
            dir.display(),
            dir.display()
        ));
        let mut command = wide(format!("\"{}\"", program.display()));
        let mut process: PROCESS_INFORMATION = std::mem::zeroed();
        let created = CreateProcessW(
            wide(program).as_ptr(),
            command.as_mut_ptr(),
            null(),
            null(),
            1,
            EXTENDED_STARTUPINFO_PRESENT
                | CREATE_UNICODE_ENVIRONMENT
                | CREATE_SUSPENDED
                | CREATE_NO_WINDOW,
            environment.as_ptr() as _,
            wide(dir).as_ptr(),
            &startup.StartupInfo,
            &mut process,
        );
        DeleteProcThreadAttributeList(list);
        if created == 0 {
            return Err(failure("AppContainerLaunch"));
        }
        let process_handle = Handle(process.hProcess);
        let thread_handle = Handle(process.hThread);
        let job = Handle(CreateJobObjectW(null(), null()));
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
            | JOB_OBJECT_LIMIT_JOB_MEMORY
            | JOB_OBJECT_LIMIT_JOB_TIME;
        limits.BasicLimitInformation.ActiveProcessLimit = 32;
        limits.BasicLimitInformation.PerJobUserTimeLimit = 30 * 10_000_000;
        limits.JobMemoryLimit = 1024 * 1024 * 1024;
        if job.0.is_null()
            || SetInformationJobObject(
                job.0,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
            || AssignProcessToJobObject(job.0, process_handle.0) == 0
        {
            TerminateProcess(process_handle.0, 1);
            return Err(failure("AppContainerJob"));
        }
        ResumeThread(thread_handle.0);
        drop(input_read);
        drop(output_write);
        drop(error_write);
        let mut writer = fs::File::from_raw_handle(input_write as RawHandle);
        let payload = input.to_vec();
        let writer = std::thread::spawn(move || writer.write_all(&payload));
        let stdout = fs::File::from_raw_handle(output_read as RawHandle);
        let stderr = fs::File::from_raw_handle(error_read as RawHandle);
        let stdout = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        });
        let stderr = std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        });
        let start = Instant::now();
        while WaitForSingleObject(process_handle.0, 20) == WAIT_TIMEOUT {
            if start.elapsed() > Duration::from_secs(50) {
                TerminateJobObject(job.0, 1);
                break;
            }
        }
        let mut exit = 1;
        GetExitCodeProcess(process_handle.0, &mut exit);
        drop(job);
        let _ = writer.join();
        let output = stdout
            .join()
            .map_err(|_| io("Output reader failed"))?
            .map_err(io)?;
        let diagnostics = stderr
            .join()
            .map_err(|_| io("Error reader failed"))?
            .map_err(io)?;
        if exit != 0 {
            return Err(Failure::new(
                "EXECUTION_RUNTIME_FAILED",
                "AppContainer",
                format!(
                    "Worker exited {exit}: {}",
                    String::from_utf8_lossy(&diagnostics)
                ),
            ));
        }
        if output.len() > 16 * 1024 * 1024 {
            return Err(Failure::new(
                "EXECUTION_OUTPUT_LIMIT",
                "Execution",
                "Worker exceeded the output limit.",
            ));
        }
        serde_json::from_slice(&output).map_err(|e| {
            Failure::new(
                "WORKER_PROTOCOL_INVALID",
                "Execution",
                format!("{e}: {}", String::from_utf8_lossy(&diagnostics)),
            )
        })
    }
}

fn inner(request: Request) -> Result<Value> {
    if request.operation == "probe" {
        return Ok(
            json!({"platform":"windows","isolation":true,"appcontainer":true,"network":"no-capabilities","languages":["python","rust","cpp"]}),
        );
    }
    let job = PathBuf::from(&request.work_dir);
    std::env::set_current_dir(&job).map_err(io)?;
    let compiling = request.operation == "compile" || request.artifact_id.is_empty();
    if compiling {
        let source = if request.instrument && request.language == "rust" {
            instrument::rust(&request.source).map_err(io)?
        } else if request.instrument && request.language == "cpp" {
            format!("{}{}", instrument::CPP_PRELUDE, request.source)
        } else {
            request.source.clone()
        };
        fs::write(
            job.join(format!(
                "solution.{}",
                match request.language.as_str() {
                    "cpp" => "cpp",
                    "rust" => "rs",
                    _ => "py",
                }
            )),
            source,
        )
        .map_err(io)?;
    }
    let path = request
        .tools
        .values()
        .filter_map(|p| Path::new(p).parent())
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join(";")
        + ";C:\\Windows\\System32";
    let sb = Sandbox::builder()
        .host_network()
        .no_private_tmp()
        .clear_env(true)
        .working_dir(&job)
        .memory_limit(768 * MB)
        .wall_time_limit(Duration::from_secs(30))
        .env("SystemRoot", "C:\\Windows")
        .env("PATH", path)
        .env("TEMP", job.to_string_lossy())
        .env("TMP", job.to_string_lossy())
        .build()
        .map_err(io)?;
    let tool = request
        .tools
        .values()
        .next()
        .ok_or_else(|| io("Toolchain is missing"))?;
    let run = |program: &str,
               args: &[&str],
               input: &[u8],
               compile: bool|
     -> Result<nanosandbox::ExecutionResult> {
        let result = sb.run_with_input(program, args, Some(input)).map_err(io)?;
        if result.killed_by_timeout {
            return Err(Failure::new(
                "EXECUTION_TIMEOUT",
                "Execution",
                "Execution timed out.",
            ));
        }
        if result.output_truncated {
            return Err(Failure::new(
                "EXECUTION_OUTPUT_LIMIT",
                "Execution",
                "Execution output limit exceeded.",
            ));
        }
        if !result.success() {
            let mut error = Failure::new(
                if compile {
                    "EXECUTION_COMPILE_FAILED"
                } else {
                    "EXECUTION_RUNTIME_FAILED"
                },
                if compile { "Compilation" } else { "Execution" },
                &result.stderr,
            );
            error.context.insert("stderr".into(), result.stderr);
            return Err(error);
        }
        Ok(result)
    };
    if compiling {
        match request.language.as_str() {
            "cpp" => {
                run(
                    tool,
                    &[
                        "solution.cpp",
                        "-std=c++20",
                        "-O0",
                        "-g",
                        "-o",
                        "program.exe",
                    ],
                    b"",
                    true,
                )?;
            }
            "rust" => {
                run(
                    tool,
                    &["--edition=2021", "solution.rs", "-o", "program.exe"],
                    b"",
                    true,
                )?;
            }
            "python" => {
                run(
                    tool,
                    &[
                        "-I",
                        "-c",
                        "import ast; ast.parse(open('solution.py').read())",
                    ],
                    b"",
                    true,
                )?;
            }
            _ => return Err(io("Unsupported language")),
        }
    }
    if request.operation == "compile" {
        return Ok(
            json!({"compiled":true,"language":request.language,"instrument":request.instrument}),
        );
    }
    let traced = request.operation == "trace";
    let result = if request.language == "python" && traced {
        run(tool,&["-I","-c",instrument::PYTHON],json!({"source":request.source,"stdin":request.stdin,"values":request.values,"max_events":request.max_events}).to_string().as_bytes(),false)?
    } else if request.language == "python" {
        run(
            tool,
            &["-I", "solution.py"],
            request.stdin.as_bytes(),
            false,
        )?
    } else {
        run(".\\program.exe", &[], request.stdin.as_bytes(), false)?
    };
    let mut output = json!({"stdout":result.stdout,"stderr":result.stderr,"exit_code":result.exit_code,"duration_ms":result.duration.as_millis()});
    if traced {
        if request.language == "python" {
            let data: Value =
                serde_json::from_str(output["stdout"].as_str().unwrap_or("")).map_err(io)?;
            output["events"] = data["events"].clone();
            output["stdout"] = data["stdout"].clone();
        } else {
            let mut values = request.values.clone();
            let mut events = Vec::new();
            for line in result.stderr.lines() {
                let line_number = if let Some(number) = line.strip_prefix("@OJLINE|") {
                    number.parse::<usize>().unwrap_or(1)
                } else if let Some(raw) = line.strip_prefix("@OJVALUES|") {
                    values = serde_json::from_str(raw).map_err(io)?;
                    1
                } else {
                    continue;
                };
                if events.len() >= request.max_events {
                    return Err(Failure::new(
                        "TRACE_EVENT_LIMIT",
                        "TraceGeneration",
                        "Trace exceeded its event limit.",
                    ));
                }
                let seq = events.len();
                events.push(json!({"id":format!("event_{seq}"),"seq":seq,"kind":"read","entity":"values","indices":[],"values":values,"explanation":format!("source line {line_number}"),"state":{},"source":{"file":if request.language=="rust"{"solution.rs"}else{"solution.cpp"},"line":line_number,"function":"statement"}}));
            }
            output["events"] = json!(events);
        }
        if output["events"].as_array().is_none_or(|e| e.is_empty()) {
            return Err(Failure::new(
                "TRACE_EMPTY",
                "TraceGeneration",
                "No source observations were produced.",
            ));
        }
    }
    Ok(output)
}
