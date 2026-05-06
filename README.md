# zport

One-command SSH port forwarding — forward remote ports to localhost with zero server-side setup.

```bash
zport myhost:2235          # connect & set default host
zport 8080                 # forward remote :8080 → local :8080
zport 5432:5433            # forward with explicit local port
zport myhost 3000          # forward on a specific host
zport ls                   # list active forwards
zport close 8080           # stop a forward
zport disconnect           # shut down connection
```

**No daemon, no agent, no server-side config.** Single binary, stdlib-heavy.

## How it works

On Linux/macOS, zport uses SSH **ControlMaster multiplexing** — one persistent
SSH connection per host, with port forwards added/removed dynamically via
`ssh -O forward` / `ssh -O cancel`. On Windows, it falls back to individual
`ssh -L` background processes.

State is stored in `~/.zport/state.json`.

## Install

```bash
cargo install --git https://github.com/YinMo19/zport
```

Requires `ssh` in PATH and key-based auth to the target host (ControlMaster
uses `BatchMode=yes`; fallback mode on Windows allows interactive password).

## Usage

```
zport <COMMAND>

Commands:
  connect     Connect to host and set as default
  forward     Forward a remote port to localhost
  ls          List active forwards
  close       Stop a forward
  disconnect  Shut down connection to host
  help        Print help
```

Positional shorthand works too — no need to type the subcommand name:

| Shorthand | Equivalent |
|-----------|-----------|
| `zport myhost` | `zport connect myhost` |
| `zport 8080` | `zport forward 8080` |
| `zport myhost 8080` | `zport forward 8080 myhost` |
| `zport 8080:8081` | `zport forward 8080:8081` |
