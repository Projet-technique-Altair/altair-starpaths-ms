use reqwest::{Client, RequestBuilder};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use tracing::warn;

const METADATA_IDENTITY_URL: &str =
    "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/identity";
const TOKEN_CACHE_TTL: Duration = Duration::from_secs(55 * 60);

struct CachedToken {
    token: String,
    expires_at: Instant,
}

static TOKEN_CACHE: OnceLock<Mutex<HashMap<String, CachedToken>>> = OnceLock::new();
static METADATA_CLIENT: OnceLock<Client> = OnceLock::new();

fn token_cache() -> &'static Mutex<HashMap<String, CachedToken>> {
    TOKEN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn metadata_client() -> &'static Client {
    METADATA_CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}

fn cloud_run_audience(target_url: &str) -> Option<String> {
    let parsed = reqwest::Url::parse(target_url).ok()?;
    let host = parsed.host_str()?;
    if !host.ends_with(".run.app") {
        return None;
    }
    let scheme = match parsed.scheme() {
        "https" | "wss" => "https",
        "http" | "ws" => "http",
        _ => return None,
    };
    let mut audience = parsed.clone();
    audience.set_scheme(scheme).ok()?;
    audience.set_path("");
    audience.set_query(None);
    audience.set_fragment(None);
    Some(audience.to_string().trim_end_matches('/').to_string())
}

async fn fetch_token(audience: &str) -> Result<String, String> {
    let now = Instant::now();
    {
        let cache = token_cache().lock().unwrap();
        if let Some(cached) = cache.get(audience) {
            if cached.expires_at > now {
                return Ok(cached.token.clone());
            }
        }
    }

    let response = metadata_client()
        .get(METADATA_IDENTITY_URL)
        .header("Metadata-Flavor", "Google")
        .query(&[("audience", audience)])
        .send()
        .await
        .map_err(|e| format!("metadata request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("metadata server returned {}", response.status()));
    }

    let token = response
        .text()
        .await
        .map_err(|e| format!("failed to read token: {e}"))?;
    let token = token.trim().to_string();

    if token.is_empty() {
        return Err("empty token from metadata server".to_string());
    }

    token_cache().lock().unwrap().insert(
        audience.to_string(),
        CachedToken {
            token: token.clone(),
            expires_at: now + TOKEN_CACHE_TTL,
        },
    );

    Ok(token)
}

pub async fn with_cloud_run_auth(builder: RequestBuilder, target_url: &str) -> RequestBuilder {
    let Some(audience) = cloud_run_audience(target_url) else {
        return builder;
    };
    match fetch_token(&audience).await {
        Ok(token) => builder.header("Authorization", format!("Bearer {token}")),
        Err(e) => {
            warn!(target_url, error = %e, "failed to attach Cloud Run identity token");
            builder
        }
    }
}
