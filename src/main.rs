use axum::http::HeaderMap;
use db::Status;
use error::Error;
use nostr_sdk::{EventId, Tag};
use tokio::sync::{Mutex, MutexGuard};
use tonic::{transport::Server, Request, Response};

use nauthz_grpc::authorization_server::{Authorization, AuthorizationServer};
use nauthz_grpc::{Decision, Event, EventReply, EventRequest};

use crate::client::NostrClient;
use crate::config::Settings;
use crate::repo::Repo;

use serde::{Deserialize, Serialize};

use axum::{
    extract::{Json, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};

use std::collections::HashMap;
use std::sync::Arc;

use tokio::task;
use tracing::{debug, info};

pub mod nauthz_grpc {
    tonic::include_proto!("nauthz");
}

pub mod client;
pub mod config;
pub mod db;
pub mod error;
pub mod repo;
pub mod utils;

pub struct EventAuthz {
    pub repo: Arc<Mutex<Repo>>,
    pub settings: Settings,
    pub nostr_client: NostrClient,
}

#[tonic::async_trait]
impl Authorization for EventAuthz {
    async fn event_admit(
        &self,
        request: Request<EventRequest>,
    ) -> Result<Response<EventReply>, tonic::Status> {
        let req = request.into_inner();
        let event = req.clone().event.unwrap();
        let content_prefix: String = event.content.chars().take(40).collect();
        info!("recvd event, [kind={}, origin={:?}, nip05_domain={:?}, tag_count={}, content_sample={:?}]",
                 event.kind, req.origin, req.nip05.as_ref().map(|x| x.domain.clone()), event.tags.len(), content_prefix);

        let author = match req.auth_pubkey {
            Some(_) => req.auth_pubkey(),
            None => &event.pubkey,
        };

        let author = hex::encode(author);

        // I just picked this kind number should maybe put more thought into it, NIP?
        if event.kind == 4242 {
            // If author is trusted pubkey decode event and update account(s)
            // admit event
            if self.settings.info.admin_keys.contains(&author) {
                // TODO: Spawn this to not block
                self.repo
                    .lock()
                    .await
                    .handle_admission_update(event)
                    .await
                    .unwrap();

                // TODO: This is testing comment out
                // self.repo.lock().await.get_all_accounts().unwrap();
                return Ok(Response::new(nauthz_grpc::EventReply {
                    decision: Decision::Permit as i32,
                    message: Some("Ok".to_string()),
                }));
            }
        }

        // Check author OR event is admitted
        let reply = match event.is_admitted(&author, &self.repo.lock().await) {
            Ok(Status::Allow) => {
                // Spawn task to admit and fetch events
                // spawn a new thread that prints a message
                let repo = self.repo.clone();
                let nostr = self.nostr_client.clone();
                let relay = self.settings.info.relay.clone();
                task::spawn(async move {
                    let referenced = event.referenced_events().unwrap();
                    // debug!("{:?}", referenced);
                    repo.lock().await.admit_events(&referenced).await.unwrap();
                    let events = nostr.fetch_events(referenced).await.unwrap();
                    nostr.broadcast_events(&relay, events).await.unwrap();
                });

                nauthz_grpc::EventReply {
                    decision: Decision::Permit as i32,
                    message: Some("Ok".to_string()),
                }
            }
            _ => nauthz_grpc::EventReply {
                decision: Decision::Deny as i32,
                message: Some("Not allowed to publish".to_string()),
            },
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    tracing_subscriber::fmt::try_init().unwrap();

    let settings = config::Settings::new(&None);

    debug!("{:?}", settings);

    let repo = Arc::new(Mutex::new(Repo::new()));

    repo.lock()
        .await
        .admit_pubkeys(&settings.info.admin_keys)
        .await?;

    repo.lock().await.get_all_accounts()?;

    let nostr_client = NostrClient::new(&settings.info.default_relays).await?;

    let checker = EventAuthz {
        repo: repo.clone(),
        settings: settings.clone(),
        nostr_client,
    };

    // run this in a new thread
    if let Some(api_key) = settings.info.api_key {
        let _handle = task::spawn(start_server(api_key, repo));
    }

    info!("EventAuthz Server listening on {addr}");
    // Start serving
    Server::builder()
        .add_service(AuthorizationServer::new(checker))
        .serve(addr)
        .await?;

    Ok(())
}

impl Event {
    pub fn is_admitted(&self, author: &str, repo: &MutexGuard<Repo>) -> Result<Status, Error> {
        if let Some(event) = repo.get_event(&hex::encode(&self.id))? {
            if event.is_admitted() {
                return Ok(Status::Allow);
            }
        }

        if let Some(account) = repo.get_account(author)? {
            if account.is_admitted() {
                return Ok(Status::Allow);
            }
        }
        Ok(Status::Deny)
    }

    pub fn referenced_events(&self) -> Result<HashMap<EventId, Option<String>>, Error> {
        let event: nostr_sdk::Event = self.into();
        let event_ids: HashMap<EventId, Option<String>> = event
            .tags
            .iter()
            .filter_map(|tag| match tag {
                Tag::Event(values, relay, ..) => Some((*values, relay.clone())),
                _ => None,
            })
            .collect();

        Ok(event_ids)
    }
}

#[derive(Clone)]
struct AppState {
    api_key: String,
    repo: Arc<Mutex<Repo>>,
}

async fn start_server(api_key: String, repo: Arc<Mutex<Repo>>) -> Result<(), Error> {
    let shared_state = AppState {
        api_key: api_key.to_string(),
        repo,
    };

    // build our application with a single route
    let app = Router::new()
        .route("/update", post(update_users))
        .route("/users", get(get_users))
        .with_state(shared_state);

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Users {
    allow: Option<Vec<String>>,
    deny: Option<Vec<String>>,
}

async fn update_users(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(payload): Json<Users>,
) -> Result<(), (StatusCode, String)> {
    debug!("Users: {payload:?}");
    if let Some(key) = headers.get("X-Api-Key") {
        debug!("Sent key: {key:?}");
        if key.eq(&state.api_key) {
            // Admit pubkeys
            if let Some(pubkeys) = &payload.allow {
                debug!("Pubkeys to allow: {pubkeys:?}");
                state.repo.lock().await.admit_pubkeys(pubkeys).await.ok();
            }

            // Deny pubkeys
            if let Some(pubkeys) = &payload.deny {
                debug!("Pubkeys to deny: {pubkeys:?}");
                state.repo.lock().await.deny_pubkeys(pubkeys).await.ok();
            }
            return Ok(());
        }
    }

    Err((StatusCode::UNAUTHORIZED, "".to_string()))
}

async fn get_users(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<Json<Users>, (StatusCode, String)> {
    debug!("{}", state.api_key);
    if let Some(key) = headers.get("X-Api-Key") {
        if key.eq(&state.api_key) {
            let users = state.repo.lock().await.get_accounts().unwrap();
            return Ok(Json(users));
        }
        return Err((StatusCode::UNAUTHORIZED, "Invalid API Key".to_string()));
    }

    Err((StatusCode::UNAUTHORIZED, "No Api Key".to_string()))
}
