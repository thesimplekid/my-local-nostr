use nostr_sdk::event::tag::Tag;
use nostr_sdk::prelude::schnorr::Signature;
use nostr_sdk::prelude::*;
use tungstenite::Message as WsMessage;

use tracing::debug;

use crate::nauthz_grpc::event::TagEntry;

use crate::error::Error;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::{nauthz_grpc, utils};

#[derive(Clone)]
pub struct NostrClient {
    /// Nostr-sdk Client
    pub client: Client,
    /// Default relays to pull events from
    pub relays: HashSet<Url>,
}

impl NostrClient {
    pub async fn new(relays: &HashSet<Url>) -> Result<Self, Error> {
        debug!("Client Relays: {:?}", relays);
        Ok(Self {
            relays: relays.to_owned(),
            client: utils::create_client(None, relays.iter().map(|r| r.to_string()).collect(), 0)
                .await
                .unwrap(),
        })
    }

    pub async fn fetch_events(
        &self,
        events: &HashMap<EventId, Option<String>>,
    ) -> Result<Vec<Event>> {
        let event_relays: HashSet<Url> = events
            .values()
            .flatten()
            .filter(|r| !self.relays.contains(&Url::from_str(r).unwrap()))
            .flat_map(|r| Url::from_str(r))
            .collect();

        // Add the relays recommend in the `e` tag
        self.client
            .add_relays(event_relays.iter().map(|r| (r.to_string(), None)).collect())
            .await
            .unwrap();

        // debug!("Fetching relays: {:?}", self.client.relays().await);

        let events = self
            .client
            .get_events_of(
                vec![Filter::new().ids(events.keys().map(|k| k.to_hex()).collect::<Vec<String>>())],
                Some(Duration::from_secs(10)),
            )
            .await
            .unwrap();

        // Remove the relays recommended by `e` tag
        for r in event_relays {
            self.client.remove_relay(r).await.unwrap();
        }

        Ok(events)
    }

    pub async fn broadcast_events(
        &self,
        relay: &str,
        events: Arc<Vec<Event>>,
    ) -> Result<(), Error> {
        // Connect to relay
        let (mut socket, _) = tungstenite::connect(relay).expect("Can't connect to relay");

        for event in events.as_ref().iter() {
            // Send msg
            let msg = ClientMessage::new_event(event.to_owned()).as_json();
            socket
                .write_message(WsMessage::Text(msg.clone()))
                .expect("Impossible to send message");

            debug!("Sent event: {}", msg);
        }

        Ok(())
    }
}

impl From<&nauthz_grpc::Event> for nostr::Event {
    fn from(event: &nauthz_grpc::Event) -> nostr::Event {
        let id = EventId::from_slice(&event.id).unwrap();
        let pubkey = XOnlyPublicKey::from_slice(&event.pubkey).unwrap();
        let sig = Signature::from_slice(&event.sig).unwrap();
        let tags = event
            .tags
            .iter()
            .map(|t| <TagEntry as Into<Tag>>::into(t.clone()))
            .collect();

        Event {
            id,
            pubkey,
            created_at: event.created_at.into(),
            kind: Kind::from(event.kind),
            content: event.content.clone(),
            sig,
            tags,
        }
    }
}

impl From<TagEntry> for Tag {
    fn from(tag: TagEntry) -> Tag {
        Tag::parse(tag.values).unwrap()
    }
}
