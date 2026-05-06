mod cmd;
mod ssh;
mod state;

use clap::Parser;

#[derive(Parser)]
#[command(name = "zport", version, about = "one-command SSH port forwarding")]
enum Cli {
    /// Connect to host and set as default
    Connect {
        /// [user@]host[:ssh-port]
        host_spec: String,
    },
    /// Forward a remote port to localhost
    Forward {
        /// <remote-port> or <remote-port>:<local-port>
        port_spec: String,
        /// [user@]host[:ssh-port] (uses default host if omitted)
        host_spec: Option<String>,
    },
    /// List active forwards
    Ls,
    /// Stop a forward
    Close {
        /// Local port to close
        local_port: u16,
        /// Host (uses default if omitted)
        host: Option<String>,
    },
    /// Shut down connection to host
    Disconnect {
        /// Host (uses default if omitted)
        host: Option<String>,
    },
}

fn parse_port_spec(spec: &str) -> Option<(u16, Option<u16>)> {
    if let Some((a, b)) = spec.split_once(':') {
        Some((a.parse().ok()?, b.parse().ok().map(Some)?))
    } else {
        spec.parse().ok().map(|r| (r, None))
    }
}

// Map positional shorthand to subcommand args: `zport 8080` → `zport forward 8080`.
fn map_positional(args: &[String]) -> Vec<String> {
    let mut out = vec!["zport".into()];

    match args.len() {
        1 => {
            let a = &args[0];
            if a.chars().all(|c| c.is_ascii_digit()) {
                out.push("forward".into());
                out.push(a.clone());
            } else if let Some((l, _)) = a.split_once(':') {
                if l.chars().all(|c| c.is_ascii_digit()) {
                    out.push("forward".into());
                } else {
                    out.push("connect".into());
                }
                out.push(a.clone());
            } else {
                out.push("connect".into());
                out.push(a.clone());
            }
        }
        2 => {
            out.push("forward".into());
            out.push(args[0].clone());
            out.push(args[1].clone());
        }
        _ => {
            eprintln!("error: too many arguments");
            std::process::exit(1);
        }
    }

    out
}

fn main() {
    let raw: Vec<String> = std::env::args().collect();
    let known = ["connect", "forward", "ls", "close", "disconnect", "help"];

    let cli = if raw.len() > 1 && !known.contains(&raw[1].as_str()) && !raw[1].starts_with('-') {
        Cli::parse_from(map_positional(&raw[1..]))
    } else {
        Cli::parse()
    };

    match cli {
        Cli::Connect { host_spec } => {
            let (host, port) = ssh::parse_host_spec(&host_spec);
            cmd::connect(host, port);
        }
        Cli::Forward {
            port_spec,
            host_spec,
        } => {
            let (host, ssh_port) = host_spec
                .as_deref()
                .map(|s| {
                    let (h, p) = ssh::parse_host_spec(s);
                    (Some(h), Some(p))
                })
                .unzip();
            let host = host.flatten();

            if let Some((remote, local)) = parse_port_spec(&port_spec) {
                cmd::forward(host, ssh_port.flatten(), remote, local);
            } else if let Ok(remote) = port_spec.parse() {
                cmd::forward(host, ssh_port.flatten(), remote, None);
            } else {
                eprintln!("error: invalid port spec: {port_spec}");
                std::process::exit(1);
            }
        }
        Cli::Ls => cmd::list(),
        Cli::Close { local_port, host } => cmd::close(host.as_deref(), local_port),
        Cli::Disconnect { host } => cmd::disconnect(host.as_deref()),
    }
}
