//! Shared test material of the spike: language coverage cases, the benchmark
//! script and small helpers for timing.

pub mod cases;

use std::time::{Duration, Instant};

/// Median of `runs` timings of `work`.
pub fn median(runs: usize, mut work: impl FnMut()) -> Duration {
    let mut times: Vec<Duration> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            work();
            start.elapsed()
        })
        .collect();
    times.sort();
    times[runs / 2]
}

/// Milliseconds with two decimals.
pub fn ms(duration: Duration) -> String {
    format!("{:.2} ms", duration.as_secs_f64() * 1000.0)
}

/// `source` in a block of its own, so its `const`s do not clash with those of
/// other scripts in the same context (the block's value is the script's).
pub fn isolated(source: &str) -> String {
    format!("{{{source}\n}}")
}
