// TODO: Clean up this file - lots of duplication

use owo_colors::OwoColorize;
use std::collections::HashSet;
use std::env;
use std::sync::Mutex;
use std::sync::OnceLock;

/// Robot emoji prefix for all agent output
const ROBOT: &str = "🤖";

static SHOW_AGENT_ID: OnceLock<bool> = OnceLock::new();

/// Track recently logged peer messages to avoid duplicates
static RECENT_MESSAGES: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn show_agent_id() -> bool {
    *SHOW_AGENT_ID.get_or_init(|| {
        let value = env::var("FERRIS_MERGED_OUTPUT")
            .unwrap_or_else(|_| "0".to_string())
            .to_lowercase();
        !(value == "1" || value == "true" || value == "yes")
    })
}

fn agent_label(agent_id: &str) -> Option<String> {
    if show_agent_id() {
        Some(format!("[{}]", agent_id))
    } else {
        None
    }
}

/// Check if a message was recently logged, and mark it as seen
fn is_duplicate(key: &str) -> bool {
    let mut guard = RECENT_MESSAGES.lock().unwrap();
    let set = guard.get_or_insert_with(HashSet::new);

    // Keep the set from growing unbounded (simple cleanup)
    if set.len() > 100 {
        set.clear();
    }

    !set.insert(key.to_string())
}

/// Print an agent status message (cyan)
pub fn agent_status(agent_id: &str, message: &str) {
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!("{} {} {}", ROBOT, label.cyan().bold(), message.cyan());
    } else {
        println!("{} {}", ROBOT, message.cyan());
    }
}

/// Print an agent info message (white/default)
pub fn agent_info(agent_id: &str, message: &str) {
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!("{} {} {}", ROBOT, label.bright_white().bold(), message);
    } else {
        println!("{} {}", ROBOT, message);
    }
}

/// Print an agent success message (green)
pub fn agent_success(agent_id: &str, message: &str) {
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!("{} {} {}", ROBOT, label.green().bold(), message.green());
    } else {
        println!("{} {}", ROBOT, message.green());
    }
}

/// Print an agent warning message (yellow)
pub fn agent_warn(agent_id: &str, message: &str) {
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!("{} {} {}", ROBOT, label.yellow().bold(), message.yellow());
    } else {
        println!("{} {}", ROBOT, message.yellow());
    }
}

/// Print an agent error message (red)
pub fn agent_error(agent_id: &str, message: &str) {
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!("{} {} {}", ROBOT, label.red().bold(), message.red());
    } else {
        println!("{} {}", ROBOT, message.red());
    }
}

/// Print a peer communication message - outgoing text (magenta with arrow)
pub fn peer_send_text(agent_id: &str, content: &str) {
    let key = format!("send:text:{}:{}", agent_id, content);
    if is_duplicate(&key) {
        return;
    }
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!(
            "{} {} {} {}",
            ROBOT,
            label.magenta().bold(),
            ">>".magenta().bold(),
            content.magenta()
        );
    } else {
        println!("{} {} {}", ROBOT, ">>".magenta().bold(), content.magenta());
    }
}

/// Print a peer communication message - incoming text (blue with arrow)
pub fn peer_recv_text(agent_id: &str, content: &str) {
    let key = format!("recv:text:{}:{}", agent_id, content);
    if is_duplicate(&key) {
        return;
    }
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!(
            "{} {} {} {}",
            ROBOT,
            label.blue().bold(),
            "<<".blue().bold(),
            content.blue()
        );
    } else {
        println!("{} {} {}", ROBOT, "<<".blue().bold(), content.blue());
    }
}

/// Print a peer communication message - outgoing number (magenta with arrow)
pub fn peer_send_number(agent_id: &str, value: u64) {
    let key = format!("send:number:{}:{}", agent_id, value);
    if is_duplicate(&key) {
        return;
    }
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!(
            "{} {} {} {}",
            ROBOT,
            label.magenta().bold(),
            ">>".magenta().bold(),
            format!("number: {}", value).magenta()
        );
    } else {
        println!(
            "{} {} {}",
            ROBOT,
            ">>".magenta().bold(),
            format!("number: {}", value).magenta()
        );
    }
}

/// Print a peer communication message - incoming number (blue with arrow)
pub fn peer_recv_number(agent_id: &str, value: u64) {
    let key = format!("recv:number:{}:{}", agent_id, value);
    if is_duplicate(&key) {
        return;
    }
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!(
            "{} {} {} {}",
            ROBOT,
            label.blue().bold(),
            "<<".blue().bold(),
            format!("number: {}", value).blue()
        );
    } else {
        println!(
            "{} {} {}",
            ROBOT,
            "<<".blue().bold(),
            format!("number: {}", value).blue()
        );
    }
}

/// Print a peer connection event (bright magenta)
pub fn peer_event(agent_id: &str, message: &str) {
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!(
            "{} {} {} {}",
            ROBOT,
            label.bright_magenta().bold(),
            "⚡".bright_magenta(),
            message.bright_magenta()
        );
    } else {
        println!(
            "{} {} {}",
            ROBOT,
            "⚡".bright_magenta(),
            message.bright_magenta()
        );
    }
}

/// Print a startup banner
pub fn startup_banner(agent_id: &str) {
    println!();
    println!("{}", "═".repeat(50).bright_cyan());
    println!(
        "{}  {} {}",
        ROBOT,
        "FERRIS LAB".bright_cyan().bold(),
        format!("- {}", agent_id).bright_white()
    );
    println!("{}", "═".repeat(50).bright_cyan());
    println!();
}

/// Print a section header
pub fn section(title: &str) {
    println!();
    println!(
        "{}  {}",
        "─".repeat(3).bright_white().dimmed(),
        title.bright_white().bold()
    );
}

/// Print agent ready message with a nice box
pub fn agent_ready(agent_id: &str, peer_count: usize) {
    println!();
    println!("{}", "┌─────────────────────────────────────────┐".green());
    println!(
        "{}  {} Agent {} is ready!",
        "│".green(),
        ROBOT,
        agent_id.green().bold()
    );
    println!(
        "{}  📡 Connected peers: {}",
        "│".green(),
        peer_count.to_string().bright_white().bold()
    );
    println!("{}", "└─────────────────────────────────────────┘".green());
    println!();
}

/// Print configuration info
pub fn config_item(agent_id: &str, key: &str, value: &str) {
    if let Some(label) = agent_label(agent_id) {
        println!(
            "{} {} {} {}",
            ROBOT,
            label.dimmed(),
            format!("{}:", key).bright_white(),
            value.bright_cyan()
        );
    } else {
        println!(
            "{} {} {}",
            ROBOT,
            format!("{}:", key).bright_white(),
            value.bright_cyan()
        );
    }
}

/// Print a code block
pub fn code_block(agent_id: &str, code: &str) {
    println!();
    if let Some(label) = agent_label(agent_id) {
        println!("{} {} Generated code:", ROBOT, label.bright_white().bold());
    } else {
        println!("{} Generated code:", ROBOT);
    }
    println!("{}", "```".dimmed());
    for line in code.lines() {
        println!("  {}", line.bright_yellow());
    }
    println!("{}", "```".dimmed());
}
