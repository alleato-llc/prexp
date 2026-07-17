#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacosProcessSource as NativeSource;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::LinuxProcessSource as NativeSource;

#[cfg(test)]
mod tests {
    use super::NativeSource;
    use crate::source::ProcessSource;

    #[test]
    fn process_summaries_lists_the_running_process() {
        let source = NativeSource;
        let procs = source.process_summaries().expect("read process summaries");
        assert!(!procs.is_empty(), "there is at least one process");
        // Our own pid should be in the list, with a name and non-zero footprint.
        let me = std::process::id() as i32;
        let mine = procs
            .iter()
            .find(|p| p.pid == me)
            .expect("the test process is listed");
        assert!(!mine.name.is_empty(), "process has a name");
        assert!(mine.memory_phys > 0, "process has a memory footprint");
    }
}
