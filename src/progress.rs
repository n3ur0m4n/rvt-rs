//! Progress of long reads, for front ends that show it.
//!
//! The library reports `(stage, done, total)` as it goes; nothing is
//! reported until a receiver is installed.

use std::sync::OnceLock;

type Sink = Box<dyn Fn(&str, usize, usize) + Send + Sync>;
static SINK: OnceLock<Sink> = OnceLock::new();

/// Install the receiver of progress reports. Only the first call counts.
pub fn set_sink(sink: impl Fn(&str, usize, usize) + Send + Sync + 'static) {
    let _ = SINK.set(Box::new(sink));
}

pub(crate) fn report(stage: &str, done: usize, total: usize) {
    if let Some(sink) = SINK.get() {
        sink(stage, done, total)
    }
}
