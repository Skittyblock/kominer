use crate::AppState;

pub async fn startup(state: &AppState) {
    tokio::fs::create_dir_all(&state.storage_root)
        .await
        .unwrap();

    #[cfg(feature = "public_mode")]
    {
        // clear files, excluding reserved dir
        let keep_name = state.auth.as_ref().map(|a| a.user.as_str());

        let mut rd = tokio::fs::read_dir(&state.storage_root).await.unwrap();
        while let Some(entry) = rd.next_entry().await.unwrap() {
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if keep_name.is_some_and(|k| k == name) {
                continue;
            }

            let path = entry.path();
            let file_type = entry.file_type().await.unwrap();

            if file_type.is_dir() {
                tokio::fs::remove_dir_all(path).await.ok();
            } else {
                tokio::fs::remove_file(path).await.ok();
            }
        }

        // ensure reserved dir exists
        if let Some(ref auth) = state.auth {
            let user_dir = state.storage_root.join(&auth.user);
            tokio::fs::create_dir_all(&user_dir).await.unwrap();
        }
    }
}

// checks for stored sessions past expiration date and clears the files and removes the in-memory data
#[cfg(feature = "public_mode")]
pub async fn clean_sessions(state: AppState) {
    use chrono::Utc;
    use tokio::time::{Duration, sleep};

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
