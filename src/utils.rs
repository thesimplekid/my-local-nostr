use nostr_sdk::prelude::*;
use nostr_sdk::{Client, Keys};
use std::time::SystemTime;

/// Seconds since 1970.
#[must_use]
pub fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|x| x.as_secs())
        .unwrap_or(0)
}

// Creates the websocket client that is used for communicating with relays
pub async fn create_client(
    keys: Option<&Keys>,
    relays: Vec<String>,
    difficulty: u8,
) -> Result<Client> {
    let keys = match keys {
        Some(k) => k.to_owned(),
        None => Keys::generate(),
    };

    let opts = Options::new().wait_for_send(true).difficulty(difficulty);
    let client = Client::new_with_opts(&keys, opts);
    let relays = relays.iter().map(|url| (url, None)).collect();
    client.add_relays(relays).await?;
    client.connect().await;
    Ok(client)
}
