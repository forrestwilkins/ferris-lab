use owo_colors::OwoColorize;

/// Robot emoji prefix for all agent output
const ROBOT: &str = "🤖";

/// Print an agent status message (cyan)
pub fn agent_status(agent_id: &str, message: &str) {
    println!();
    println!(
        "{} {} {}",
        ROBOT,
        format!("[{}]", agent_id).cyan().bold(),
        message.cyan()
    );
}

/// Print an agent info message (white/default)
pub fn agent_info(agent_id: &str, message: &str) {
    println!();
    println!(
        "{} {} {}",
        ROBOT,
        format!("[{}]", agent_id).bright_white().bold(),
        message
    );
}

/// Print an agent success message (green)
pub fn agent_success(agent_id: &str, message: &str) {
    println!();
    println!(
        "{} {} {}",
        ROBOT,
        format!("[{}]", agent_id).green().bold(),
        message.green()
    );
}

/// Print an agent warning message (yellow)
pub fn agent_warn(agent_id: &str, message: &str) {
    println!();
    println!(
        "{} {} {}",
        ROBOT,
        format!("[{}]", agent_id).yellow().bold(),
        message.yellow()
    );
}

/// Print an agent error message (red)
pub fn agent_error(agent_id: &str, message: &str) {
    println!();
    println!(
        "{} {} {}",
        ROBOT,
        format!("[{}]", agent_id).red().bold(),
        message.red()
    );
}

/// Print a peer communication message - outgoing (magenta with arrow)
pub fn peer_send(agent_id: &str, message: &str) {
    println!();
    println!(
        "{} {} {} {}",
        ROBOT,
        format!("[{}]", agent_id).magenta().bold(),
        ">>".magenta().bold(),
        message.magenta()
    );
}

/// Print a peer communication message - incoming (blue with arrow)
pub fn peer_recv(agent_id: &str, message: &str) {
    println!();
    println!(
        "{} {} {} {}",
        ROBOT,
        format!("[{}]", agent_id).blue().bold(),
        "<<".blue().bold(),
        message.blue()
    );
}

/// Print a peer connection event (bright magenta)
pub fn peer_event(agent_id: &str, message: &str) {
    println!();
    println!(
        "{} {} {} {}",
        ROBOT,
        format!("[{}]", agent_id).bright_magenta().bold(),
        "⚡".bright_magenta(),
        message.bright_magenta()
    );
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
    println!(
        "{} {} {} {}",
        ROBOT,
        format!("[{}]", agent_id).dimmed(),
        format!("{}:", key).bright_white(),
        value.bright_cyan()
    );
}

/// Print a code block
pub fn code_block(agent_id: &str, code: &str) {
    println!();
    println!(
        "{} {} Generated code:",
        ROBOT,
        format!("[{}]", agent_id).bright_white().bold()
    );
    println!("{}", "```".dimmed());
    for line in code.lines() {
        println!("  {}", line.bright_yellow());
    }
    println!("{}", "```".dimmed());
}
