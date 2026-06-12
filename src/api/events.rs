use crate::{AppState, validate_session};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{
        Sse,
        sse::{Event, KeepAlive},
    },
};
use chrono::Utc;
use futures_util::{Stream, stream};
use serde::Deserialize;
use serde::Serialize;
use std::convert::Infallible;
use tokio::sync::broadcast;

const EVENT_BUFFER_CAPACITY: usize = 32;

#[derive(Clone, Serialize)]
pub struct FileEvent {
    kind: String,
    id: String,
    path: String,
    at: chrono::DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct EventsQuery {
    id: String,
    password: String,
}

pub async fn events(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    if !validate_session(&state, &query.id, &query.password).await {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let tx = notifier_for_user(&state, &query.id).await;
    let rx = tx.subscribe();

    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(file_event) => {
                let json = serde_json::to_string(&file_event).ok()?;
                let event = Event::default().event("file-updated").data(json);
                Some((Ok(event), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                let event = Event::default()
                    .event("warning")
                    .data(r#"{"kind":"lagged"}"#);
                Some((Ok(event), rx))
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
        }
    });

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub async fn notify_file_updated(state: &AppState, id: &str) {
    let tx = notifier_for_user(state, id).await;

    let _ = tx.send(FileEvent {
        kind: "file-updated".to_string(),
        id: id.to_string(),
        path: crate::DB_FILE_NAME.to_string(),
        at: Utc::now(),
    });
}

async fn notifier_for_user(state: &AppState, id: &str) -> broadcast::Sender<FileEvent> {
    let mut notifiers = state.notifiers.write().await;

    notifiers
        .entry(id.to_string())
        .or_insert_with(|| {
            let (tx, _rx) = broadcast::channel(EVENT_BUFFER_CAPACITY);
            tx
        })
        .clone()
}
