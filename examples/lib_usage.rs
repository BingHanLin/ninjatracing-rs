use ninjatracing::{log_to_dicts, TracingOptions};
use std::io::Cursor;

fn main() {
    let log_content = "# ninja log v5\n100\t200\t0\tmy_output\tdeadbeef\n";
    let cursor = Cursor::new(log_content);
    
    let options = TracingOptions {
        show_all: true,
        granularity: 0,
        embed_time_trace: false,
    };
    
    let events = log_to_dicts(cursor, None, 42, &options).expect("Failed to parse log");
    
    println!("Parsed {} events", events.len());
    for event in events {
        println!("Event: {:?}", event);
    }
}
