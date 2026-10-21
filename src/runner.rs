//! Runs the phreeqc engine on a worker thread so the UI never blocks, with
//! Cancel, a wall-clock timeout and a resident-memory guard. The guard exists
//! because a malformed input once made the real engine grow past 7.5 GB.

use crate::phreeqc::{self, RunFiles};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30 * 60);
pub const DEFAULT_MEMORY_LIMIT: u64 = 4 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub timeout: Duration,
    pub memory_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            memory_bytes: DEFAULT_MEMORY_LIMIT,
        }
    }
}

#[derive(Debug)]
pub enum Outcome {
    Finished {
        success: bool,
        output_text: String,
        stderr_text: String,
    },
    Cancelled,
    TimedOut(Duration),
    MemoryExceeded(u64),
    Failed(String),
}

pub struct RunHandle {
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<AtomicBool>,
    receiver: Receiver<Outcome>,
    started: Instant,
    /// Run directory, for reading back SELECTED_OUTPUT files afterwards.
    pub work_dir: PathBuf,
}

impl RunHandle {
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    /// The finished outcome, once available.
    pub fn poll(&self) -> Option<Outcome> {
        match self.receiver.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Outcome::Failed("run thread stopped unexpectedly".into())),
        }
    }

    /// Kill the engine now; the worker then reports `Cancelled`.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
}

impl Drop for RunHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub fn start(engine: &Path, input_body: &str, database: &Path, work_dir: &Path, limits: Limits) -> RunHandle {
    let (sender, receiver) = mpsc::channel();
    let shared: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));
    let cancelled = Arc::new(AtomicBool::new(false));
    let handle = RunHandle {
        child: shared.clone(),
        cancelled: cancelled.clone(),
        receiver,
        started: Instant::now(),
        work_dir: work_dir.to_path_buf(),
    };

    let spawned = phreeqc::prepare_run(input_body, database, work_dir).and_then(|files| {
        let child = spawn_engine(engine, &files, work_dir)?;
        Ok((files, child))
    });
    let (files, child) = match spawned {
        Ok(v) => v,
        Err(e) => {
            let _ = sender.send(Outcome::Failed(format!("Could not start phreeqc: {e}")));
            return handle;
        }
    };
    let pid = child.id();
    *shared.lock().unwrap() = Some(child);

    std::thread::spawn(move || {
        let outcome = supervise(&shared, &cancelled, pid, &files, limits);
        let _ = sender.send(outcome);
    });
    handle
}

fn spawn_engine(engine: &Path, files: &RunFiles, work_dir: &Path) -> std::io::Result<Child> {
    const TEXT_FILE_BUSY: i32 = 26;
    let mut attempt = 0;
    loop {
        let stderr_file = std::fs::File::create(&files.stderr)?;
        // stderr goes to a file, not a pipe: the engine prints progress there
        // on long runs and a full pipe would stall it.
        let mut command = Command::new(engine);
        command
            .arg(&files.input)
            .arg(&files.output)
            .current_dir(work_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr_file);
        #[cfg(windows)]
        {
            // phreeqc.exe is a console program: without this, every run
            // flashes a console window.
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let result = command.spawn();
        // ETXTBSY: the executable was being written (e.g. replaced during an
        // update) while some thread forked; it clears within milliseconds.
        match result {
            Err(e) if cfg!(unix) && e.raw_os_error() == Some(TEXT_FILE_BUSY) && attempt < 10 => {
                attempt += 1;
                std::thread::sleep(Duration::from_millis(20));
            }
            other => return other,
        }
    }
}

fn supervise(shared: &Mutex<Option<Child>>, cancelled: &AtomicBool, pid: u32, files: &RunFiles, limits: Limits) -> Outcome {
    let started = Instant::now();
    let mut last_memory_check = Instant::now() - Duration::from_secs(1);
    loop {
        let status = {
            let mut guard = shared.lock().unwrap();
            let Some(child) = guard.as_mut() else {
                return Outcome::Failed("engine process lost".into());
            };
            match child.try_wait() {
                Ok(status) => status,
                Err(e) => return Outcome::Failed(format!("waiting for phreeqc failed: {e}")),
            }
        };
        if let Some(status) = status {
            if cancelled.load(Ordering::SeqCst) {
                return Outcome::Cancelled;
            }
            if status.code().is_none() {
                return Outcome::Failed("phreeqc stopped abnormally (killed by a signal)".into());
            }
            return Outcome::Finished {
                success: status.success(),
                output_text: std::fs::read_to_string(&files.output).unwrap_or_default(),
                stderr_text: std::fs::read_to_string(&files.stderr).unwrap_or_default(),
            };
        }
        if started.elapsed() > limits.timeout {
            kill(shared);
            return Outcome::TimedOut(limits.timeout);
        }
        if last_memory_check.elapsed() >= Duration::from_millis(500) {
            last_memory_check = Instant::now();
            if let Some(rss) = resident_bytes(pid) {
                if rss > limits.memory_bytes {
                    kill(shared);
                    return Outcome::MemoryExceeded(rss);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn kill(shared: &Mutex<Option<Child>>) {
    if let Some(child) = shared.lock().unwrap().as_mut() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(target_os = "linux")]
fn resident_bytes(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let line = status.lines().find(|l| l.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

#[cfg(windows)]
fn resident_bytes(pid: u32) -> Option<u64> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    // SAFETY: the handle is checked before use and closed afterwards; the
    // counters struct is plain data sized for the call.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        let ok = GetProcessMemoryInfo(handle, &mut counters, size);
        CloseHandle(handle);
        (ok != 0).then_some(counters.WorkingSetSize as u64)
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
fn resident_bytes(pid: u32) -> Option<u64> {
    let out = Command::new("/bin/ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    let kb: u64 = String::from_utf8_lossy(&out.stdout).trim().parse().ok()?;
    Some(kb * 1024)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pq-studio-runner-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A stand-in "engine": a shell script with the given body.
    fn fake_engine(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("fake-phreeqc");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    fn wait(handle: &RunHandle) -> Outcome {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(o) = handle.poll() {
                return o;
            }
            assert!(Instant::now() < deadline, "runner did not finish");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn finishes_and_reads_output_from_work_dir() {
        let dir = scratch("ok");
        // $2 is the output path; cwd must be the work dir.
        let engine = fake_engine(&dir, "echo done > \"$2\"; echo note >&2; pwd > cwd.txt");
        let h = start(&engine, "SOLUTION 1\nEND\n", Path::new("/db.dat"), &dir, Limits::default());
        match wait(&h) {
            Outcome::Finished { success, output_text, stderr_text } => {
                assert!(success);
                assert_eq!(output_text.trim(), "done");
                assert_eq!(stderr_text.trim(), "note");
            }
            other => panic!("unexpected {other:?}"),
        }
        let cwd = std::fs::read_to_string(dir.join("cwd.txt")).unwrap();
        assert_eq!(std::fs::canonicalize(cwd.trim()).unwrap(), std::fs::canonicalize(&dir).unwrap());
        assert!(std::fs::read_to_string(dir.join("scratch.pqi")).unwrap().starts_with("DATABASE /db.dat\n"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cancel_kills_the_engine() {
        let dir = scratch("cancel");
        let engine = fake_engine(&dir, "exec sleep 30");
        let h = start(&engine, "", Path::new("/db.dat"), &dir, Limits::default());
        std::thread::sleep(Duration::from_millis(200));
        h.cancel();
        assert!(matches!(wait(&h), Outcome::Cancelled));
        assert!(h.elapsed() < Duration::from_secs(5));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn timeout_kills_the_engine() {
        let dir = scratch("timeout");
        let engine = fake_engine(&dir, "exec sleep 30");
        let limits = Limits { timeout: Duration::from_millis(300), ..Limits::default() };
        let h = start(&engine, "", Path::new("/db.dat"), &dir, limits);
        assert!(matches!(wait(&h), Outcome::TimedOut(_)));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn memory_guard_kills_the_engine() {
        let dir = scratch("memory");
        // Holds ~200 MB resident; the limit is 50 MB.
        let engine = fake_engine(
            &dir,
            "exec python3 -I -c 'import time; b = bytearray(200*1024*1024); time.sleep(30)'",
        );
        let limits = Limits { memory_bytes: 50 * 1024 * 1024, ..Limits::default() };
        let h = start(&engine, "", Path::new("/db.dat"), &dir, limits);
        match wait(&h) {
            Outcome::MemoryExceeded(rss) => assert!(rss > 50 * 1024 * 1024),
            other => panic!("unexpected {other:?}"),
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn database_templates_run_in_real_engine_and_round_trip() {
        use crate::model::{Block, DATABASE_KEYWORDS, Document, Raw, Solution};
        let Some(engine) = phreeqc::find_engine() else {
            eprintln!("phreeqc not installed; skipping");
            return;
        };
        let Some(database) = phreeqc::find_database(Some(&engine)) else {
            eprintln!("phreeqc.dat not found; skipping");
            return;
        };
        // One run per keyword: PHREEQC refuses PITZER and SIT in the same run.
        for (_, keyword, template) in DATABASE_KEYWORDS {
            let doc = Document {
                title: format!("{keyword} template"),
                blocks: vec![
                    Block::Solution(Solution::default()),
                    Block::Raw(Raw {
                        header_line: keyword.to_string(),
                        body: template.to_string(),
                    }),
                ],
            };
            let text = crate::serialize::document_to_pqi(&doc);
            let reopened = crate::io::parse_document(&text);
            assert_eq!(crate::serialize::document_to_pqi(&reopened), text, "{keyword} round-trip");

            let dir = scratch(&format!("template-{keyword}"));
            let limits = Limits { timeout: Duration::from_secs(60), ..Limits::default() };
            let h = start(&engine, &text, &database, &dir, limits);
            match wait(&h) {
                Outcome::Finished { success, output_text, stderr_text } => {
                    assert!(success, "{keyword}: {}", phreeqc::summarize_errors(&stderr_text));
                    assert!(output_text.contains("End of simulation"), "{keyword}");
                }
                other => panic!("{keyword}: unexpected {other:?}"),
            }
            std::fs::remove_dir_all(dir).unwrap();
        }
    }

    #[test]
    fn missing_engine_reports_failure() {
        let dir = scratch("missing");
        let h = start(Path::new("/nonexistent/phreeqc"), "", Path::new("/db.dat"), &dir, Limits::default());
        assert!(matches!(wait(&h), Outcome::Failed(_)));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
