use serde::Serialize;

/// Process state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Idle,
    Unknown,
}

impl ProcessState {
    pub fn label(self) -> &'static str {
        match self {
            ProcessState::Running => "RUN",
            ProcessState::Sleeping => "SLP",
            ProcessState::Stopped => "STP",
            ProcessState::Zombie => "ZMB",
            ProcessState::Idle => "IDL",
            ProcessState::Unknown => "???",
        }
    }
}

/// A network connection associated with a process.
#[derive(Debug, Clone, Serialize)]
pub struct NetworkConnection {
    pub proto: String,
    pub local_addr: String,
    pub remote_addr: Option<String>,
    pub state: Option<String>,
}

/// Detailed process information for the info panel.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessDetail {
    pub pid: i32,
    pub ppid: i32,
    pub parent_name: String,
    pub name: String,
    pub path: String,
    pub cwd: String,
    pub user: String,
    pub uid: u32,
    pub state: ProcessState,
    pub nice: i32,
    pub started_secs: u64,
    pub thread_count: i32,
    pub virtual_size: u64,
    pub memory_rss: u64,
    pub memory_phys: u64,
    pub cpu_time_ns: u64,
    pub fd_files: usize,
    pub fd_sockets: usize,
    pub fd_pipes: usize,
    pub fd_other: usize,
    pub fd_total: usize,
    pub faults: i32,
    pub context_switches: i32,
    pub syscalls_mach: i32,
    pub syscalls_unix: i32,
    pub disk_bytes_read: u64,
    pub disk_bytes_written: u64,
    pub network: Vec<NetworkConnection>,
    pub environment: Vec<(String, String)>,
}

/// Memory usage for a process.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessMemory {
    /// Resident set size in bytes.
    pub rss: u64,
    /// Physical footprint (private memory) in bytes.
    pub phys: u64,
}

/// CPU and scheduling activity for a process.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessActivity {
    /// Cumulative CPU time (user + system) in nanoseconds.
    pub cpu_time_ns: u64,
    pub thread_count: i32,
    pub faults: i32,
    pub context_switches: i32,
    pub syscalls_mach: i32,
    pub syscalls_unix: i32,
}

/// Disk I/O counters for a process.
#[derive(Debug, Clone, Serialize)]
pub struct DiskIo {
    pub bytes_read: u64,
    pub bytes_written: u64,
}

/// A snapshot of a single process and its open resources.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessSnapshot {
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    /// Process state (running, sleeping, zombie, etc.).
    pub state: ProcessState,
    /// Whether we had full access to this process's fds.
    /// False when permission was denied — pid/name are still valid.
    pub accessible: bool,
    pub memory: ProcessMemory,
    pub activity: ProcessActivity,
    pub disk_io: DiskIo,
    pub resources: Vec<OpenResource>,
}

/// A single open file descriptor or resource.
#[derive(Debug, Clone, Serialize)]
pub struct OpenResource {
    pub descriptor: i32,
    pub kind: ResourceKind,
    pub path: Option<String>,
}

impl ProcessSnapshot {
    /// Count resources by kind.
    pub fn count_by_kind(&self, kind: &ResourceKind) -> usize {
        self.resources.iter().filter(|r| &r.kind == kind).count()
    }
}

/// The type of an open resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceKind {
    File,
    Socket,
    Pipe,
    Device,
    Kqueue,
    Unknown,
}
