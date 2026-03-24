use serde::Serialize;

/// A snapshot of a single process and its open resources.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessSnapshot {
    pub pid: i32,
    pub ppid: i32,
    pub name: String,
    pub thread_count: i32,
    /// Resident set size in bytes.
    pub memory_rss: u64,
    /// Physical footprint (private memory) in bytes.
    pub memory_phys: u64,
    /// Cumulative CPU time (user + system) in nanoseconds.
    pub cpu_time_ns: u64,
    /// Process state (running, sleeping, zombie, etc.).
    pub state: prexp_ffi::ProcessState,
    /// Whether we had full access to this process's fds.
    /// False when permission was denied — pid/name are still valid.
    pub accessible: bool,
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
