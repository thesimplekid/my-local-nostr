use nostr_sdk::event::tag::Tag;
use nostr_sdk::prelude::schnorr::Signature;
use nostr_sdk::prelude::*;

use crate::nauthz_grpc::event::TagEntry;

use crate::error::Error;

use std::collections::{HashMap, HashSet};
use std::str::FromStr;

use crate::{nauthz_grpc, utils};

#[derive(Clone)]
pub struct NostrClient {
    pub client: Client,
    pub relays: HashSet<Url>,
}

impl NostrClient {
    pub async fn new(relays: &HashSet<Url>) -> Result<Self, Error> {
        Ok(Self {
            relays: relays.to_owned(),
            client: utils::create_client(None, relays.iter().map(|r| r.to_string()).collect(), 0)
                .await
                .unwrap(),
        })
    }

    pub async fn fetch_events(
        &self,
        events: HashMap<EventId, Option<String>>,
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

        let events = self
            .client
            .get_events_of(
                vec![Filter::new().ids(
                    events
                        .keys()
                        .map(|k| k.to_string())
                        .collect::<Vec<String>>(),
                )],
                None,
            )
            .await
            .unwrap();

        // Remove the relays recommended by `e` tag
        for r in event_relays {
            self.client.remove_relay(r).await.unwrap();
        }

        Ok(events)
    }

    pub async fn broadcast_events(&self, relay: &str, events: Vec<Event>) -> Result<(), Error> {
        self.client
            .add_relay(Url::from_str(relay).unwrap(), None)
            .await
            .unwrap();
        for event in events {
            self.client.send_event_to(relay, event).await.unwrap();
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
