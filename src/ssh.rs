use std::fs;
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::state;

const COMMON_OPTS: &[&str] = &["-o", "StrictHostKeyChecking=accept-new"];

// Parse `[user@]host[:port]`. IPv6 addresses in brackets like `[::1]` or `[::1]:2235`.
pub fn parse_host_spec(spec: &str) -> (&str, u16) {
    // Bare IPv6 bracket — no port.
    if spec.ends_with(']') {
        return (spec, 22);
    }
    if let Some((host, port_str)) = spec.rsplit_once(':')
        && let Ok(port) = port_str.parse()
    {
        return (host, port);
    }
    (spec, 22)
}

pub fn port_free(port: u16) -> bool {
    TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}").parse().unwrap(),
        Duration::from_millis(200),
    )
    .is_err()
}

pub fn resolve_port(remote: u16, local: Option<u16>) -> u16 {
    if let Some(l) = local {
        if !port_free(l) {
            eprintln!("error: localhost:{l} already in use");
            std::process::exit(1);
        }
        return l;
    }

    let mut p = remote;
    while !port_free(p) {
        p += 1;
    }

    if p != remote {
        eprintln!("warning: localhost:{remote} in use, using localhost:{p}");
    }
    p
}

// Windows OpenSSH's ControlMaster is broken (getsockname failures), so we fall back to
// individual `ssh -L` processes there.
pub fn cm_available() -> bool {
    !cfg!(windows)
}

pub fn pid_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
}

pub fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

// Base SSH command with common options. Does NOT set BatchMode — the caller decides
// whether the invocation should be non-interactive (BatchMode=yes) or allow password prompts.
fn ssh_base(port: u16) -> Command {
    let mut c = Command::new("ssh");
    c.args(COMMON_OPTS);
    if port != 22 {
        c.args(["-p", &port.to_string()]);
    }
    c
}

// SSH command targeted at an existing ControlMaster socket. Always uses BatchMode
// because the master is already authenticated — no password prompt needed.
pub fn ssh_cm(host: &str, port: u16) -> Command {
    let mut c = ssh_base(port);
    c.arg("-o").arg("BatchMode=yes");
    c.arg("-S")
        .arg(state::sock_path(host).to_string_lossy().as_ref());
    c.arg(host);
    c
}

pub fn master_alive(host: &str, port: u16) -> bool {
    ssh_cm(host, port)
        .args(["-O", "check"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn ensure_master(host: &str, port: u16) {
    if !cm_available() {
        return;
    }
    if master_alive(host, port) {
        return;
    }

    // Stale socket from a dead session blocks a fresh one — remove it.
    let sock = state::sock_path(host);
    if sock.exists() {
        let _ = fs::remove_file(&sock);
    }
    let _ = fs::create_dir_all(state::zport_dir());

    let mut c = ssh_base(port);
    c.arg("-o").arg("BatchMode=yes");
    c.args(["-N", "-M", "-S"]);
    c.arg(sock.to_string_lossy().as_ref());
    c.arg(host);
    c.stdin(Stdio::null());
    c.stdout(Stdio::null());
    c.stderr(Stdio::null());

    let mut child = c.spawn().unwrap_or_else(|e| {
        eprintln!("error: failed to start ssh: {e}");
        std::process::exit(1);
    });

    // Poll up to 5 s for the master to come alive.
    for _ in 0..50 {
        thread::sleep(Duration::from_millis(100));
        if master_alive(host, port) {
            return;
        }
    }

    let detail = match child.try_wait() {
        Ok(Some(s)) => format!(" (ssh exited with code {s})"),
        Ok(None) => " (still running after 5 s timeout)".into(),
        Err(e) => format!(" (ssh error: {e})"),
    };
    eprintln!("error: ControlMaster failed to start for {host}{detail}");
    std::process::exit(1);
}

// Fallback: start an independent `ssh -L` background process. No BatchMode — the user
// may need to type a password. On Windows, `-f` backgrounds after auth and we poll
// netstat for the real PID. On Unix we detach via null stdio.
pub fn spawn_forward(host: &str, port: u16, local: u16, remote: u16) -> u32 {
    let mut c = ssh_base(port);

    #[cfg(windows)]
    c.arg("-f");

    c.args(["-L", &format!("{local}:localhost:{remote}"), "-N", host]);

    #[cfg(windows)]
    {
        c.stdout(Stdio::null());
        c.spawn().unwrap_or_else(|e| {
            eprintln!("error: failed to start ssh: {e}");
            std::process::exit(1);
        });

        for _ in 0..60 {
            thread::sleep(Duration::from_millis(500));
            if let Some(pid) = find_listening_pid(local) {
                return pid;
            }
        }
        eprintln!("error: forward failed (connection not established within 30 s)");
        std::process::exit(1);
    }

    #[cfg(not(windows))]
    {
        c.stdin(Stdio::null());
        c.stdout(Stdio::null());
        c.stderr(Stdio::null());

        let mut child = c.spawn().unwrap_or_else(|e| {
            eprintln!("error: failed to start ssh: {e}");
            std::process::exit(1);
        });

        thread::sleep(Duration::from_millis(500));
        if let Ok(Some(s)) = child.try_wait() {
            eprintln!("error: forward failed (ssh exited with code {s})");
            std::process::exit(1);
        }
        child.id()
    }
}

#[cfg(windows)]
fn find_listening_pid(local: u16) -> Option<u32> {
    let target = format!("127.0.0.1:{local}");
    let out = Command::new("netstat").args(["-ano"]).output().ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);

    for line in stdout.lines() {
        if line.contains(&target) && line.contains("LISTENING") {
            return line.split_whitespace().last()?.parse().ok();
        }
    }
    None
}
