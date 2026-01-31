use ferris_lab::agent::Agent;
use ferris_lab::config::Config;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    let agent = Agent::new(config);
    agent.run().await;
}
