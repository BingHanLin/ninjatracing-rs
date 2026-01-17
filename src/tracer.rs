use crate::models::{Target, TraceEvent};
use crate::parser::read_targets;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub struct TracingOptions {
    pub show_all: bool,
    pub granularity: u64, // microseconds
    pub embed_time_trace: bool,
}

pub struct Threads {
    workers: Vec<u64>,
}

impl Default for Threads {
    fn default() -> Self {
        Self::new()
    }
}

impl Threads {
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
        }
    }

    pub fn alloc(&mut self, target: &Target) -> u32 {
        for (i, worker_end) in self.workers.iter_mut().enumerate() {
            if *worker_end >= target.end {
                *worker_end = target.start;
                return i as u32;
            }
        }
        self.workers.push(target.start);
        (self.workers.len() - 1) as u32
    }
}

pub fn log_to_dicts<R: std::io::BufRead>(
    log: R,
    log_path: Option<&Path>,
    pid: u32,
    options: &TracingOptions,
) -> Result<Vec<TraceEvent>> {
    let targets = read_targets(log, options.show_all)?;
    let mut threads = Threads::new();
    let mut events = Vec::new();

    for target in targets {
        let tid = threads.alloc(&target);

        // Ninja event
        events.push(TraceEvent {
            name: target.targets.join(", "),
            cat: "targets".to_string(),
            ph: "X".to_string(),
            ts: target.start * 1000,
            dur: (target.end - target.start) * 1000,
            pid,
            tid,
            args: HashMap::new(),
        });

        if options.embed_time_trace {
            if let Some(dir) = log_path.and_then(|p| p.parent()) {
                if let Ok(embedded_events) = embed_time_trace(dir, &target, pid, tid, options) {
                    events.extend(embedded_events);
                }
            }
        }
    }

    Ok(events)
}

fn embed_time_trace(
    ninja_log_dir: &Path,
    target: &Target,
    pid: u32,
    tid: u32,
    options: &TracingOptions,
) -> Result<Vec<TraceEvent>> {
    let mut events = Vec::new();
    for t in &target.targets {
        let o_path = ninja_log_dir.join(t);
        let parent = o_path.parent();
        let file_stem = o_path.file_stem();

        if let (Some(parent), Some(stem)) = (parent, file_stem) {
            let json_path = parent.join(format!("{}.json", stem.to_string_lossy()));

            if json_path.exists() {
                let file =
                    File::open(&json_path).context(format!("Failed to open {:?}", json_path))?;
                let reader = BufReader::new(file);
                let embedded_events = parse_clang_trace(reader, target, pid, tid, options)?;
                events.extend(embedded_events);
            }
        }
    }
    Ok(events)
}

fn parse_clang_trace<R: std::io::Read>(
    reader: R,
    target: &Target,
    pid: u32,
    tid: u32,
    options: &TracingOptions,
) -> Result<Vec<TraceEvent>> {
    let trace_data: serde_json::Value =
        serde_json::from_reader(reader).unwrap_or(serde_json::Value::Null);
    let mut events = Vec::new();

    if let Some(trace_events) = trace_data.get("traceEvents").and_then(|v| v.as_array()) {
        let ninja_time_us = (target.end - target.start) * 1000;

        for event_val in trace_events {
            let ph = event_val["ph"].as_str().unwrap_or("");
            let dur = event_val["dur"].as_u64().unwrap_or(0);
            let name = event_val["name"].as_str().unwrap_or("");

            if ph == "X" && dur >= options.granularity && !name.starts_with("Total") {
                if dur > ninja_time_us {
                    eprintln!("Inconsistent timing found (clang time > ninja time).");
                }

                let mut args = HashMap::new();
                if let Some(a) = event_val.get("args").and_then(|v| v.as_object()) {
                    for (k, v) in a {
                        args.insert(k.clone(), v.clone());
                    }
                }

                let ts_raw = event_val["ts"].as_u64().unwrap_or(0);

                events.push(TraceEvent {
                    name: name.to_string(),
                    cat: event_val["cat"].as_str().unwrap_or("").to_string(),
                    ph: ph.to_string(),
                    ts: ts_raw + (target.start * 1000),
                    dur,
                    pid,
                    tid,
                    args,
                });
            }
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn test_simple() {
        let log = Cursor::new("# ninja log v5\n100\t200\t0\tmy_output\tdeadbeef\n50\t120\t0\tmy_first_output\t0afef\n");
        let options = TracingOptions {
            show_all: true,
            granularity: 0,
            embed_time_trace: false,
        };
        let events = log_to_dicts(log, None, 42, &options).unwrap();

        assert_eq!(events.len(), 2);

        let e1 = &events[0];
        assert_eq!(e1.name, "my_output");
        assert_eq!(e1.ts, 100000); // 100 * 1000
        assert_eq!(e1.dur, 100000); // (200-100) * 1000
        assert_eq!(e1.pid, 42);
        assert_eq!(e1.tid, 0); // First thread

        let e2 = &events[1];
        assert_eq!(e2.name, "my_first_output");
        assert_eq!(e2.ts, 50000);
        assert_eq!(e2.dur, 70000); // 120-50
        assert_eq!(e2.pid, 42);
        assert_eq!(e2.tid, 1); // Second thread because 50 < 200 (overlap)
    }

    #[test]
    fn test_last_only() {
        // Test the behavior without --showall.
        let log = Cursor::new("# ninja log v5\n100\t200\t0\tmy_output\tdeadbeef\n50\t120\t0\tmy_first_output\t0afef\n");
        let options = TracingOptions {
            show_all: false,
            granularity: 0,
            embed_time_trace: false,
        };
        let events = log_to_dicts(log, None, 42, &options).unwrap();

        // "my_first_output" (end 120) < "my_output" (end 200).
        // `last_end_seen` logic in parser:
        // 1. 100-200. last_end=200.
        // 2. 50-120. 120 < 200. Clear targets!
        // 3. Add 50-120.
        // Result: only my_first_output.

        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.name, "my_first_output");
        assert_eq!(e.tid, 0); // Reset threads? No, `Threads` is new per `log_to_dicts`.
    }

    #[test]
    fn test_multiple_outputs() {
        let log = Cursor::new(
            "# ninja log v5\n100\t200\t0\toutput\tdeadbeef\n100\t200\t0\tother_output\tdeadbeef\n",
        );
        let options = TracingOptions {
            show_all: true,
            granularity: 0,
            embed_time_trace: false,
        };
        let events = log_to_dicts(log, None, 42, &options).unwrap();

        assert_eq!(events.len(), 1);
        let e = &events[0];
        // Names are joined
        assert!(e.name.contains("output"));
        assert!(e.name.contains("other_output")); // Order depends on hashmap iteration in parser? No, `targets.entry(..).or_insert(..).targets.push(..)`
                                                  // `parser.rs` preserves order of lines for same hash.
                                                  // "output" then "other_output".
        assert_eq!(e.name, "output, other_output");
        assert_eq!(e.ts, 100000);
        assert_eq!(e.dur, 100000);
    }

    #[test]
    fn test_trace() {
        let trace_json = json!({
            "traceEvents": [
                { "dur": 1500, "name": "LongEvent", "ph": "X", "pid": 1, "tid": 12345, "ts": 1000 },
                { "dur": 500, "name": "TooShort", "ph": "X", "pid": 1, "tid": 12345, "ts": 1000 },
                { "args": { "avg ms": 1, "count": 2 }, "dur": 1111, "name": "Total Count", "ph": "X", "pid": 1, "tid": 12345, "ts": 0 },
                { "args": { "name": "clang" }, "cat": "", "name": "process_name", "ph": "M", "pid": 1, "tid": 0, "ts": 0 }
            ]
        });
        let trace_str = trace_json.to_string();
        let reader = Cursor::new(trace_str);

        let target = Target {
            start: 5,
            end: 10,
            targets: vec![],
        }; // 5ms to 10ms. Ninja time = 5ms = 5000us.

        let options = TracingOptions {
            show_all: false,
            granularity: 1000,
            embed_time_trace: false,
        };

        let events = parse_clang_trace(reader, &target, 42, 5, &options).unwrap();

        // process_name: ph M. Drop.

        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.name, "LongEvent");
        assert_eq!(e.ts, 6000);
        assert_eq!(e.pid, 42);
        assert_eq!(e.tid, 5);
    }

    #[test]
    fn test_comments() {
        let log = Cursor::new("# ninja log v5\n#\n100\t200\t0\tmy_output\tdeadbeef\n# 100\t666\t0\tignored_output\tdeadbeef\n50\t120\t0\tmy_first_output\t0afef\n# lastline");
        let options = TracingOptions {
            show_all: true,
            granularity: 0,
            embed_time_trace: false,
        };
        let events = log_to_dicts(log, None, 42, &options).unwrap();

        assert_eq!(events.len(), 2);
    }
}
