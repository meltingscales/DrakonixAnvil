use std::env;

const CF_BASE: &str = "https://api.curseforge.com/v1";

#[tokio::main]
async fn main() {
    let api_key = env::var("CF_API_KEY").unwrap_or_else(|_| {
        eprintln!("Usage: CF_API_KEY=<key> test-cf-api");
        std::process::exit(1);
    });

    let client = reqwest::Client::new();

    // 1. Basic auth check
    println!("=== Test 1: GET /games (auth check) ===");
    let resp = client
        .get(format!("{}/games", CF_BASE))
        .header("x-api-key", &api_key)
        .send()
        .await
        .unwrap();
    println!("Status: {}", resp.status());
    let body = resp.text().await.unwrap_or_default();
    println!("Body (first 200): {}\n", &body[..body.len().min(200)]);

    // 2. Search for a well-known modpack
    println!("=== Test 2: Search modpacks (query='all the mods') ===");
    let resp = client
        .get(format!("{}/mods/search", CF_BASE))
        .header("x-api-key", &api_key)
        .query(&[
            ("gameId", "432"),
            ("classId", "4471"),
            ("searchFilter", "all the mods"),
            ("pageSize", "3"),
        ])
        .send()
        .await
        .unwrap();
    println!("Status: {}", resp.status());
    let body = resp.text().await.unwrap_or_default();
    println!("Body (first 300): {}\n", &body[..body.len().min(300)]);

    // 3. Fetch a specific mod (FTB StoneBlock 4 = project 1373378)
    println!("=== Test 3: GET /mods/1373378 (FTB StoneBlock 4) ===");
    let resp = client
        .get(format!("{}/mods/1373378", CF_BASE))
        .header("x-api-key", &api_key)
        .send()
        .await
        .unwrap();
    println!("Status: {}", resp.status());
    let body = resp.text().await.unwrap_or_default();
    println!("Body (first 300): {}\n", &body[..body.len().min(300)]);
}
