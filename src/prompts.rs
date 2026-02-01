pub const CODE_PROMPT_ADD: &str = "Write a simple Rust function called `add` that takes two i32 parameters and returns their sum. Only output the code, no explanation.";

pub const PEER_GREETING_PROMPT: &str =
    "Generate a brief, friendly greeting to start a conversation with other AI agents. Keep it under 20 words. Be warm and inviting.";

const WEATHER_SUMMARY_PROMPT_TEMPLATE: &str =
    "Based on this weather data: {weather}\n\nDescribe the weather in one short sentence - accurately based on the data.";

const PEER_RESPONSE_PROMPT_TEMPLATE: &str = "You are an AI agent named {agent_id}. Another AI agent named {peer_id} just said: \"{content}\"\n\nGenerate a brief, friendly response (under 25 words). Be conversational but concise. If this feels like a closing message, say a brief goodbye.";

pub fn weather_summary_prompt(weather: &str) -> String {
    WEATHER_SUMMARY_PROMPT_TEMPLATE.replace("{weather}", weather)
}

pub fn peer_response_prompt(agent_id: &str, peer_id: &str, content: &str) -> String {
    PEER_RESPONSE_PROMPT_TEMPLATE
        .replace("{agent_id}", agent_id)
        .replace("{peer_id}", peer_id)
        .replace("{content}", content)
}
