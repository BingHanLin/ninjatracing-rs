[task list](#task-port-ninjatracing-to-rust)

# ninjatracing

Rust port of the `ninjatracing` Python script. Converts Ninja build logs (`.ninja_log`) to Chrome Tracing format, enabling visualization of build performance in `chrome://tracing` or [perfetto.dev](https://ui.perfetto.dev/).

## Installation

### As a CLI Tool

```bash
cargo install --path .
```

### As a Library

Add this to your `Cargo.toml`:

```toml
[dependencies]
ninjatracing = { path = ".", default-features = false }
```

## Usage

### CLI

```bash
ninjatracing .ninja_log > trace.json
ninjatracing --showall .ninja_log > trace.json
ninjatracing --embed-time-trace .ninja_log > trace.json
```

### Library

```rust
use ninjatracing::{log_to_dicts, TracingOptions};
use std::fs::File;
use std::io::BufReader;

fn main() -> anyhow::Result<()> {
    let file = File::open(".ninja_log")?;
    let reader = BufReader::new(file);
    let options = TracingOptions {
        show_all: false,
        granularity: 50000,
        embed_time_trace: false,
    };
    
    let events = log_to_dicts(reader, None, 42, &options)?;
    println!("Found {} events", events.len());
    Ok(())
}
```
