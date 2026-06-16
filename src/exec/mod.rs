pub mod spawner;
pub mod watcher;
pub mod backgrounder;
pub mod signal;

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use regex::Regex;

use crate::core::SmolError;

const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const GRAY: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

/// Get terminal width, falling back to 80 on error or when not a tty.
fn terminal_width() -> usize {
    // Safety: ioctl TIOCGWINSZ is safe on all Unix platforms we support.
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 {
            ws.ws_col as usize
        } else {
            80
        }
    }
}

/// Count visible characters in a string, skipping ANSI escape sequences.
fn visible_len(s: &str) -> usize {
    let mut count = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            count += 1;
        }
    }
    count
}

/// Take up to `n` visible characters from `s`, preserving ANSI codes.
fn visible_chars_take(s: &str, n: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut visible = 0;
    let mut in_escape = false;

    for ch in s.chars() {
        out.push(ch);
        if in_escape {
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else {
            visible += 1;
            if visible >= n {
                break;
            }
        }
    }
    // Close any dangling escape sequence we may have interrupted
    if in_escape {
        out.push('\x1b');
        out.push('[');
        out.push('0');
        out.push('m');
    }
    out
}

/// Strip ANSI escape sequences and control characters that would break
/// the terminal line management (e.g. \r, \x1b[...m from colored output).
fn sanitize_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_escape = false;
    for ch in s.chars() {
        if in_escape {
            if ch.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else if ch == '\x1b' {
            in_escape = true;
        } else if ch == '\r' || ch == '\n' {
            // Carriage return / newline would scramble indicatif's line
        } else if (ch as u32) < 0x20 {
            // Skip other ASCII control characters
        } else {
            out.push(ch);
        }
    }
    out
}

const BUILD_COMMANDS: &[&str] = &[
    "cargo", "mvn", "make", "gcc", "g++", "clang", "clang++",
    "npm", "npx", "yarn", "pnpm", "gradle", "cmake", "ninja",
    "go", "rustc", "javac", "msbuild", "dotnet", "xcodebuild",
    "bazel", "scons", "rake", "ant", "pip", "python",
];

struct LiveState {
    build_detected: bool,
    build_errors: u32,
    build_warnings: u32,
    test_detected: bool,
    test_passed: u32,
    test_failed: u32,
    test_skipped: u32,
}

fn build_status_line(state: &LiveState) -> String {
    let mut parts = Vec::new();

    if state.build_detected {
        let mut inner = vec!["Build".to_string()];
        if state.build_errors > 0 {
            inner.push(format!("{}err:{}{}", RED, state.build_errors, RESET));
        }
        if state.build_warnings > 0 {
            inner.push(format!("{}warn:{}{}", YELLOW, state.build_warnings, RESET));
        }
        // Show build section even without errors/warnings to indicate build in progress
        parts.push(format!("[{}]", inner.join(" ")));
    }

    if state.test_detected {
        let mut inner = vec!["Test".to_string()];
        if state.test_passed > 0 {
            inner.push(format!("{}pass:{}{}", GREEN, state.test_passed, RESET));
        }
        if state.test_failed > 0 {
            inner.push(format!("{}fail:{}{}", RED, state.test_failed, RESET));
        }
        if state.test_skipped > 0 {
            inner.push(format!("{}skip:{}{}", YELLOW, state.test_skipped, RESET));
        }
        parts.push(format!("[{}]", inner.join(" ")));
    }

    parts.join(" ")
}

fn is_build_output(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("compiling")
        || lower.contains("building")
        || (lower.contains("build") && (lower.contains("success") || lower.contains("failure") || lower.contains("failed") || lower.contains("starting")))
        || lower.contains("linking")
        || lower.contains("generating")
}

fn detect_test_line(re_cargo: &Regex, re_maven: &Regex, re_jest: &Regex, line: &str) -> Option<(u32, u32, u32)> {
    // Maven — compute passed from total - failures - errors - skipped
    if let Some(caps) = re_maven.captures(line) {
        let total    = caps.name("total").and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(0);
        let failures = caps.name("failures").and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(0);
        let errors   = caps.name("errors").and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(0);
        let skipped  = caps.name("skipped").and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(0);
        let passed   = total.saturating_sub(failures).saturating_sub(errors).saturating_sub(skipped);
        // Failures + errors are both "failed" tests for display purposes
        return Some((passed, failures.saturating_add(errors), skipped));
    }
    // Cargo / Jest — standard passed / failed / skipped groups
    for re in &[re_cargo, re_jest] {
        if let Some(caps) = re.captures(line) {
            let passed = caps.name("passed").and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(0);
            let failed = caps.name("failed").or_else(|| caps.name("failures")).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(0);
            let skipped = caps.name("skipped").or_else(|| caps.name("errors")).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(0);
            return Some((passed, failed, skipped));
        }
    }
    None
}

/// Heuristic to tell whether a line belongs to test-runner output
/// (and should *not* be counted as a build error/warning).
fn is_test_output_line(line: &str, re_cargo: &Regex, re_maven: &Regex, re_jest: &Regex) -> bool {
    // Test summary lines
    if re_cargo.is_match(line) || re_maven.is_match(line) || re_jest.is_match(line) {
        return true;
    }
    // Maven surefire runner lines
    if line.contains("Running ") && line.contains("Test") {
        return true;
    }
    // Individual test failure markers
    if line.contains("<<< FAILURE!") || line.contains("<<< ERROR!") {
        return true;
    }
    // Maven failure-detail section headers
    if line.contains("Failed tests:") || line.contains("Tests in error:") {
        return true;
    }
    // JUnit-style test execution (often contains class name with "Test")
    // Actually this is too aggressive — leave out for now.
    false
}

/// Result of running a command.
#[derive(Debug)]
pub struct RunResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub pid: Option<u32>,
    pub duration_sec: u64,
}

/// Spawn a command and return the child process with piped output.
pub fn spawn_command(cmd: &[String]) -> Result<(Child, std::process::ChildStdout, std::process::ChildStderr), SmolError> {
    if cmd.is_empty() {
        return Err(SmolError::other("Empty command"));
    }

    let program = &cmd[0];
    let args = &cmd[1..];

    let mut command = Command::new(program);
    command.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;
    let stdout = child.stdout.take()
        .ok_or_else(|| SmolError::other("Failed to capture stdout"))?;
    let stderr = child.stderr.take()
        .ok_or_else(|| SmolError::other("Failed to capture stderr"))?;

    Ok((child, stdout, stderr))
}

/// Read all output from the child process into strings.
pub fn read_all_output(
    child: &mut Child,
    stdout: std::process::ChildStdout,
    stderr: std::process::ChildStderr,
    max_bytes: u64,
) -> Result<(String, String), SmolError> {
    let (tx, rx) = mpsc::channel();

    // Thread for stdout
    let tx_stdout = tx.clone();
    let handle_stdout = thread::spawn(move || {
        let mut reader = stdout;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).ok();
        tx_stdout.send(("stdout", buf)).ok();
    });

    // Thread for stderr
    let handle_stderr = thread::spawn(move || {
        let mut reader = stderr;
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).ok();
        tx.send(("stderr", buf)).ok();
    });

    // Wait for both threads
    handle_stdout.join().ok();
    handle_stderr.join().ok();

    // Collect results
    let mut stdout_str = String::new();
    let mut stderr_str = String::new();
    for (stream, data) in rx.iter() {
        // Truncate if too large
        let max = max_bytes as usize;
        let truncated = if data.len() > max {
            &data[..max]
        } else {
            &data[..]
        };
        let s = String::from_utf8_lossy(truncated).to_string();
        match stream {
            "stdout" => stdout_str = s,
            "stderr" => stderr_str = s,
            _ => {}
        }
    }

    // Wait for child
    let _ = child.wait();

    Ok((stdout_str, stderr_str))
}

/// Run a command synchronously and return the output.
pub fn run_sync(cmd: &[String], max_bytes: u64) -> Result<RunResult, SmolError> {
    let start = std::time::Instant::now();
    let (mut child, stdout, stderr) = spawn_command(cmd)?;
    let pid = child.id();
    let (stdout_str, stderr_str) = read_all_output(&mut child, stdout, stderr, max_bytes)?;
    let exit_code = child.wait()?.code();
    let duration = start.elapsed().as_secs();

    Ok(RunResult {
        stdout: stdout_str,
        stderr: stderr_str,
        exit_code,
        pid: Some(pid),
        duration_sec: duration,
    })
}

/// Run a command with a timeout (for auto mode).
/// If the command finishes before timeout, returns RunResult.
/// If it's still running after timeout, returns the child for backgrounding.
pub fn run_with_timeout(
    cmd: &[String],
    max_bytes: u64,
    timeout_secs: u64,
) -> Result<RunResult, SmolError> {
    let start = std::time::Instant::now();
    let (mut child, stdout_pipe, stderr_pipe) = spawn_command(cmd)?;
    let pid = child.id();

    // Read output in a thread with timeout
    let (tx, rx) = mpsc::channel();
    let tx_out = tx.clone();

    thread::spawn(move || {
        let mut stdout_reader = stdout_pipe;
        let mut stderr_reader = stderr_pipe;
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        stdout_reader.read_to_end(&mut stdout_buf).ok();
        stderr_reader.read_to_end(&mut stderr_buf).ok();
        tx_out.send((stdout_buf, stderr_buf)).ok();
    });

    // Wait for either completion or timeout
    let timeout = Duration::from_secs(timeout_secs);
    let elapsed = start.elapsed();

    let (stdout_buf, stderr_buf) = if elapsed < timeout {
        // Try to receive with remaining timeout
        let remaining = timeout - elapsed;
        match rx.recv_timeout(remaining) {
            Ok(data) => data,
            Err(_) => {
                // Timed out — kill child and return what we have
                let _ = child.kill();
                let _ = child.wait();
                return Err(SmolError::CommandTimedOut);
            }
        }
    } else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(SmolError::CommandTimedOut);
    };

    let exit_code = child.wait()?.code();
    let duration = start.elapsed().as_secs();

    // Apply max_bytes limit
    let truncate = |buf: Vec<u8>| -> String {
        let max = max_bytes as usize;
        let data = if buf.len() > max { &buf[..max] } else { &buf[..] };
        String::from_utf8_lossy(data).to_string()
    };

    Ok(RunResult {
        stdout: truncate(stdout_buf),
        stderr: truncate(stderr_buf),
        exit_code,
        pid: Some(pid),
        duration_sec: duration,
    })
}

/// Run a command with live progress display using indicatif.
///
/// Shows a spinner with elapsed time, build errors/warnings, test results,
/// and the last output line. Output is still captured and returned as a
/// normal `RunResult`.
pub fn run_interactive(
    cmd: &[String],
    max_bytes: u64,
) -> Result<RunResult, SmolError> {
    use indicatif::{ProgressBar, ProgressStyle};

    let start = std::time::Instant::now();
    let (mut child, stdout_pipe, stderr_pipe) = spawn_command(cmd)?;
    let pid = child.id();

    // Detect if this is a known build command
    let cmd_name = cmd.first().map(|s| {
        std::path::Path::new(s)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(s)
    }).unwrap_or("");

    // Compile test-detection regexes
    let re_cargo = Regex::new(r"test result:\s*\w+\.\s*(?P<passed>\d+) passed;\s*(?P<failed>\d+) failed;\s*(?P<skipped>\d+) (?:ignored|skipped)").unwrap();
    let re_maven = Regex::new(r"Tests run:\s*(?P<total>\d+),\s*Failures:\s*(?P<failures>\d+),\s*Errors:\s*(?P<errors>\d+),\s*Skipped:\s*(?P<skipped>\d+)").unwrap();
    let re_jest  = Regex::new(r"Tests:\s*(?:(?P<failed>\d+)\s+failed)?,?\s*(?:(?P<passed>\d+)\s+passed)?,?\s*(?:(?P<total>\d+)\s+total)?").unwrap();

    let mut state = LiveState {
        build_detected: BUILD_COMMANDS.contains(&cmd_name),
        build_errors: 0,
        build_warnings: 0,
        test_detected: false,
        test_passed: 0,
        test_failed: 0,
        test_skipped: 0,
    };

    // ── progress display ──────────────────────────────────────────────
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    let initial_msg = if state.build_detected {
        "[Build] running…".to_string()
    } else {
        "running…".to_string()
    };
    pb.set_message(initial_msg);

    // ── tick thread (250ms) so the spinner animates even when no output arrives ──
    let tick_running = Arc::new(AtomicBool::new(true));
    let tick_flag = tick_running.clone();
    let tick_pb = pb.clone();
    let tick_handle = thread::spawn(move || {
        while tick_flag.load(Ordering::Relaxed) {
            tick_pb.tick();
            thread::sleep(Duration::from_millis(250));
        }
    });

    // ── streaming output capture ──────────────────────────────────────
    let (tx, rx) = mpsc::channel::<(&'static str, String)>();

    let tx_out = tx.clone();
    let tx_err = tx.clone();
    let out_reader = BufReader::new(stdout_pipe);
    thread::spawn(move || {
        for line in out_reader.lines() {
            if let Ok(l) = line {
                if tx_out.send(("stdout", l)).is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    });

    let err_reader = BufReader::new(stderr_pipe);
    thread::spawn(move || {
        for line in err_reader.lines() {
            if let Ok(l) = line {
                if tx_err.send(("stderr", l)).is_err() {
                    break;
                }
            } else {
                break;
            }
        }
    });

    // Drop the original tx so the channel closes once both reader threads finish
    drop(tx);

    let mut stdout_buf = String::new();
    let mut stderr_buf = String::new();
    let max = max_bytes as usize;
    let mut truncated = false;

    // ── process lines as they arrive ──────────────────────────────────
    for (stream, line) in rx {
        let lower = line.to_lowercase();

        // Detect build from output patterns
        if !state.build_detected && is_build_output(&line) {
            state.build_detected = true;
        }

        // Detect test results
        if !state.test_detected {
            if let Some((p, f, s)) = detect_test_line(&re_cargo, &re_maven, &re_jest, &line) {
                state.test_detected = true;
                state.test_passed = p;
                state.test_failed = f;
                state.test_skipped = s;
            }
        } else {
            // Update test counts if we see newer summary lines
            if let Some((p, f, s)) = detect_test_line(&re_cargo, &re_maven, &re_jest, &line) {
                if p > 0 { state.test_passed = state.test_passed.max(p); }
                if f > 0 { state.test_failed = state.test_failed.max(f); }
                if s > 0 { state.test_skipped = state.test_skipped.max(s); }
            }
        }

        // Simple heuristic counting for build errors/warnings.
        // Skip test-output lines so that test failures/errors don't inflate build counts.
        if !is_test_output_line(&line, &re_cargo, &re_maven, &re_jest) {
            if lower.contains("error") || lower.contains("failure") {
                state.build_errors = state.build_errors.saturating_add(1);
            }
            if lower.contains("warning") {
                state.build_warnings = state.build_warnings.saturating_add(1);
            }
        }

        // Build status sections + last line (truncate to terminal width)
        let sections = build_status_line(&state);
        let clean_line = sanitize_line(&line);
        let width = terminal_width().saturating_sub(2);

        let prefix = if sections.is_empty() {
            "› ".to_string()
        } else {
            format!("{} › ", sections)
        };
        let prefix_visible = visible_len(&prefix);
        let max_line = width.saturating_sub(prefix_visible).saturating_sub(1);

        let display = if clean_line.len() > max_line {
            // Truncate safely on a char boundary
            let mut end = max_line.saturating_sub(1);
            while end > 0 && !clean_line.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}{}{}…", GRAY, &clean_line[..end], RESET)
        } else {
            format!("{}{}{}", GRAY, clean_line, RESET)
        };

        pb.set_message(format!("{}{}", prefix, display));

        // Store (with truncation)
        if !truncated {
            let target = match stream {
                "stdout" => &mut stdout_buf,
                _ => &mut stderr_buf,
            };
            let line_bytes = line.len() + 1; // +1 for newline
            if target.len() + line_bytes > max {
                let remaining = max.saturating_sub(target.len());
                if remaining > 0 {
                    target.push_str(&line[..remaining.min(line.len())]);
                    target.push('\n');
                }
                target.push_str("[truncated]");
                truncated = true;
            } else {
                target.push_str(&line);
                target.push('\n');
            }
        }
    }

    // ── finalise ──────────────────────────────────────────────────────
    // Stop the tick thread
    tick_running.store(false, Ordering::Relaxed);
    let _ = tick_handle.join();
    let exit_code = child.wait().ok().and_then(|s| s.code());
    let duration = start.elapsed().as_secs();

    // Build a final status line so the spinner doesn't just vanish
    let status_icon = match exit_code {
        Some(0) => "✓",
        Some(_) => "✗",
        None => "?",
    };
    let sections = build_status_line(&state);
    let final_msg = if sections.is_empty() {
        format!("{} done ({}s)", status_icon, duration)
    } else {
        let visible = visible_len(&sections);
        let max_sections = terminal_width().saturating_sub(30); // leave room for icon + time
        let sections_short = if visible > max_sections {
            visible_chars_take(&sections, max_sections)
        } else {
            sections
        };
        format!("{} done ({}s) — {}", status_icon, duration, sections_short)
    };
    pb.finish_with_message(final_msg);

    Ok(RunResult {
        stdout: stdout_buf,
        stderr: stderr_buf,
        exit_code,
        pid: Some(pid),
        duration_sec: duration,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_empty_command() {
        let result = spawn_command(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_sync_echo() {
        let result = run_sync(&["echo".into(), "hello smol".into()], 1024 * 1024).unwrap();
        assert!(result.stdout.contains("hello smol"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn test_run_sync_false() {
        let result = run_sync(&["false".into()], 1024 * 1024).unwrap();
        assert_eq!(result.exit_code, Some(1));
    }

    #[test]
    fn test_run_sync_with_output_truncation() {
        // Generate 100KB of output
        let result = run_sync(&["bash".into(), "-c".into(), "for i in {1..10000}; do echo 'line '$i; done".into()], 1024).unwrap();
        // Should be truncated but still have some content
        assert!(result.stdout.len() <= 1024);
        assert!(result.exit_code == Some(0));
    }

    #[test]
    fn test_run_with_timeout_completes() {
        let result = run_with_timeout(&["echo".into(), "quick".into()], 1024 * 1024, 5).unwrap();
        assert!(result.stdout.contains("quick"));
    }

    #[test]
    fn test_run_with_timeout_exceeded() {
        let result = run_with_timeout(&["sleep".into(), "10".into()], 1024 * 1024, 1);
        assert!(result.is_err());
    }

    // ── Maven test-line detection ─────────────────────────────────────

    fn make_maven_regex() -> Regex {
        Regex::new(r"Tests run:\s*(?P<total>\d+),\s*Failures:\s*(?P<failures>\d+),\s*Errors:\s*(?P<errors>\d+),\s*Skipped:\s*(?P<skipped>\d+)").unwrap()
    }

    fn make_cargo_regex() -> Regex {
        Regex::new(r"test result:\s*\w+\.\s*(?P<passed>\d+) passed;\s*(?P<failed>\d+) failed;\s*(?P<skipped>\d+) (?:ignored|skipped)").unwrap()
    }

    fn make_jest_regex() -> Regex {
        Regex::new(r"Tests:\s*(?:(?P<failed>\d+)\s+failed)?,?\s*(?:(?P<passed>\d+)\s+passed)?,?\s*(?:(?P<total>\d+)\s+total)?").unwrap()
    }

    #[test]
    fn test_maven_detect_all_passed() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        // All passed, 4 skipped
        let (p, f, s) = detect_test_line(
            &re_c, &re_m, &re_j,
            "Tests run: 15, Failures: 0, Errors: 0, Skipped: 4",
        ).unwrap();
        assert_eq!(p, 11); // 15 total - 4 skipped
        assert_eq!(f, 0);
        assert_eq!(s, 4);
    }

    #[test]
    fn test_maven_detect_with_failures_and_errors() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        let (p, f, s) = detect_test_line(
            &re_c, &re_m, &re_j,
            "Tests run: 25, Failures: 2, Errors: 1, Skipped: 3",
        ).unwrap();
        assert_eq!(p, 19);  // 25 - 2 - 1 - 3
        assert_eq!(f, 3);   // failures + errors
        assert_eq!(s, 3);
    }

    #[test]
    fn test_maven_detect_with_time_suffix() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        let (p, f, s) = detect_test_line(
            &re_c, &re_m, &re_j,
            "Tests run: 10, Failures: 1, Errors: 0, Skipped: 0, Time elapsed: 1.234 s",
        ).unwrap();
        assert_eq!(p, 9);
        assert_eq!(f, 1);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_maven_detect_with_info_prefix() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        let (p, f, s) = detect_test_line(
            &re_c, &re_m, &re_j,
            "[INFO] Tests run: 42, Failures: 0, Errors: 0, Skipped: 1",
        ).unwrap();
        assert_eq!(p, 41);
        assert_eq!(f, 0);
        assert_eq!(s, 1);
    }

    #[test]
    fn test_maven_with_failure_marker() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        let (p, f, s) = detect_test_line(
            &re_c, &re_m, &re_j,
            "Tests run: 5, Failures: 1, Errors: 0, Skipped: 0, Time elapsed: 0.123 s <<< FAILURE!",
        ).unwrap();
        assert_eq!(p, 4);
        assert_eq!(f, 1);
        assert_eq!(s, 0);
    }

    #[test]
    fn test_is_test_output_line_maven_summary() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        assert!(is_test_output_line(
            "Tests run: 15, Failures: 2, Errors: 1, Skipped: 3", &re_c, &re_m, &re_j));
        assert!(is_test_output_line(
            "[INFO] Tests run: 15, Failures: 2, Errors: 1, Skipped: 3", &re_c, &re_m, &re_j));
    }

    #[test]
    fn test_is_test_output_line_runner() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        assert!(is_test_output_line(
            "[INFO] Running org.apache.commons.lang3.time.FastDateParserTest", &re_c, &re_m, &re_j));
        assert!(is_test_output_line(
            "Running com.example.MyTest", &re_c, &re_m, &re_j));
    }

    #[test]
    fn test_is_test_output_line_failure_details() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        assert!(is_test_output_line("testMethod  Time elapsed: 0.001s  <<< FAILURE!", &re_c, &re_m, &re_j));
        assert!(is_test_output_line("testMethod  Time elapsed: 0.001s  <<< ERROR!", &re_c, &re_m, &re_j));
        assert!(is_test_output_line("Failed tests:", &re_c, &re_m, &re_j));
        assert!(is_test_output_line("Tests in error:", &re_c, &re_m, &re_j));
    }

    #[test]
    fn test_is_test_output_line_build_error_not_suppressed() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();
        // Real build errors should NOT be classified as test output
        assert!(!is_test_output_line(
            "[ERROR] /src/main/java/App.java:42: error: cannot find symbol", &re_c, &re_m, &re_j));
        assert!(!is_test_output_line(
            "error[E0425]: cannot find value `foo` in this scope", &re_c, &re_m, &re_j));
    }

    #[test]
    fn test_build_status_line_full() {
        let state = LiveState {
            build_detected: true,
            build_errors: 2,
            build_warnings: 1,
            test_detected: true,
            test_passed: 25,
            test_failed: 3,
            test_skipped: 4,
        };
        let line = build_status_line(&state);
        assert!(line.contains("Build"));
        assert!(line.contains("err:2"));
        assert!(line.contains("warn:1"));
        assert!(line.contains("Test"));
        assert!(line.contains("pass:25"));
        assert!(line.contains("fail:3"));
        assert!(line.contains("skip:4"));
    }

    #[test]
    fn test_build_status_line_no_tests() {
        let state = LiveState {
            build_detected: true,
            build_errors: 0,
            build_warnings: 0,
            test_detected: false,
            test_passed: 0,
            test_failed: 0,
            test_skipped: 0,
        };
        let line = build_status_line(&state);
        assert!(line.contains("Build"));
        assert!(!line.contains("Test"));
    }

    /// Simulate processing lines from the Maven fixture script.
    /// Build errors (2 compile errors) should not be inflated by test lines.
    #[test]
    fn test_maven_pipeline_no_double_count() {
        let re_c = make_cargo_regex();
        let re_m = make_maven_regex();
        let re_j = make_jest_regex();

        let mut state = LiveState {
            build_detected: true, // mvn is a known build command
            build_errors: 0,
            build_warnings: 0,
            test_detected: false,
            test_passed: 0,
            test_failed: 0,
            test_skipped: 0,
        };

        // Simulate processing the fixture output line by line
        let lines = [
            "[INFO] --- maven-compiler-plugin:3.8.1:compile ---",
            "[ERROR] /src/App.java:15: error: cannot find symbol",
            "[ERROR] /src/App.java:22: error: incompatible types",
            "[WARNING] /src/App.java:10: unchecked cast",
            "[WARNING] /src/Utils.java:5: rawtypes warning",
            "[INFO] Running com.example.FastDateParserTest",
            "Tests run: 15, Failures: 2, Errors: 1, Skipped: 3",
            "<<< FAILURE!",
            "Tests in error:",
            "Failed tests:",
            "[INFO] Tests run: 28, Failures: 2, Errors: 1, Skipped: 4",
            "[INFO] BUILD FAILURE",
        ];

        for line in &lines {
            let lower = line.to_lowercase();

            if let Some((p, f, s)) = detect_test_line(&re_c, &re_m, &re_j, line) {
                state.test_detected = true;
                if p > 0 { state.test_passed = state.test_passed.max(p); }
                if f > 0 { state.test_failed = state.test_failed.max(f); }
                if s > 0 { state.test_skipped = state.test_skipped.max(s); }
            }

            if !is_test_output_line(line, &re_c, &re_m, &re_j) {
                if lower.contains("error") || lower.contains("failure") {
                    state.build_errors = state.build_errors.saturating_add(1);
                }
                if lower.contains("warning") {
                    state.build_warnings = state.build_warnings.saturating_add(1);
                }
            }
        }

        // Build: only the 2 real [ERROR] compilation lines + "build failure"
        assert_eq!(state.build_errors, 3, "only compile errors + BUILD FAILURE");
        assert_eq!(state.build_warnings, 2, "two [WARNING] lines");
        // Tests: 28 total - 2 failures - 1 error - 4 skipped = 21 passed
        assert_eq!(state.test_passed, 21);
        assert_eq!(state.test_failed, 3); // 2 failures + 1 error
        assert_eq!(state.test_skipped, 4);
    }
}
