//! Trusted absolute-path command resolution (sudo fallback helpers).
//! Never accepts free-form shell. Only /usr/bin /bin /usr/sbin /sbin.

use std::path::{Path, PathBuf};
use std::process::Command;

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

    pub fn resolve(self) -> Result<PathBuf, AppError> {
        resolve_trusted(self.binary_name())
    }
}

/// Caps for external process execution (timeouts / stdout / kill_on_drop).
#[derive(Debug, Clone, Copy)]
pub struct ExecutionPolicy {
    pub timeout: std::time::Duration,
    pub max_stdout_bytes: usize,
    pub kill_on_drop: bool,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(30),
            max_stdout_bytes: 2 * 1024 * 1024,
            kill_on_drop: true,
        }
    }
}

impl ExecutionPolicy {
    pub fn short() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(5),
            ..Self::default()
        }
    }
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
    Err(AppError::Internal(format!(
        "trusted binary not found: {binary}"
    )))
}

/// Abstract command runner for tests (no real sudo).
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &Path, args: &[&str]) -> Result<std::process::Output, AppError>;
}

pub struct RealCommandRunner;

impl CommandRunner for RealCommandRunner {
    fn run(&self, program: &Path, args: &[&str]) -> Result<std::process::Output, AppError> {
        Command::new(program)
            .args(args)
            .output()
            .map_err(|e| AppError::Internal(format!("spawn {}: {e}", program.display())))
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
    let sudo = resolve_trusted("sudo")?;
    let systemctl = resolve_trusted("systemctl")?;
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
}
