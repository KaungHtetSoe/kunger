//! The single safe abstraction every provider uses to run external
//! commands. Enforces the constraints in `docs/SECURITY.md`: no shell
//! interpolation (arguments are always passed as a vector, never
//! interpolated into a shell string), a timeout on every invocation, and a
//! hard cap on how much output is read into memory. See
//! `docs/ARCHITECTURE.md` §2.7.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// A command to run, with its arguments already split into a vector — never
/// a single interpolated shell string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunError {
    #[error("command `{0}` was not found on PATH")]
    NotFound(String),
    #[error("command `{0}` timed out after {1:?}")]
    TimedOut(String, Duration),
    #[error("command `{0}` exited with status {1}: {2}")]
    NonZeroExit(String, i32, String),
    #[error("command `{0}` output exceeded the {1} byte limit")]
    OutputTooLarge(String, usize),
    #[error("failed to run `{0}`: {1}")]
    SpawnFailed(String, String),
}

/// Executes [`CommandSpec`]s with a timeout and an output size cap. Every
/// provider that shells out must go through this type rather than calling
/// `tokio::process::Command` directly, so the security constraints in
/// `docs/SECURITY.md` are enforced in exactly one place.
#[derive(Debug, Clone, Copy)]
pub struct ProcessRunner {
    pub timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for ProcessRunner {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(15),
            max_output_bytes: 8 * 1024 * 1024,
        }
    }
}

impl ProcessRunner {
    pub fn new(timeout: Duration, max_output_bytes: usize) -> Self {
        Self {
            timeout,
            max_output_bytes,
        }
    }

    /// Runs `spec` to completion, or fails with [`RunError::TimedOut`] if it
    /// does not finish within `self.timeout`. Note the timeout wraps both
    /// reading output *and* waiting for exit, as one bound — a process that
    /// keeps writing past `max_output_bytes` is only guaranteed to be
    /// killed once this single timeout elapses, not immediately upon
    /// exceeding the cap; callers needing tighter responsiveness should use
    /// a shorter `timeout`.
    ///
    /// A non-zero exit status is treated as a failure ([`RunError::NonZeroExit`]).
    /// For commands where that isn't true — e.g. `dpkg -S` exits non-zero
    /// when *any* queried path is unowned, even while still printing valid
    /// results for the owned ones on stdout — use
    /// [`Self::run_allow_any_exit`] instead.
    pub async fn run(&self, spec: &CommandSpec) -> Result<CommandOutput, RunError> {
        let output = self.run_raw(spec).await?;

        if !output.exit_code.map(|code| code == 0).unwrap_or(false) {
            return Err(RunError::NonZeroExit(
                spec.program.clone(),
                output.exit_code.unwrap_or(-1),
                output.stderr,
            ));
        }

        Ok(output)
    }

    /// Like [`Self::run`], but returns the output regardless of exit code —
    /// only a genuine execution problem (not found, timed out, spawn
    /// failure, output too large) is an error. Use this for commands whose
    /// exit code encodes a result (e.g. "no match") rather than
    /// "execution failed."
    pub async fn run_allow_any_exit(&self, spec: &CommandSpec) -> Result<CommandOutput, RunError> {
        self.run_raw(spec).await
    }

    async fn run_raw(&self, spec: &CommandSpec) -> Result<CommandOutput, RunError> {
        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command
            .spawn()
            .map_err(|error| map_spawn_error(&spec.program, &error))?;

        let stdout = child.stdout.take().ok_or_else(|| {
            RunError::SpawnFailed(spec.program.clone(), "missing stdout pipe".into())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            RunError::SpawnFailed(spec.program.clone(), "missing stderr pipe".into())
        })?;

        let max = self.max_output_bytes;

        let run = async {
            let (stdout_result, stderr_result) =
                tokio::join!(read_limited(stdout, max), read_limited(stderr, max));
            let status = child.wait().await;
            (stdout_result, stderr_result, status)
        };

        let (stdout_result, stderr_result, status_result) =
            match tokio::time::timeout(self.timeout, run).await {
                Ok(triple) => triple,
                Err(_elapsed) => {
                    return Err(RunError::TimedOut(spec.program.clone(), self.timeout));
                }
            };

        let (stdout, stdout_truncated) = stdout_result
            .map_err(|error| RunError::SpawnFailed(spec.program.clone(), error.to_string()))?;
        let (stderr, stderr_truncated) = stderr_result
            .map_err(|error| RunError::SpawnFailed(spec.program.clone(), error.to_string()))?;

        if stdout_truncated || stderr_truncated {
            return Err(RunError::OutputTooLarge(spec.program.clone(), max));
        }

        let status = status_result
            .map_err(|error| RunError::SpawnFailed(spec.program.clone(), error.to_string()))?;

        Ok(CommandOutput {
            stdout,
            stderr,
            exit_code: status.code(),
        })
    }
}

fn map_spawn_error(program: &str, error: &std::io::Error) -> RunError {
    if error.kind() == std::io::ErrorKind::NotFound {
        RunError::NotFound(program.to_string())
    } else {
        RunError::SpawnFailed(program.to_string(), error.to_string())
    }
}

/// Reads `reader` to EOF or until `limit` bytes have been read, whichever
/// comes first. Returns the decoded (lossily, for safety against non-UTF-8
/// output) text plus whether the limit was hit.
async fn read_limited<R>(mut reader: R, limit: usize) -> Result<(String, bool), std::io::Error>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; 8192];

    loop {
        let read = reader.read(&mut chunk).await?;
        if read == 0 {
            return Ok((String::from_utf8_lossy(&buf).into_owned(), false));
        }

        if buf.len() + read > limit {
            let remaining = limit.saturating_sub(buf.len());
            buf.extend_from_slice(&chunk[..remaining]);
            return Ok((String::from_utf8_lossy(&buf).into_owned(), true));
        }

        buf.extend_from_slice(&chunk[..read]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runs_a_simple_command_and_captures_stdout() {
        let runner = ProcessRunner::default();
        let spec = CommandSpec::new("echo").arg("hello kunger");

        let output = runner.run(&spec).await.expect("echo should succeed");

        assert_eq!(output.stdout.trim(), "hello kunger");
        assert_eq!(output.exit_code, Some(0));
    }

    #[tokio::test]
    async fn arguments_are_never_shell_interpreted() {
        // If this were passed through a shell, `; echo pwned` would run as
        // a second command. Passed as a single argument vector element, it
        // must come back verbatim as one argument to `echo`.
        let runner = ProcessRunner::default();
        let spec = CommandSpec::new("echo").arg("safe; echo pwned");

        let output = runner.run(&spec).await.expect("echo should succeed");

        assert_eq!(output.stdout.trim(), "safe; echo pwned");
    }

    #[tokio::test]
    async fn missing_command_reports_not_found() {
        let runner = ProcessRunner::default();
        let spec = CommandSpec::new("kunger-definitely-not-a-real-command-xyz");

        let error = runner.run(&spec).await.unwrap_err();

        assert_eq!(
            error,
            RunError::NotFound("kunger-definitely-not-a-real-command-xyz".to_string())
        );
    }

    #[tokio::test]
    async fn non_zero_exit_is_reported_as_an_error() {
        let runner = ProcessRunner::default();
        let spec = CommandSpec::new("sh").args(["-c", "exit 3"]);

        let error = runner.run(&spec).await.unwrap_err();

        match error {
            RunError::NonZeroExit(_, code, _) => assert_eq!(code, 3),
            other => panic!("expected NonZeroExit, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn slow_command_times_out() {
        let runner = ProcessRunner::new(Duration::from_millis(50), 8 * 1024 * 1024);
        let spec = CommandSpec::new("sleep").arg("5");

        let error = runner.run(&spec).await.unwrap_err();

        assert!(matches!(error, RunError::TimedOut(_, _)));
    }

    #[tokio::test]
    async fn output_larger_than_the_limit_is_rejected() {
        let runner = ProcessRunner::new(Duration::from_secs(5), 10);
        // `yes` repeats "y\n" forever; head caps it, but the point here is
        // exercising our own byte cap independent of the child's behavior.
        let spec = CommandSpec::new("sh").args(["-c", "head -c 1000 /dev/zero"]);

        let error = runner.run(&spec).await.unwrap_err();

        assert!(matches!(error, RunError::OutputTooLarge(_, 10)));
    }

    #[tokio::test]
    async fn run_allow_any_exit_returns_output_even_on_non_zero_exit() {
        let runner = ProcessRunner::default();
        let spec = CommandSpec::new("sh").args(["-c", "echo partial-output; exit 3"]);

        let output = runner
            .run_allow_any_exit(&spec)
            .await
            .expect("should not error on non-zero exit");

        assert_eq!(output.stdout.trim(), "partial-output");
        assert_eq!(output.exit_code, Some(3));
    }

    #[tokio::test]
    async fn run_allow_any_exit_still_reports_a_missing_command() {
        let runner = ProcessRunner::default();
        let spec = CommandSpec::new("kunger-definitely-not-a-real-command-xyz");

        let error = runner.run_allow_any_exit(&spec).await.unwrap_err();

        assert_eq!(
            error,
            RunError::NotFound("kunger-definitely-not-a-real-command-xyz".to_string())
        );
    }
}
