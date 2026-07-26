use std::cmp::Reverse;
use std::fmt;

use prexp_core::models::{ProcessSnapshot, ResourceKind};

use super::cpu::CpuTracker;

/// A column the process table can be ordered by. Numeric fields default to
/// descending (biggest consumers first); `NAME`/`PID` default to ascending.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortField {
    Cpu,
    Rss,
    Pmem,
    Threads,
    Files,
    Total,
    Name,
    Pid,
}

impl SortField {
    /// Every field, in dropdown order.
    pub const ALL: [SortField; 8] = [
        SortField::Cpu,
        SortField::Rss,
        SortField::Pmem,
        SortField::Threads,
        SortField::Files,
        SortField::Total,
        SortField::Name,
        SortField::Pid,
    ];

    fn label(self) -> &'static str {
        match self {
            SortField::Cpu => "CPU%",
            SortField::Rss => "MEM",
            SortField::Pmem => "PMEM",
            SortField::Threads => "THR",
            SortField::Files => "FILES",
            SortField::Total => "TOTAL",
            SortField::Name => "NAME",
            SortField::Pid => "PID",
        }
    }
}

impl fmt::Display for SortField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Order `procs` in place by `field`. Each field has a natural default
/// direction (numeric = descending, textual = ascending); `reverse` flips it.
pub fn sort_processes(
    procs: &mut [ProcessSnapshot],
    field: SortField,
    cpu: &CpuTracker,
    reverse: bool,
) {
    // `Reverse` keys give the numeric fields their descending default; CPU% is
    // an `f64`, so it uses `total_cmp` (there's no `Ord` key to reverse).
    match field {
        SortField::Name => procs.sort_by_key(|p| p.name.to_lowercase()),
        SortField::Pid => procs.sort_by_key(|p| p.pid),
        SortField::Cpu => {
            procs.sort_by(|a, b| cpu.percent(b.pid).total_cmp(&cpu.percent(a.pid)))
        }
        SortField::Rss => procs.sort_by_key(|p| Reverse(p.memory.rss)),
        SortField::Pmem => procs.sort_by_key(|p| Reverse(p.memory.phys)),
        SortField::Threads => procs.sort_by_key(|p| Reverse(p.activity.thread_count)),
        SortField::Files => procs.sort_by_key(|p| Reverse(p.count_by_kind(&ResourceKind::File))),
        SortField::Total => procs.sort_by_key(|p| Reverse(p.resources.len())),
    }
    if reverse {
        procs.reverse();
    }
}
