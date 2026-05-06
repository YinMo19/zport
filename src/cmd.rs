use std::process::Stdio;

use crate::ssh;
use crate::state::{self, Entry};

pub fn connect(host: &str, ssh_port: u16) {
    let mut state = state::load();
    state.default_host = Some(host.to_string());

    if ssh_port != 22 {
        state.ports.insert(host.to_string(), ssh_port);
    }

    state::save(&state);

    if ssh::cm_available() {
        ssh::ensure_master(host, ssh_port);
        println!("✓ connected to {host}");
    } else {
        println!("✓ default host set to {host}");
    }
}

pub fn forward(host: Option<&str>, ssh_port: Option<u16>, remote: u16, local: Option<u16>) {
    let mut state = state::load();
    let host = host
        .map(|s| s.to_string())
        .or_else(|| state.default_host.clone())
        .unwrap_or_else(|| {
            eprintln!("error: no host configured.  Run 'zport <host>' first.");
            std::process::exit(1);
        });
    let port = ssh_port
        .or_else(|| state.ports.get(&host).copied())
        .unwrap_or(22);
    let local = ssh::resolve_port(remote, local);

    // Two code paths: ControlMaster multiplexing (Linux/macOS) vs individual `ssh -L` (Windows).
    if ssh::cm_available() {
        ssh::ensure_master(&host, port);
        let mut c = ssh::ssh_cm(&host, port);
        c.args([
            "-O",
            "forward",
            "-L",
            &format!("{local}:localhost:{remote}"),
        ]);
        c.stdout(Stdio::null());
        c.stderr(Stdio::piped());
        let out = c.output().unwrap_or_else(|e| {
            eprintln!("error: ssh failed: {e}");
            std::process::exit(1);
        });
        if !out.status.success() {
            let msg = String::from_utf8_lossy(&out.stderr);
            eprintln!("error: forward failed: {msg}", msg = msg.trim());
            std::process::exit(1);
        }
        state
            .hosts
            .entry(host.clone())
            .or_default()
            .insert(local.to_string(), Entry::Cm { remote });
    } else {
        let pid = ssh::spawn_forward(&host, port, local, remote);
        state
            .hosts
            .entry(host.clone())
            .or_default()
            .insert(local.to_string(), Entry::Proc { remote, pid });
    }

    if port != 22 {
        state.ports.insert(host.clone(), port);
    }
    state.default_host = Some(host.clone());
    state::save(&state);
    println!("✓ :{local} → :{remote} on {host}");
}

pub fn list() {
    let state = state::load();
    let hosts = &state.hosts;
    if hosts.is_empty() {
        println!("no active forwards");
        return;
    }
    for (host, forwards) in hosts {
        let port = state.ports.get(host).copied().unwrap_or(22);
        let alive = if ssh::cm_available() {
            ssh::master_alive(host, port)
        } else {
            forwards
                .values()
                .any(|e| e.pid().is_some_and(ssh::pid_alive))
        };
        let label = if port == 22 {
            host.clone()
        } else {
            format!("{host}:{port}")
        };
        println!(
            "{label} ({})",
            if alive { "connected" } else { "disconnected" }
        );
        let mut sorted: Vec<_> = forwards.iter().collect();
        sorted.sort_by_key(|(k, _)| k.parse::<u16>().unwrap_or(0));
        println!("  {:>10}  REMOTE", "LOCAL");
        for (lp, e) in &sorted {
            println!("  {lp:>10}  localhost:{}", e.remote());
        }
    }
    println!();
}

pub fn close(host: Option<&str>, local_port: u16) {
    let mut state = state::load();
    let host = host
        .or(state.default_host.as_deref())
        .unwrap_or_else(|| {
            eprintln!("error: no host configured");
            std::process::exit(1);
        })
        .to_string();

    let entry = state
        .hosts
        .get_mut(&host)
        .and_then(|fws| fws.remove(&local_port.to_string()))
        .unwrap_or_else(|| {
            eprintln!("error: port {local_port} not found in active forwards");
            std::process::exit(1);
        });

    let port = state.ports.get(&host).copied().unwrap_or(22);

    if ssh::cm_available() {
        let mut c = ssh::ssh_cm(&host, port);
        c.args([
            "-O",
            "cancel",
            "-L",
            &format!("{local_port}:localhost:{}", entry.remote()),
        ]);
        c.stdout(Stdio::null());
        c.stderr(Stdio::null());

        // Best-effort: the connection may already be dead, so a failed cancel is not fatal.
        if !c.status().map(|s| s.success()).unwrap_or(false) {
            eprintln!("warning: could not cancel on {host} (connection lost?), cleaning up state");
        }
    } else if let Some(pid) = entry.pid() {
        ssh::kill_pid(pid);
    }

    if state.hosts.get(&host).is_none_or(|fws| fws.is_empty()) {
        state.hosts.remove(&host);
    }
    state::save(&state);
    println!("✓ closed :{local_port} on {host}");
}

pub fn disconnect(host: Option<&str>) {
    let mut state = state::load();
    let host = host
        .or(state.default_host.as_deref())
        .unwrap_or_else(|| {
            eprintln!("error: no host configured");
            std::process::exit(1);
        })
        .to_string();

    let forwards = state.hosts.remove(&host).unwrap_or_default();
    let port = state.ports.get(&host).copied().unwrap_or(22);

    if ssh::cm_available() {
        for (lp, e) in &forwards {
            let mut c = ssh::ssh_cm(&host, port);
            c.args([
                "-O",
                "cancel",
                "-L",
                &format!("{lp}:localhost:{}", e.remote()),
            ]);
            c.stdout(Stdio::null());
            c.stderr(Stdio::null());
            c.status().ok();
        }
        // Tell the ControlMaster to shut down, then remove its socket.
        let mut c = ssh::ssh_cm(&host, port);
        c.args(["-O", "exit"]);
        c.stdout(Stdio::null());
        c.stderr(Stdio::null());
        c.status().ok();
        let sock = state::sock_path(&host);
        if sock.exists() {
            let _ = std::fs::remove_file(&sock);
        }
    } else {
        for e in forwards.values() {
            if let Some(pid) = e.pid() {
                ssh::kill_pid(pid);
            }
        }
    }

    if state.default_host.as_deref() == Some(&host) {
        state.default_host = None;
    }
    state::save(&state);
    println!("✓ disconnected from {host}");
}
