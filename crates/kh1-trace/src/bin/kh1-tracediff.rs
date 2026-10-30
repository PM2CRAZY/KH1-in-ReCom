//! `kh1-tracediff <kh1 trace> <playground trace>`: compares two traces frame by frame
//! (`kh1_trace::diff`) and prints the first divergence and each field's errors.
//! Exit code: 0 match, 1 divergence, 2 usage or read error.

use std::path::Path;

use kh1_trace::diff::{self, Align, Tolerance};
use kh1_trace::schema;

const USAGE: &str = "kh1-tracediff <a.csv> <b.csv> [--from-start] [--frames N] [--tol-pos U] [--tol-angle RAD] [--tol-frame F]\n\
  a is usually the KH1 trace (kh1-trace-<n>.csv), b the playground's (--replay ... --trace b.csv).\n\
  Lines the traces up at their first input (or at row 0 with --from-start), then compares state,\n\
  motion request / entry / frame, position and facing relative to the start, combo counter and row.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return;
    }
    let mut files = Vec::new();
    let mut tol = Tolerance::default();
    let mut align = Align::FirstInput;
    let mut frames = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let mut val = |name: &str| -> f32 {
            i += 1;
            match args.get(i).and_then(|v| v.parse().ok()) {
                Some(v) => v,
                None => {
                    eprintln!("{name} needs a number\n{USAGE}");
                    std::process::exit(2);
                }
            }
        };
        match a {
            "--from-start" => align = Align::Start,
            "--frames" => frames = Some(val("--frames") as usize),
            "--tol-pos" => tol.pos = val("--tol-pos"),
            "--tol-angle" => tol.angle = val("--tol-angle"),
            "--tol-frame" => tol.frame = val("--tol-frame"),
            _ if a.starts_with("--") => {
                eprintln!("unknown option {a}\n{USAGE}");
                std::process::exit(2);
            }
            _ => files.push(a.to_string()),
        }
        i += 1;
    }
    if files.len() != 2 {
        eprintln!("{USAGE}");
        std::process::exit(2);
    }
    let read = |p: &str| match schema::read_file(Path::new(p)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let (a, b) = (read(&files[0]), read(&files[1]));
    let report = diff::compare(&a.rows, &b.rows, align, tol, frames);
    print!("{}", diff::format(&report, &files[0], &files[1]));
    for (name, t) in [(&files[0], &a), (&files[1], &b)] {
        if t.has("in_cam_raw") && t.rows.iter().any(|r| r.in_cam_raw.is_finite()) {
            let (n, ok, max) = diff::camera_check(&t.rows, 1e-3);
            println!("camera check ({name}): {ok} of {n} tilted can-act frames reproduce KH1's move vector within 0.001 rad (max error {max:.5} rad)");
        }
    }
    std::process::exit(if report.matches() { 0 } else { 1 });
}
