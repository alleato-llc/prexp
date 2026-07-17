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

    #[test]
    fn process_path_resolves_the_running_executable() {
        let source = NativeSource;
        let me = std::process::id() as i32;
        let path = source.process_path(me).expect("read own executable path");
        assert!(!path.is_empty(), "path is non-empty");
        assert!(
            std::path::Path::new(&path).is_absolute(),
            "path is absolute: {path}"
        );
    }

    #[test]
    fn kill_process_signal_zero_probes_existence() {
        // Signal 0 sends nothing — it just checks the target exists and is ours,
        // so we can test the plumbing without terminating anything.
        let source = NativeSource;
        let me = std::process::id() as i32;
        assert!(source.kill_process(me, 0).is_ok(), "the test process exists");
        // A pid that (almost certainly) isn't running fails.
        assert!(
            source.kill_process(0x7FFF_FF00, 0).is_err(),
            "a bogus pid has no process to signal"
        );
    }
}
