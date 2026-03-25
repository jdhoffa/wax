//! Lightweight user-facing progress messages for long-running CLI commands.

/// Writes coarse progress updates to stderr without affecting stdout output.
#[derive(Debug, Clone, Copy)]
pub struct ProgressReporter {
    enabled: bool,
}

impl ProgressReporter {
    /// Create a progress reporter.
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Print a single status line.
    pub fn stage(&self, message: &str) {
        if self.enabled {
            eprintln!("{message}");
        }
    }

    /// Print a periodic progress update for a bounded loop.
    pub fn item_progress(&self, label: &str, current: usize, total: usize) {
        if !self.enabled || total == 0 {
            return;
        }

        if current == 1 || current == total || current.is_multiple_of(10) {
            eprintln!("{label}: {current}/{total}...");
        }
    }
}
