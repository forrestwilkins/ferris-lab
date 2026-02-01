use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};
use terminal_size::{terminal_size, Width};
use unicode_width::UnicodeWidthChar;

const RESET: &str = "\x1b[0m";

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
    let width = terminal_width().unwrap_or(90).max(60);
    let title = format!(" Agent: {} ", agent_id);
    let title_len = title.chars().count();
    let inner = width.saturating_sub(2);
    let left_pad = inner.saturating_sub(title_len) / 2;
    let right_pad = inner.saturating_sub(title_len + left_pad);

    println!();
    println!(
        "{}┌{}{}{}┐{}",
        RESET,
        "─".repeat(left_pad),
        title,
        "─".repeat(right_pad),
        RESET
    );
    let content_width = inner.saturating_sub(2);
    let trimmed = trim_empty_lines(lines);
    println!("{}│ {} │{}", RESET, " ".repeat(content_width), RESET);
    for line in trimmed {
        for chunk in wrap_line_words(line, content_width) {
            let padded = pad_to_width(&chunk, content_width);
            println!("{}│ {} │{}", RESET, padded, RESET);
        }
    }
    println!("{}│ {} │{}", RESET, " ".repeat(content_width), RESET);
    println!("{}└{}┘{}", RESET, "─".repeat(inner), RESET);
}

fn trim_empty_lines<'a>(lines: &'a [String]) -> &'a [String] {
    let mut start = 0usize;
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    &lines[start..end]
}

fn terminal_width() -> Option<usize> {
    if let Some((Width(w), _)) = terminal_size() {
        if w > 0 {
            return Some(w as usize);
        }
    }
    if let Ok(columns) = env::var("COLUMNS") {
        if let Ok(value) = columns.parse::<usize>() {
            if value > 0 {
                return Some(value);
            }
        }
    }
    None
}

fn wrap_line_words(line: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    if line.is_empty() {
        return vec![String::new()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut active_sgr = String::new();

    let mut word = String::new();
    let mut word_width = 0usize;
    let mut word_has_text = false;

    let flush_word = |current: &mut String,
                      current_width: &mut usize,
                      active_sgr: &str,
                      word: &mut String,
                      word_width: &mut usize,
                      word_has_text: &mut bool,
                      chunks: &mut Vec<String>| {
        if word.is_empty() {
            return;
        }
        if !*word_has_text {
            word.clear();
            *word_width = 0;
            return;
        }
        if *word_width > width {
            for part in split_word(word, width) {
                if !current.is_empty() {
                    chunks.push(std::mem::take(current));
                    *current_width = 0;
                }
                let mut line = String::new();
                if !active_sgr.is_empty() {
                    line.push_str(active_sgr);
                }
                line.push_str(&part);
                chunks.push(line);
            }
        } else if *current_width + *word_width > width && !current.is_empty() {
            chunks.push(std::mem::take(current));
            *current_width = 0;
            if !active_sgr.is_empty() {
                current.push_str(active_sgr);
            }
            current.push_str(word);
            *current_width += *word_width;
        } else {
            if current.is_empty() && !active_sgr.is_empty() {
                current.push_str(active_sgr);
            }
            current.push_str(word);
            *current_width += *word_width;
        }
        word.clear();
        *word_width = 0;
        *word_has_text = false;
    };

    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            let mut seq = String::new();
            seq.push(ch);
            if let Some('[') = chars.peek().copied() {
                seq.push('[');
                chars.next();
                while let Some(next) = chars.next() {
                    seq.push(next);
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            update_sgr_state(&seq, &mut active_sgr);
            word.push_str(&seq);
            continue;
        }

        if ch.is_whitespace() {
            flush_word(
                &mut current,
                &mut current_width,
                &active_sgr,
                &mut word,
                &mut word_width,
                &mut word_has_text,
                &mut chunks,
            );
            if !current.is_empty() {
                if current_width + 1 > width {
                    chunks.push(std::mem::take(&mut current));
                    current_width = 0;
                } else {
                    current.push(' ');
                    current_width += 1;
                }
            }
            continue;
        }

        word.push(ch);
        word_width += UnicodeWidthChar::width(ch).unwrap_or(0);
        word_has_text = true;
    }

    flush_word(
        &mut current,
        &mut current_width,
        &active_sgr,
        &mut word,
        &mut word_width,
        &mut word_has_text,
        &mut chunks,
    );

    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.is_empty() {
        chunks.push(String::new());
    }

    chunks
}

fn split_word(word: &str, width: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut chars = word.chars().peekable();
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
            parts.push(current);
            current = String::new();
            current_width = 0;
        }
        current.push(ch);
        current_width += ch_width;
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn update_sgr_state(seq: &str, active: &mut String) {
    if !seq.ends_with('m') {
        return;
    }
    let seq_body = seq.trim_start_matches("\x1b[").trim_end_matches('m');
    if seq_body.is_empty() || seq_body.split(';').any(|part| part == "0") {
        active.clear();
    } else {
        *active = seq.to_string();
    }
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
    if line.contains('\x1b') && !line.ends_with(RESET) {
        padded.push_str(RESET);
    }
    padded.push_str(&" ".repeat(width - current));
    padded
}
