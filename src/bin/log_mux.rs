use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

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
                println!();
                println!("---- [{}] ----", agent_id);
                for line in lines.drain(..) {
                    println!("{}", line);
                }
            }
        }
    }

    for (agent_id, lines) in buffers {
        if lines.is_empty() {
            continue;
        }
        println!();
        println!("---- [{}] ----", agent_id);
        for line in lines {
            println!("{}", line);
        }
    }

    Ok(())
}
