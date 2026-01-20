# ninjatracing-rs

Rust port of the `ninjatracing` Python script (originally from [https://github.com/nico/ninjatracing](https://github.com/nico/ninjatracing)). Converts Ninja build logs (`.ninja_log`) to Chrome Tracing format, enabling visualization of build performance in `chrome://tracing` or [perfetto.dev](https://ui.perfetto.dev/).

## Installation

### As a CLI Tool

**From crates.io (Recommended)**

```bash
cargo install ninjatracing
```

**From source**

```bash
cargo install --path .
```

### As a Library

Add this to your `Cargo.toml`:

```toml
[dependencies]
ninjatracing = "0.1"
```

Or with only the library (without CLI):

```toml
[dependencies]
ninjatracing = { version = "0.1", default-features = false }
```

## CLI Usage

```bash
ninjatracing [OPTIONS] <LOG_FILES>... > trace.json
```

### Arguments

| Argument | Description |
| :--- | :--- |
| `<LOG_FILES>...` | One or more `.ninja_log` files to parse. |
| `-a`, `--showall` | Report on last build step for all outputs. Default is to report just on the last (possibly incremental) build. |
| `-g`, `--granularity <US>` | Minimum length time-trace event to embed in microseconds (default: 50000). |
| `-e`, `--embed-time-trace` | Embed `clang -ftime-trace` json file found adjacent to a target file. |

### Examples

```bash
# Basic usage
ninjatracing .ninja_log > trace.json

# Show all incremental steps
ninjatracing --showall .ninja_log > trace.json

# Embed clang traces
ninjatracing --embed-time-trace .ninja_log > trace.json
```

## Library Usage

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
