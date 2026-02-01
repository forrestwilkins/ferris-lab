use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthChar;

fn parse_arg(arg: &str) -> Option<(String, String)> {
    if let Some((left, right)) = arg.split_once('=') {
        return Some((left.to_string(), right.to_string()));
    }
    if let Some((left, right)) = arg.split_once(':') {
        return Some((left.to_string(), right.to_string()));
    }
    None
}

fn main() -> io::Result<()> {
    let mut pairs = Vec::new();
    for arg in env::args().skip(1) {
        if let Some(pair) = parse_arg(&arg) {
            pairs.push(pair);
        }
    }

    if pairs.is_empty() {
        eprintln!("usage: log_mux agent-1=/path/to/fifo agent-2=/path/to/fifo");
        std::process::exit(2);
    }

    let (tx, rx) = mpsc::channel::<(String, String)>();

    for (agent_id, path) in &pairs {
        let agent_id = agent_id.clone();
        let path = path.clone();
        let tx = tx.clone();
        thread::spawn(move || {
            let file = match File::open(&path) {
                Ok(file) => file,
                Err(_) => return,
            };
            let reader = BufReader::new(file);
            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        if tx.send((agent_id.clone(), line)).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
    }
    drop(tx);

    let mut buffers: HashMap<String, Vec<String>> = HashMap::new();
    let mut last_seen: HashMap<String, Instant> = HashMap::new();
    let flush_after = Duration::from_millis(150);

    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok((agent_id, line)) => {
                buffers.entry(agent_id.clone()).or_default().push(line);
                last_seen.insert(agent_id, Instant::now());
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }

        let now = Instant::now();
        let mut ready = Vec::new();
        for (agent_id, lines) in &buffers {
            if !lines.is_empty() {
                if let Some(ts) = last_seen.get(agent_id) {
                    if now.duration_since(*ts) >= flush_after {
                        ready.push(agent_id.clone());
                    }
                }
            }
        }

        for agent_id in ready {
            if let Some(lines) = buffers.get_mut(&agent_id) {
                if lines.is_empty() {
                    continue;
                }
                print_card(&agent_id, &lines);
                lines.clear();
            }
        }
    }

    for (agent_id, lines) in buffers {
        if lines.is_empty() {
            continue;
        }
        print_card(&agent_id, &lines);
    }

    Ok(())
}

fn print_card(agent_id: &str, lines: &[String]) {
    let width = 90usize;
    let title = format!(" Agent: {} ", agent_id);
    let title_len = title.chars().count();
    let inner = width.saturating_sub(2);
    let left_pad = inner.saturating_sub(title_len) / 2;
    let right_pad = inner.saturating_sub(title_len + left_pad);

    println!();
    println!("┌{}{}{}┐", "─".repeat(left_pad), title, "─".repeat(right_pad));
    let content_width = inner.saturating_sub(2);
    for line in lines {
        for chunk in wrap_line(line, content_width) {
            let padded = pad_to_width(&chunk, content_width);
            println!("│ {} │", padded);
        }
    }
    println!("└{}┘", "─".repeat(inner));
}

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    if line.is_empty() {
        return vec![String::new()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            current.push(ch);
            if let Some('[') = chars.peek().copied() {
                current.push('[');
                chars.next();
                while let Some(next) = chars.next() {
                    current.push(next);
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }

        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > width && !current.is_empty() {
            chunks.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }

    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn visible_width(line: &str) -> usize {
    let mut width = 0usize;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if let Some('[') = chars.peek().copied() {
                chars.next();
                while let Some(next) = chars.next() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }
        width += UnicodeWidthChar::width(ch).unwrap_or(0);
    }
    width
}

fn pad_to_width(line: &str, width: usize) -> String {
    let current = visible_width(line);
    if current >= width {
        return line.to_string();
    }
    let mut padded = String::with_capacity(line.len() + (width - current));
    padded.push_str(line);
    padded.push_str(&" ".repeat(width - current));
    padded
}
