use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub start: u64,
    pub end: u64,
    pub targets: Vec<String>,
}

impl Target {
    pub fn new(start: u64, end: u64) -> Self {
        Self {
            start,
            end,
            targets: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct TraceEvent {
    pub name: String,
    #[serde(default)]
    pub cat: String,
    pub ph: String,
    pub ts: u64,
    pub dur: u64,
    pub pid: u32,
    pub tid: u32,
    #[serde(default)]
    pub args: HashMap<String, Value>,
}
