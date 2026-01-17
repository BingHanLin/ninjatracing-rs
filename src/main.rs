use anyhow::Result;
use clap::Parser;
use ninjatracing::tracer::{log_to_dicts, TracingOptions};
use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// .ninja_log file(s) to parse
    #[arg(required = true, num_args = 1..)]
    log_files: Vec<PathBuf>,

    /// Report on last build step for all outputs. Default is to report just on the last (possibly incremental) build
    #[arg(short = 'a', long)]
    showall: bool,

    /// Minimum length time-trace event to embed in microseconds
    #[arg(short = 'g', long, default_value_t = 50000)]
    granularity: u64,

    /// Embed clang -ftime-trace json file found adjacent to a target file
    #[arg(short = 'e', long = "embed-time-trace")]
    embed_time_trace: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let options = TracingOptions {
        show_all: args.showall,
        granularity: args.granularity,
        embed_time_trace: args.embed_time_trace,
    };

    let mut all_events = Vec::new();

    for (pid, log_file) in args.log_files.iter().enumerate() {
        let file = File::open(log_file)?;
        let reader = BufReader::new(file);

        let events = log_to_dicts(reader, Some(log_file), pid as u32, &options)?;
        all_events.extend(events);
    }

    serde_json::to_writer(io::stdout(), &all_events)?;

    Ok(())
}
