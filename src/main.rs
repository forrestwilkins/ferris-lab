mod agent;
mod config;
mod executor;
mod ollama;
mod search;
mod writer;

use agent::Agent;
use config::Config;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    let agent = Agent::new(config);
    agent.run().await;
}
