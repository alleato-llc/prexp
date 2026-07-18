# Security

## What prexp can access

prexp is a local process explorer. It reads process and system information through
standard macOS APIs — `libproc`, Mach (`task_info`, `host_*`), `sysctl`, and IOKit
(battery). Specifically it can read, for processes **the current user can inspect**:
names, PIDs, open file-descriptor paths, memory/CPU stats, network connections, and
environment variables; and it can send signals (e.g. `SIGTERM`, `SIGKILL`) to those
processes.

- It uses `task_name_for_pid` (not `task_for_pid`), so it needs **no root** and gets
  the same visibility as the user running it. Processes it can't inspect are shown as
  inaccessible, not bypassed.
- It makes **no network connections** and collects **no telemetry** — everything is
  read locally and rendered locally.
- The environment-variable view can expose secrets that a process was launched with
  (API keys, tokens). Treat that view as sensitive.

## The desktop app is not sandboxed

The native SwiftUI app (`swift/App`) is deliberately built **without** the macOS App
Sandbox — the sandbox forbids the libproc/Mach calls needed to inspect other
processes. It is ad-hoc signed for local development and is **not** distributed as a
notarized app. Build and run it from source only. The same applies to the Rust GUI
(`prexp-desktop`).

## Reporting an issue

This is a personal/source-first project with no formal release channel. To report a
security issue, open an issue in the repository (or contact the maintainer directly)
with steps to reproduce. Please don't include real secrets in reports.
