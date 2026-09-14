//! Runs `ssh <host> squeue ...` and turns the result into jobs.

use crate::slurm::{build_ssh_args, parse_squeue_output, Job};
use serde::Serialize;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FetchError {
    pub message: String,
}

/// The ssh program name. Resolved through PATH, so on Windows this finds
/// the built-in OpenSSH `ssh.exe`.
pub const SSH_PROGRAM: &str = "ssh";

pub async fn fetch_jobs(host: &str, user: &str) -> Result<Vec<Job>, FetchError> {
    fetch_jobs_with(SSH_PROGRAM, host, user).await
}

/// Same as `fetch_jobs` but with an explicit program, so tests can point it
/// at a fake ssh script.
pub async fn fetch_jobs_with(
    program: &str,
    host: &str,
    user: &str,
) -> Result<Vec<Job>, FetchError> {
    let mut cmd = Command::new(program);
    cmd.args(build_ssh_args(host, user))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    // `output()` drains stdout and stderr concurrently, so a chatty stderr
    // (banners, warnings) cannot fill the pipe and deadlock the child.
    let output = cmd.output().await.map_err(|e| FetchError {
        message: format!("Could not run {program}: {e}"),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            match output.status.code() {
                Some(code) => format!("ssh exited with status {code}"),
                None => "ssh was terminated by a signal".to_string(),
            }
        } else {
            stderr
        };
        return Err(FetchError { message });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_squeue_output(&stdout))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    /// Writes an executable shell script and returns its path.
    fn fake_ssh(dir: &std::path::Path, body: &str) -> PathBuf {
        let path = dir.join("fake-ssh");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[tokio::test]
    async fn success_parses_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = fake_ssh(
            dir.path(),
            "printf '1|p|n|RUNNING|1:00|1|node1|2:00|4|8G\\n2|p|m|PENDING|0:00|1|(Priority)|2:00|4|8G\\n'",
        );
        let jobs = fetch_jobs_with(ssh.to_str().unwrap(), "h", "u")
            .await
            .unwrap();
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].id, "1");
        assert_eq!(jobs[1].state, "PENDING");
    }

    #[tokio::test]
    async fn success_with_empty_stdout_is_no_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = fake_ssh(dir.path(), "exit 0");
        let jobs = fetch_jobs_with(ssh.to_str().unwrap(), "h", "u")
            .await
            .unwrap();
        assert!(jobs.is_empty());
    }

    #[tokio::test]
    async fn nonzero_exit_reports_trimmed_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = fake_ssh(
            dir.path(),
            "echo '  Permission denied (publickey).  ' >&2; exit 255",
        );
        let err = fetch_jobs_with(ssh.to_str().unwrap(), "h", "u")
            .await
            .unwrap_err();
        assert_eq!(err.message, "Permission denied (publickey).");
    }

    #[tokio::test]
    async fn nonzero_exit_with_empty_stderr_reports_status() {
        let dir = tempfile::tempdir().unwrap();
        let ssh = fake_ssh(dir.path(), "exit 3");
        let err = fetch_jobs_with(ssh.to_str().unwrap(), "h", "u")
            .await
            .unwrap_err();
        assert_eq!(err.message, "ssh exited with status 3");
    }

    #[tokio::test]
    async fn missing_program_is_reported() {
        let err = fetch_jobs_with("/nonexistent/dir/ssh", "h", "u")
            .await
            .unwrap_err();
        assert!(
            err.message
                .starts_with("Could not run /nonexistent/dir/ssh:"),
            "{}",
            err.message
        );
    }

    #[tokio::test]
    async fn passes_built_args_to_program() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("args.txt");
        let ssh = fake_ssh(
            dir.path(),
            &format!("printf '%s\\n' \"$@\" > '{}'", out.display()),
        );
        fetch_jobs_with(ssh.to_str().unwrap(), "myhost", "alice")
            .await
            .unwrap();
        let recorded: Vec<String> = std::fs::read_to_string(&out)
            .unwrap()
            .lines()
            .map(String::from)
            .collect();
        assert_eq!(recorded, build_ssh_args("myhost", "alice"));
    }

    #[tokio::test]
    async fn large_stderr_does_not_deadlock() {
        let dir = tempfile::tempdir().unwrap();
        // 256 KiB of stderr, well past a typical 64 KiB pipe buffer.
        let ssh = fake_ssh(
            dir.path(),
            "i=0; while [ $i -lt 4096 ]; do printf '%064d\\n' $i >&2; i=$((i+1)); done; exit 1",
        );
        let err = fetch_jobs_with(ssh.to_str().unwrap(), "h", "u")
            .await
            .unwrap_err();
        assert!(err.message.len() > 200_000);
    }
}
