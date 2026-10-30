//! KH1 trace tool (`docs/kh1-trace.md`, roadmap M1): checks the KH1 Rust rewrite against real
//! KH1 frame by frame.
//!
//! - On Windows the crate is also `kh1_trace.dll`: OpenKH Panacea loads it into
//!   `KINGDOM HEARTS FINAL MIX.exe` (Steam). It hooks nothing but one read-only logging point and
//!   writes Sora's state and inputs per frame while recording (F9).
//! - [`schema`]: the trace CSV, shared with `kh1-playground --trace` / `--replay`.
//! - [`diff`]: frame-by-frame comparison, used by the `kh1-tracediff` program.
//! - [`kh1`]: the KH1 Steam addresses the logger reads.

pub mod diff;
pub mod kh1;
pub mod schema;

#[cfg(windows)]
mod win;
