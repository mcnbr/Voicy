fn main() {
    let api = hf_hub::api::sync::Api::new().unwrap();
    let repo = api.model("QuantFactory/Whisper-Large-V3-Turbo-GGUF".to_string());
    println!("Fetching config.json...");
    let config = repo.get("config.json").unwrap();
    println!("Config downloaded to {:?}", config);
}
