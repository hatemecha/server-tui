//! Trusted absolute-path command resolution and bounded execution.
//! Never accepts free-form shell. Only /usr/bin /bin /usr/sbin /sbin.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::error::AppError;
use crate::model::{ServiceActionKind, UnitRegistry};

const TRUSTED_DIRS: &[&str] = &["/usr/bin", "/bin", "/usr/sbin", "/sbin"];

/// Allowlisted binaries under trusted absolute dirs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustedCommand {
    Sudo,
    Systemctl,
    Timedatectl,
    Journalctl,
    Smartctl,
    Coredumpctl,
}

impl TrustedCommand {
    pub fn binary_name(self) -> &'static str {
        match self {
            Self::Sudo => "sudo",
            Self::Systemctl => "systemctl",
            Self::Timedatectl => "timedatectl",
            Self::Journalctl => "journalctl",
            Self::Smartctl => "smartctl",
            Self::Coredumpctl => "coredumpctl",
        }
    }

    /// Resolve under trusted directories. Missing optional tools → `Unsupported`.
    pub fn resolve(self) -> Result<PathBuf, AppError> {
        resolve_trusted(self.binary_name())
    }

    /// Soft resolve for optional probes (None when unavailable).
    pub fn try_resolve(self) -> Option<PathBuf> {
        self.resolve().ok()
    }
}

/// Caps for external process execution (timeouts / stdout / stderr / kill_on_drop).
#[derive(Debug, Clone, Copy)]
pub struct ExecutionPolicy {
    pub timeout: Duration,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
    pub kill_on_drop: bool,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_stdout_bytes: 2 * 1024 * 1024,
            max_stderr_bytes: 256 * 1024,
            kill_on_drop: true,
        }
    }
}

impl ExecutionPolicy {
    pub fn journalctl() -> Self {
        Self {
            timeout: Duration::from_secs(8),
            max_stdout_bytes: 2 * 1024 * 1024,
            max_stderr_bytes: 256 * 1024,
            kill_on_drop: true,
        }
    }

    pub fn coredumpctl() -> Self {
        Self {
            timeout: Duration::from_secs(8),
            max_stdout_bytes: 1024 * 1024,
            max_stderr_bytes: 256 * 1024,
            kill_on_drop: true,
        }
    }

    pub fn timedatectl() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            max_stdout_bytes: 64 * 1024,
            max_stderr_bytes: 64 * 1024,
            kill_on_drop: true,
        }
    }

    pub fn smartctl() -> Self {
        Self {
            timeout: Duration::from_secs(10),
            max_stdout_bytes: 512 * 1024,
            max_stderr_bytes: 128 * 1024,
            kill_on_drop: true,
        }
    }

    pub fn sudo_systemctl() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_stdout_bytes: 64 * 1024,
            max_stderr_bytes: 256 * 1024,
            kill_on_drop: true,
        }
    }

    /// Availability probes (`--version`) — tiny output, short timeout.
    pub fn probe() -> Self {
        Self {
            timeout: Duration::from_secs(3),
            max_stdout_bytes: 16 * 1024,
            max_stderr_bytes: 16 * 1024,
            kill_on_drop: true,
        }
    }
}

/// Successful oneshot capture (status may still be non-zero).
#[derive(Debug, Clone)]
pub struct CapturedOutput {
    pub status_code: Option<i32>,
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Resolve a binary name to an absolute path under trusted directories only.
pub fn resolve_trusted(binary: &str) -> Result<PathBuf, AppError> {
    if binary.is_empty()
        || binary.contains('/')
        || binary.contains('\0')
        || binary.chars().any(|c| c.is_whitespace())
    {
        return Err(AppError::Internal(format!(
            "refusing untrusted binary name: {binary}"
        )));
    }
    for dir in TRUSTED_DIRS {
        let candidate = Path::new(dir).join(binary);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(AppError::Unsupported(format!(
        "trusted binary not found: {binary}"
    )))
}

fn map_exit(status: &std::process::ExitStatus) -> (bool, Option<i32>) {
    (status.success(), status.code())
}

/// Bounded oneshot via tokio (trusted absolute path, no shell, stdin null).
pub async fn run_oneshot(
    program: &Path,
    args: &[&str],
    policy: ExecutionPolicy,
) -> Result<CapturedOutput, AppError> {
    use tokio::io::AsyncReadExt;
    use tokio::process::Command as TokioCommand;

    let mut cmd = TokioCommand::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(policy.kill_on_drop);

    let mut child = cmd
        .spawn()
        .map_err(|e| AppError::ExternalCommand(format!("spawn {}: {e}", program.display())))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::ExternalCommand("missing stdout pipe".into()))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| AppError::ExternalCommand("missing stderr pipe".into()))?;

    let max_out = policy.max_stdout_bytes;
    let max_err = policy.max_stderr_bytes;
    let collect = async {
        let mut out_buf = Vec::new();
        let mut err_buf = Vec::new();
        let mut out_chunk = [0u8; 8192];
        let mut err_chunk = [0u8; 8192];
        let mut out_done = false;
        let mut err_done = false;
        loop {
            tokio::select! {
                n = stdout.read(&mut out_chunk), if !out_done => {
                    match n {
                        Ok(0) => out_done = true,
                        Ok(n) => {
                            if out_buf.len() + n > max_out {
                                return Err(AppError::ExternalOutputLimit {
                                    stream: "stdout",
                                    limit_bytes: max_out,
                                });
                            }
                            out_buf.extend_from_slice(&out_chunk[..n]);
                        }
                        Err(e) => {
                            return Err(AppError::ExternalCommand(format!("stdout read: {e}")));
                        }
                    }
                }
                n = stderr.read(&mut err_chunk), if !err_done => {
                    match n {
                        Ok(0) => err_done = true,
                        Ok(n) => {
                            if err_buf.len() + n > max_err {
                                return Err(AppError::ExternalOutputLimit {
                                    stream: "stderr",
                                    limit_bytes: max_err,
                                });
                            }
                            err_buf.extend_from_slice(&err_chunk[..n]);
                        }
                        Err(e) => {
                            return Err(AppError::ExternalCommand(format!("stderr read: {e}")));
                        }
                    }
                }
                else => break,
            }
            if out_done && err_done {
                break;
            }
        }
        Ok((out_buf, err_buf))
    };

    let collected = match tokio::time::timeout(policy.timeout, collect).await {
        Ok(Ok(bufs)) => bufs,
        Ok(Err(e)) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(e);
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(AppError::ExternalTimeout {
                program: program.display().to_string(),
                timeout_secs: policy.timeout.as_secs(),
            });
        }
    };

    let status = match tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return Err(AppError::ExternalCommand(format!("wait failed: {e}")));
        }
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(AppError::ExternalTimeout {
                program: program.display().to_string(),
                timeout_secs: policy.timeout.as_secs(),
            });
        }
    };

    let (success, status_code) = map_exit(&status);
    Ok(CapturedOutput {
        status_code,
        success,
        stdout: collected.0,
        stderr: collected.1,
    })
}

/// Run a TrustedCommand oneshot with policy (argv fixed by caller).
pub async fn run_trusted(
    command: TrustedCommand,
    args: &[&str],
    policy: ExecutionPolicy,
) -> Result<CapturedOutput, AppError> {
    let path = command.resolve()?;
    run_oneshot(&path, args, policy).await
}

/// Soft oneshot: missing binary → None (capability unavailable).
pub async fn run_trusted_optional(
    command: TrustedCommand,
    args: &[&str],
    policy: ExecutionPolicy,
) -> Result<Option<CapturedOutput>, AppError> {
    let Some(path) = command.try_resolve() else {
        return Ok(None);
    };
    Ok(Some(run_oneshot(&path, args, policy).await?))
}

/// Abstract command runner for tests (no real sudo). Sync path for elevation.
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &Path, args: &[&str]) -> Result<std::process::Output, AppError>;
}

pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &Path, args: &[&str]) -> Result<std::process::Output, AppError> {
        use std::io::Read;

        let policy = ExecutionPolicy::sudo_systemctl();
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Internal(format!("spawn {}: {e}", program.display())))?;

        let mut stdout_pipe = child
            .stdout
            .take()
            .ok_or_else(|| AppError::Internal("missing stdout".into()))?;
        let mut stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| AppError::Internal("missing stderr".into()))?;

        let out_handle = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stdout_pipe.read_to_end(&mut buf);
            buf
        });
        let err_handle = std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = stderr_pipe.read_to_end(&mut buf);
            buf
        });

        let deadline = std::time::Instant::now() + policy.timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break s,
                Ok(None) if std::time::Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = out_handle.join();
                    let _ = err_handle.join();
                    return Err(AppError::ExternalTimeout {
                        program: program.display().to_string(),
                        timeout_secs: policy.timeout.as_secs(),
                    });
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(e) => {
                    return Err(AppError::Internal(format!(
                        "wait {}: {e}",
                        program.display()
                    )));
                }
            }
        };

        let stdout = out_handle.join().unwrap_or_default();
        let stderr = err_handle.join().unwrap_or_default();

        if stdout.len() > policy.max_stdout_bytes {
            return Err(AppError::ExternalOutputLimit {
                stream: "stdout",
                limit_bytes: policy.max_stdout_bytes,
            });
        }
        if stderr.len() > policy.max_stderr_bytes {
            return Err(AppError::ExternalOutputLimit {
                stream: "stderr",
                limit_bytes: policy.max_stderr_bytes,
            });
        }
        Ok(std::process::Output {
            status,
            stdout,
            stderr,
        })
    }
}

/// Recorded fake runner for unit tests.
#[derive(Default)]
pub struct FakeCommandRunner {
    pub calls: std::sync::Mutex<Vec<(PathBuf, Vec<String>)>>,
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CommandRunner for FakeCommandRunner {
    fn run(&self, program: &Path, args: &[&str]) -> Result<std::process::Output, AppError> {
        self.calls.lock().expect("lock").push((
            program.to_path_buf(),
            args.iter().map(|s| (*s).into()).collect(),
        ));
        Ok(std::process::Output {
            status: exit_status(self.exit_code),
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
        })
    }
}

#[cfg(unix)]
fn exit_status(code: i32) -> std::process::ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    // Linux wait status: exit code in high byte.
    std::process::ExitStatus::from_raw(if code == 0 { 0 } else { code << 8 })
}

#[cfg(not(unix))]
fn exit_status(_code: i32) -> std::process::ExitStatus {
    Command::new("true").status().unwrap()
}

/// Build argv for `systemctl <action> <unit>` under sudo -n (non-interactive).
/// Caller must enforce known-unit registry membership separately.
pub fn sudo_systemctl_args(
    action: ServiceActionKind,
    unit: &str,
) -> Result<(PathBuf, Vec<String>), AppError> {
    let sudo = TrustedCommand::Sudo.resolve()?;
    let systemctl = TrustedCommand::Systemctl.resolve()?;
    if !crate::providers::linux::systemd::unit_looks_safe(unit) {
        return Err(AppError::Systemd("refusing unsafe unit name".into()));
    }
    Ok((
        sudo,
        vec![
            "-n".into(),
            systemctl.to_string_lossy().into_owned(),
            action.label().into(),
            unit.into(),
        ],
    ))
}

/// Execute typed systemctl via sudo -n using a CommandRunner (tests use Fake).
/// Enforces lexical unit safety and known-unit registry (SECURITY.md).
pub fn run_sudo_systemctl(
    runner: &dyn CommandRunner,
    action: ServiceActionKind,
    unit: &str,
    registry: &UnitRegistry,
) -> Result<(), AppError> {
    if registry.is_empty() || !registry.contains(unit) {
        return Err(AppError::Systemd(format!(
            "refusing action on unregistered unit {unit}"
        )));
    }
    let (sudo, args) = sudo_systemctl_args(action, unit)?;
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = runner.run(&sudo, &arg_refs)?;
    if out.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        Err(AppError::Permission(format!(
            "sudo systemctl {} {unit} failed: {err}",
            action.label()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_injection() {
        assert!(resolve_trusted("../evil").is_err());
        assert!(resolve_trusted("sys ctl").is_err());
    }

    #[test]
    fn missing_binary_is_unsupported_not_internal() {
        let err = resolve_trusted("this-binary-does-not-exist-xyzzy").unwrap_err();
        assert!(matches!(err, AppError::Unsupported(_)));
    }

    #[test]
    fn fake_runner_records_call() {
        let fake = FakeCommandRunner {
            exit_code: 0,
            ..Default::default()
        };
        let unit = "ssh.service";
        assert!(crate::providers::linux::systemd::unit_looks_safe(unit));
        let systemctl = "/usr/bin/systemctl";
        let args = vec!["-n", systemctl, "restart", unit];
        let prog = PathBuf::from("/usr/bin/sudo");
        let _ = fake.run(&prog, &args);
        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, args);
    }

    #[test]
    fn sudo_args_reject_bad_unit() {
        assert!(sudo_systemctl_args(ServiceActionKind::Restart, "bad unit").is_err());
    }

    #[test]
    fn sudo_run_rejects_unregistered_unit() {
        let fake = FakeCommandRunner {
            exit_code: 0,
            ..Default::default()
        };
        let empty = UnitRegistry::new();
        let err = run_sudo_systemctl(&fake, ServiceActionKind::Restart, "ssh.service", &empty)
            .expect_err("empty registry");
        assert!(err.to_string().contains("unregistered"));

        let reg = UnitRegistry::new();
        reg.replace_all(["ssh.service".into()]);
        let err = run_sudo_systemctl(&fake, ServiceActionKind::Restart, "other.service", &reg)
            .expect_err("not in registry");
        assert!(err.to_string().contains("unregistered"));

        // Registered unit reaches the runner (Fake succeeds without real sudo).
        run_sudo_systemctl(&fake, ServiceActionKind::Restart, "ssh.service", &reg)
            .expect("registered unit");
        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].1.iter().any(|a| a == "ssh.service"));
        assert!(calls[0].1.iter().any(|a| a == "restart"));
    }

    #[test]
    fn execution_policy_presets_are_bounded() {
        let j = ExecutionPolicy::journalctl();
        assert!(j.timeout <= Duration::from_secs(10));
        assert_eq!(j.max_stdout_bytes, 2 * 1024 * 1024);
        assert_eq!(j.max_stderr_bytes, 256 * 1024);

        let t = ExecutionPolicy::timedatectl();
        assert!(t.max_stdout_bytes <= 64 * 1024);
    }

    #[tokio::test]
    async fn oneshot_respects_stdout_cap() {
        // Test-only: resolve an allowlisted helper. Production never passes user paths.
        let Ok(yes) = resolve_trusted("yes") else {
            // Skip when `yes` is absent from trusted dirs (unusual on Linux).
            return;
        };
        let policy = ExecutionPolicy {
            timeout: Duration::from_secs(3),
            max_stdout_bytes: 1024,
            max_stderr_bytes: 1024,
            kill_on_drop: true,
        };
        let err = run_oneshot(&yes, &[], policy)
            .await
            .expect_err("yes must exceed stdout cap");
        assert!(
            matches!(
                err,
                AppError::ExternalOutputLimit {
                    stream: "stdout",
                    ..
                }
            ),
            "got {err:?}"
        );
    }

    #[tokio::test]
    async fn oneshot_timeout_kills_child() {
        let Ok(sleep) = resolve_trusted("sleep") else {
            return;
        };
        let policy = ExecutionPolicy {
            timeout: Duration::from_millis(200),
            max_stdout_bytes: 1024,
            max_stderr_bytes: 1024,
            kill_on_drop: true,
        };
        let err = run_oneshot(&sleep, &["5"], policy)
            .await
            .expect_err("sleep must time out");
        assert!(
            matches!(err, AppError::ExternalTimeout { .. }),
            "got {err:?}"
        );
    }
}
