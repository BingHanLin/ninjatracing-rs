use crate::models::Target;
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::io::BufRead;

pub fn read_targets<R: BufRead>(reader: R, show_all: bool) -> Result<Vec<Target>> {
    let mut lines = reader.lines();

    // Read header
    let header = lines
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty log file"))??;
    let version_regex = regex::Regex::new(r"^# ninja log v(\d+)$").unwrap();
    let captures = version_regex
        .captures(&header)
        .ok_or_else(|| anyhow::anyhow!("unrecognized ninja log version {:?}", header))?;

    let version: i32 = captures[1].parse()?;
    if !(5..=6).contains(&version) {
        bail!("unsupported ninja log version {}", version);
    }

    if version == 6 {
        // Skip header line (v6 specific behavior)
        if let Some(l) = lines.next() {
            l?; // consume
        }
    }

    let mut targets: HashMap<String, Target> = HashMap::new();
    let mut last_end_seen: u64 = 0;

    for line in lines {
        let line = line?;
        if line.starts_with('#') {
            continue;
        }

        // Format: start\tend\trestat\tname\tcmdhash
        let parts: Vec<&str> = line.trim().split('\t').collect();
        if parts.len() < 5 {
            continue;
        }

        let start: u64 = parts[0].parse()?;
        let end: u64 = parts[1].parse()?;
        let name = parts[3];
        let cmdhash = parts[4];

        // Incremental build check
        if !show_all && end < last_end_seen {
            targets.clear();
        }
        last_end_seen = end;

        let target = targets
            .entry(cmdhash.to_string())
            .or_insert_with(|| Target::new(start, end));
        target.targets.push(name.to_string());
    }

    let mut result: Vec<Target> = targets.into_values().collect();
    result.sort_by_key(|t| t.end);
    result.reverse();

    Ok(result)
}
