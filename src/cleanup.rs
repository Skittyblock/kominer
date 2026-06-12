use crate::AppState;
use chrono::Utc;
use tokio::time::{Duration, sleep};

// checks for stored sessions past expiration date and clears the files and removes the in-memory data
pub async fn clean_sessions(state: AppState) {
    loop {
        let now = Utc::now();

        let expired_ids: Vec<String> = {
            let sessions = state.sessions.read().await;
            sessions
                .iter()
                .filter_map(|(id, s)| (s.expires_at <= now).then_some(id.clone()))
                .collect()
        };

        if !expired_ids.is_empty() {
            let mut sessions = state.sessions.write().await;
            for id in &expired_ids {
                sessions.remove(id);
            }
            let mut notifiers = state.notifiers.write().await;
            for id in &expired_ids {
                notifiers.remove(id);
            }

            for id in expired_ids {
                let dir = state.storage_root.join(&id);
                if let Err(err) = tokio::fs::remove_dir_all(&dir).await {
                    eprintln!("cleanup failed for {id}: {err}");
                }
            }
        }

        sleep(Duration::from_mins(10)).await;
    }
}
