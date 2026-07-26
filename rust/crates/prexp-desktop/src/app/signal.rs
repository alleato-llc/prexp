//! State for the send-signal flow: the signal a user can pick and the pending
//! confirmation prompt for a chosen process.

use std::fmt;

/// A POSIX signal the UI can send. Numbers are the macOS/BSD values (which match
/// Linux for this common set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signal {
    Term,
    Kill,
    Int,
    Hup,
    Quit,
    Stop,
    Cont,
}

impl Signal {
    pub const ALL: [Signal; 7] = [
        Signal::Term,
        Signal::Kill,
        Signal::Int,
        Signal::Hup,
        Signal::Quit,
        Signal::Stop,
        Signal::Cont,
    ];

    /// The signal number passed to `kill(2)`.
    pub fn number(self) -> i32 {
        match self {
            Signal::Hup => 1,
            Signal::Int => 2,
            Signal::Quit => 3,
            Signal::Kill => 9,
            Signal::Term => 15,
            Signal::Stop => 17,
            Signal::Cont => 19,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Signal::Term => "SIGTERM",
            Signal::Kill => "SIGKILL",
            Signal::Int => "SIGINT",
            Signal::Hup => "SIGHUP",
            Signal::Quit => "SIGQUIT",
            Signal::Stop => "SIGSTOP",
            Signal::Cont => "SIGCONT",
        }
    }
}

impl fmt::Display for Signal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.name(), self.number())
    }
}

/// A pending "send signal to this process" confirmation.
pub struct SignalPrompt {
    pub pid: i32,
    pub name: String,
    pub signal: Signal,
}

impl SignalPrompt {
    /// A prompt for `pid`/`name`, defaulting to the graceful `SIGTERM`.
    pub fn new(pid: i32, name: String) -> Self {
        Self {
            pid,
            name,
            signal: Signal::Term,
        }
    }
}
