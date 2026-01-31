mod agent;
mod config;
mod executor;
mod ollama;
mod search;

use agent::Agent;
use config::Config;

#[tokio::main]
async fn main() {
    let config = Config::from_env();
    let agent = Agent::new(config);
    agent.run().await;
}
