# Native macOS APIs (shared reference)

Both implementations read the same macOS APIs to enumerate processes, their open
file descriptors, and system metrics. This is the shared reference — the APIs, the
exact arithmetic, and the gotchas. How each side reaches them:

- **Rust** — raw `extern "C"` bindings + safe wrappers in the `prexp-ffi` crate
  (`raw.rs` for the `#[repr(C)]` structs and signatures). All `unsafe` is here.
- **Swift** — reached **directly from the `Darwin` module** in `PrexpCore`'s
  `NativeSource`. **No C shim is needed** — every symbol, constant, and struct
  below is surfaced by `Darwin` (verified for the full set: proc enumeration, fd
  listing, `task_info`, `proc_pid_rusage`, `host_statistics64`, `proc_listpidspath`,
  IOKit power sources).

The two implementations are kept in agreement by [PARITY.md](PARITY.md).

## libproc

| API | Purpose |
|---|---|
| `proc_listallpids` | enumerate all PIDs |
| `proc_pidinfo(PROC_PIDTBSDINFO)` | PPID, name (`pbi_name`, 32 chars), state (`pbi_status`), uid, nice, start time |
| `proc_pidinfo(PROC_PIDTASKINFO)` | threads, RSS, CPU time, faults, context switches, syscalls |
| `proc_pidinfo(PROC_PIDLISTFDS)` | list open file descriptors |
| `proc_pidinfo(PROC_PIDVNODEPATHINFO)` | current working directory |
| `proc_pidfdinfo(PROC_PIDFDVNODEPATHINFO)` | resolve a vnode fd → path |
| `proc_pidfdinfo(PROC_PIDFDSOCKETINFO)` | resolve a socket fd → family/type/addrs/state |
| `proc_pid_rusage(RUSAGE_INFO_V4)` | disk I/O bytes read/written |
| `proc_pidpath` | executable path |
| `proc_listpidspath` | reverse lookup (PIDs with a given path open) |

**`pbi_status` → state:** `SIDL(1)`→Idle, `SRUN(2)`→Running, `SSLEEP(3)`→Sleeping,
`SSTOP(4)`→Stopped, `SZOMB(5)`→Zombie, else Unknown.

**fd-type classification** (`proc_fdinfo.proc_fdtype`): `VNODE(1)`→File (or Device
if the path starts `/dev/`), `SOCKET(2)`→Socket, `PIPE(6)`→Pipe, `KQUEUE(5)`→Kqueue,
else Unknown.

## Mach

| API | Purpose |
|---|---|
| `mach_timebase_info` | Mach ticks → nanoseconds (cached once) |
| `task_name_for_pid` | task port without root |
| `task_info(TASK_VM_INFO)` | physical footprint (`phys_footprint`, matches Activity Monitor) |
| `host_processor_info(PROCESSOR_CPU_LOAD_INFO)` | per-core tick counts (user/system/idle/nice) |
| `host_statistics64(HOST_VM_INFO64)` | system memory (free/active/wired/compressed pages) |

## sysctl / getloadavg

| Name | Purpose |
|---|---|
| `sysctlbyname("hw.memsize")` | total physical memory |
| `sysctlbyname("hw.pagesize")` | page size for VM-stat conversion |
| `sysctlbyname("vm.swapusage")` | swap total/used |
| `sysctl(CTL_KERN, KERN_BOOTTIME)` | boot time |
| `sysctl(CTL_KERN, KERN_PROCARGS2, pid)` | process arguments + environment |
| `getloadavg(3)` | 1/5/15-minute load average |
| `hw.perflevelN.logicalcpu` / `hw.nperflevels` | P/E core split (Swift; Rust reads IORegistry `cluster-type`) |

## IOKit

`IOPSCopyPowerSourcesInfo` / `IOPSCopyPowerSourcesList` /
`IOPSGetPowerSourceDescription` — battery percent, charging, time-to-empty/full.

## Gotchas (both implementations must match these)

- **Mach timebase.** `proc_taskinfo`'s `pti_total_user`/`pti_total_system` are Mach
  absolute-time **ticks**, not nanoseconds. Convert with `numer/denom` from
  `mach_timebase_info` (Apple Silicon is typically **125:3**, Intel 1:1). Sum the
  raw ticks first, then convert.
- **`proc_listallpids` returns a COUNT; `proc_listpidspath` returns BYTES.** A real
  libproc inconsistency. Divide the second by `sizeof(pid_t)`, but *not* the first.
  (This bit the Swift port — it under-enumerated until fixed.)
- **Physical footprint needs `task_name_for_pid`**, not `task_for_pid` — the former
  works without root for the user's own processes.
- **`in_sockinfo` must be the full struct size.** `tcp_sockinfo` embeds it as its
  first member, so a short `in_sockinfo` makes `tcpsi_state` read the wrong offset
  (every TCP connection comes out `CLOSED`). This was a latent Rust bug found by the
  parity check; keep both sides at the exact C struct size.
- **Socket ports are shown raw** (no `ntohs`) — a shared quirk, so a byte-swapped
  443 displays as 47873. Both sides do this identically; if fixed, fix both.
- **KERN_PROCARGS2 env parsing.** Layout is
  `[argc:i32][exec_path\0][padding\0…][argv\0…][env\0…\0]`; skip argc argv entries
  after the padding, then read `KEY=VALUE` pairs until the terminating empty string.
