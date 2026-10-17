//! OKF (Open Knowledge Format) tools.
//!
//! Provides the "Knowledge API" for grok-cli:
//! - `okf_lookup` — search across loaded OKF bundles.
//!
//! These bundles are also loaded automatically at session start
//! (see MemoryStore + knowledge loading) to act as the "Knowledge OS".

use anyhow::{Result, anyhow};

use crate::config::Config;
use crate::knowledge::okf::{OkfBundle, OkfConcept, load_okf_bundles};
use chrono;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};
use std::time::Duration;

/// Global cache of loaded OKF bundles for the current process.
/// Uses RwLock so we can invalidate it after `okf_create`.
static OKF_BUNDLES: OnceLock<RwLock<Vec<OkfBundle>>> = OnceLock::new();

/// ETag cache for remote v1 manifest fetches: url -> (manifest body, etag).
/// Lets us send If-None-Match and honor 304 across lookups (task 480.3).
static REMOTE_ETAG_CACHE: OnceLock<RwLock<std::collections::HashMap<String, (String, String)>>> =
    OnceLock::new();

fn remote_etag_cache(
) -> &'static RwLock<std::collections::HashMap<String, (String, String)>> {
    REMOTE_ETAG_CACHE.get_or_init(|| RwLock::new(std::collections::HashMap::new()))
}

fn get_okf_cache() -> &'static RwLock<Vec<OkfBundle>> {
    OKF_BUNDLES.get_or_init(|| RwLock::new(Vec::new()))
}

/// Load OKF bundles according to the current config.
/// This is called both at session start and on first tool use.
///
/// Remote (v1, `[okf] remote_url`) is the shared source of truth and is
/// fetched first; local `knowledge_bundles` dirs always load too and serve
/// as the offline fallback when the remote is unreachable.
pub fn load_okf_from_config(config: &Config) -> Vec<OkfBundle> {
    if !config.okf.enabled {
        return vec![];
    }

    let mut bundles: Vec<OkfBundle> = Vec::new();

    // Remote first: GET {remote_url}/okf/manifest.json (+ knowledge docs).
    if config.okf.remote_url.is_some() {
        match load_remote_bundle_sync(&config.okf) {
            Some(remote) => bundles.push(remote),
            None => tracing::warn!(
                "Remote OKF fetch failed; falling back to local bundle dirs"
            ),
        }
    }

    let mut paths: Vec<PathBuf> = config
        .okf
        .knowledge_bundles
        .iter()
        .map(|s| {
            // Expand ~ and make absolute if relative
            let expanded = shellexpand::tilde(s).to_string();
            let p = PathBuf::from(expanded);
            if p.is_relative() {
                std::env::current_dir().unwrap_or_default().join(p)
            } else {
                p
            }
        })
        .collect();

    // Also support the legacy trace-forwarder style if someone puts a single dir
    // in server field as a hack (not recommended, but we stay flexible).
    let extra = &config.okf.server;
    if !extra.trim().is_empty() && (extra.contains('/') || extra.contains('\\')) {
        paths.push(PathBuf::from(shellexpand::tilde(extra).to_string()));
    }

    match load_okf_bundles(&paths) {
        Ok(mut local) => bundles.append(&mut local),
        Err(e) => {
            tracing::warn!("Failed to load OKF bundles: {}", e);
        }
    }

    bundles
}

// ── Remote v1 bundle fetch (task 480.3) ────────────────────────────────────
// Fetches the shared bundle from the OKF server (PROTOCOL.md) so grok-cli
// can read the same knowledge Helix serves. Local dirs are the fallback.

/// Minimal shape of the v1 manifest served by the OKF server.
#[derive(serde::Deserialize)]
struct RemoteOkfManifest {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    knowledge: Vec<RemoteKnowledgeEntry>,
}

#[derive(serde::Deserialize)]
struct RemoteKnowledgeEntry {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

/// Fetch the remote bundle in a dedicated thread: sync contexts can't
/// block_on a runtime they don't own (same pattern as get_okf_bundles).
/// Returns None on any failure — the caller falls back to local dirs.
fn load_remote_bundle_sync(cfg: &crate::config::OkfConfig) -> Option<OkfBundle> {
    let remote_url = cfg
        .remote_url
        .as_ref()
        .map(|u| u.trim().trim_end_matches('/').to_string())
        .filter(|u| !u.is_empty())?;
    let api_key = cfg.api_key.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(fetch_remote_bundle(&remote_url, api_key.as_deref()))
            .ok()
    })
    .join()
    .ok()
    .flatten()
}

/// GET {base}/okf/manifest.json with ETag, then per-entry knowledge docs.
async fn fetch_remote_bundle(base_url: &str, api_key: Option<&str>) -> Result<OkfBundle> {
    let client = crate::utils::http::http_client_builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let manifest_url = format!("{}/okf/manifest.json", base_url);
    let mut req = client.get(&manifest_url);
    if let Some((_, etag)) = remote_etag_cache().read().unwrap().get(&manifest_url) {
        req = req.header(reqwest::header::IF_NONE_MATCH, etag);
    }
    if let Some(key) = api_key
        && !key.trim().is_empty()
    {
        req = req.bearer_auth(key.trim());
    }
    let resp = req.send().await?;

    let body = match resp.status() {
        reqwest::StatusCode::NOT_MODIFIED => remote_etag_cache()
            .read()
            .unwrap()
            .get(&manifest_url)
            .map(|(b, _)| b.clone())
            .ok_or_else(|| anyhow!("304 with no cached manifest"))?,
        reqwest::StatusCode::OK => {
            let etag = resp
                .headers()
                .get(reqwest::header::ETAG)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string();
            let text = resp.text().await?;
            if !etag.is_empty() {
                remote_etag_cache()
                    .write()
                    .unwrap()
                    .insert(manifest_url.clone(), (text.clone(), etag));
            }
            text
        }
        s => {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("Remote OKF manifest returned {}: {}", s, text));
        }
    };

    let manifest: RemoteOkfManifest = serde_json::from_str(&body)?;
    let bundle_name = manifest
        .name
        .clone()
        .unwrap_or_else(|| manifest.id.clone());
    let mut concepts = Vec::with_capacity(manifest.knowledge.len());
    let mut by_id = std::collections::HashMap::new();

    for entry in manifest.knowledge {
        let content = match entry.content {
            Some(c) => c,
            None => fetch_remote_knowledge(&client, base_url, &entry.id, api_key).await?,
        };
        let title = entry.title.unwrap_or_else(|| entry.id.clone());
        by_id.insert(entry.id.clone(), concepts.len());
        concepts.push(OkfConcept {
            id: entry.id,
            r#type: String::new(),
            title,
            description: String::new(),
            resource: None,
            tags: Vec::new(),
            timestamp: None,
            body: content,
            source_path: PathBuf::from(format!("{}/okf/knowledge", base_url)),
            bundle_name: bundle_name.clone(),
        });
    }

    Ok(OkfBundle {
        name: bundle_name,
        root: PathBuf::from(base_url),
        concepts,
        by_id,
    })
}

/// GET {base}/okf/knowledge/{kid} — raw document text.
async fn fetch_remote_knowledge(
    client: &reqwest::Client,
    base_url: &str,
    kid: &str,
    api_key: Option<&str>,
) -> Result<String> {
    let mut req = client.get(format!("{}/okf/knowledge/{}", base_url, kid));
    if let Some(key) = api_key
        && !key.trim().is_empty()
    {
        req = req.bearer_auth(key.trim());
    }
    let resp = req.send().await?;
    if resp.status().is_success() {
        Ok(resp.text().await?)
    } else {
        Err(anyhow!(
            "Remote OKF knowledge '{}' returned {}",
            kid,
            resp.status()
        ))
    }
}

/// Get (or lazily load) the current OKF bundles.
pub fn get_okf_bundles(config: Option<&Config>) -> Vec<OkfBundle> {    let cache = get_okf_cache();
    {
        let guard = cache.read().unwrap();
        if !guard.is_empty() {
            return guard.clone();
        }
    }

    let bundles = if let Some(cfg) = config {
        load_okf_from_config(cfg)
    } else {
        match std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().ok()?;
            rt.block_on(crate::config::Config::load_hierarchical()).ok()
        })
        .join()
        .ok()
        .flatten()
        {
            Some(cfg) => load_okf_from_config(&cfg),
            None => vec![],
        }
    };

    let mut guard = cache.write().unwrap();
    *guard = bundles.clone();
    bundles
}

/// Force reload of OKF bundles (useful after config change).
pub fn reload_okf_bundles(config: &Config) -> Vec<OkfBundle> {
    let bundles = load_okf_from_config(config);
    let cache = get_okf_cache();
    let mut guard = cache.write().unwrap();
    *guard = bundles.clone();
    bundles
}

/// The main OKF lookup tool.
///
/// Searches across all loaded Open Knowledge Format bundles.
/// Returns the most relevant concepts with their content.
pub fn okf_lookup(query: &str, max_results: Option<usize>) -> Result<String> {
    let max = max_results.unwrap_or(5).min(20);

    // Try to get bundles. If not loaded yet, attempt a config load.
    let bundles = get_okf_bundles(None);

    if bundles.is_empty() {
        return Ok("No OKF knowledge bundles are currently loaded.\n\n\
             To use OKF knowledge:\n\
             1. Set `okf.enabled = true` in your config.\n\
             2. Add directories to `okf.knowledge_bundles`.\n\
             3. Put markdown files with YAML frontmatter in those directories.\n\n\
             Example concept:\n\
             ---\n\
             type: Metric\n\
             title: Weekly Active Users\n\
             ---\n\
             # Definition\n\
             ..."
        .to_string());
    }

    let mut all_results: Vec<(OkfConcept, f32)> = Vec::new();

    for bundle in &bundles {
        for concept in bundle.search(query) {
            // crude scoring boost by bundle if needed
            all_results.push((concept.clone(), 1.0));
        }
    }

    // Dedup by id and take top N
    all_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    all_results.truncate(max);

    if all_results.is_empty() {
        return Ok(format!("No OKF concepts matched query: \"{}\"", query));
    }

    let mut output = format!(
        "Found {} OKF concept(s) for query: \"{}\"\n\n",
        all_results.len(),
        query
    );

    for (i, (concept, _score)) in all_results.iter().enumerate() {
        output.push_str(&format!(
            "### {}. {}  (type: {})\n",
            i + 1,
            concept.title,
            if concept.r#type.is_empty() {
                "Concept"
            } else {
                &concept.r#type
            }
        ));

        if !concept.description.is_empty() {
            output.push_str(&format!("**Description**: {}\n", concept.description));
        }

        if let Some(res) = &concept.resource {
            output.push_str(&format!("**Resource**: {}\n", res));
        }

        if !concept.tags.is_empty() {
            output.push_str(&format!("**Tags**: {}\n", concept.tags.join(", ")));
        }

        output.push_str(&format!(
            "**Source**: {} (bundle: {})\n\n",
            concept.id, concept.bundle_name
        ));

        // Include a useful chunk of the body
        let body_preview = if concept.body.len() > 1200 {
            format!(
                "{}...\n\n(Use more specific query or `okf_get` for full content)",
                &concept.body[..1200]
            )
        } else {
            concept.body.clone()
        };

        output.push_str(&body_preview);
        output.push_str("\n\n---\n\n");
    }

    Ok(output)
}

/// Get full content of a specific OKF concept by its ID (path inside bundle).
pub fn okf_get(id: &str) -> Result<String> {
    let bundles = get_okf_bundles(None);

    for bundle in &bundles {
        if let Some(concept) = bundle.get_by_id(id) {
            return Ok(format!(
                "# {} ({})\n\n**Type**: {}\n**Bundle**: {}\n\n{}\n\n---\nSource: {}",
                concept.title,
                id,
                concept.r#type,
                concept.bundle_name,
                concept.body,
                concept.source_path.display()
            ));
        }
    }

    Err(anyhow!("OKF concept not found: {}", id))
}

/// Create a new OKF concept.
///
/// If a remote OKF server is configured (`okf.remote_url`), it will attempt
/// to push the concept to the server (to the `default_bundle`).
/// Otherwise it falls back to creating it locally in the first knowledge bundle.
///
/// Returns a success message with the new concept's ID.
pub async fn okf_create(
    r#type: &str,
    title: &str,
    body: &str,
    description: Option<&str>,
    tags: Option<Vec<String>>,
    resource: Option<&str>,
    explicit_id: Option<&str>,
) -> Result<String> {
    let cfg = crate::config::Config::load_hierarchical().await?;

    if !cfg.okf.enabled {
        return Err(anyhow!(
            "OKF is disabled. Set `okf.enabled = true` in config."
        ));
    }

    let id = explicit_id.map(|s| s.to_string()).unwrap_or_else(|| {
        // Generate a clean ID from title + type
        let slug = title
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
            .trim_matches('-')
            .to_string();
        let type_slug = r#type.to_lowercase().replace(' ', "-");
        format!("{}/{}", type_slug, slug)
    });

    let concept = OkfConcept {
        id: id.clone(),
        r#type: r#type.to_string(),
        title: title.to_string(),
        description: description.unwrap_or("").to_string(),
        resource: resource.map(|s| s.to_string()),
        tags: tags.unwrap_or_default(),
        timestamp: Some(chrono::Utc::now().to_rfc3339()),
        body: body.to_string(),
        source_path: PathBuf::new(),
        bundle_name: cfg.okf.default_bundle.clone(),
    };

    // Try remote first if configured
    if let Some(remote) = &cfg.okf.remote_url {
        match push_concept_to_remote(&concept, remote, &cfg.okf.default_bundle, &cfg.okf.api_key)
            .await
        {
            Ok(_) => {
                // Invalidate cache so future lookups see the new concept
                let cache = get_okf_cache();
                let mut guard = cache.write().unwrap();
                *guard = vec![];
                return Ok(format!(
                    "✅ Created concept '{}' on remote OKF server (bundle: {})\nID: {}",
                    title, cfg.okf.default_bundle, id
                ));
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to push to remote OKF server: {}. Falling back to local.",
                    e
                );
            }
        }
    }

    // Local fallback
    write_concept_locally(&concept, &cfg).await?;

    // Invalidate cache
    let cache = get_okf_cache();
    let mut guard = cache.write().unwrap();
    *guard = vec![];

    Ok(format!(
        "✅ Created local OKF concept '{}'\nID: {}\nBundle: {}",
        title, id, cfg.okf.default_bundle
    ))
}

async fn push_concept_to_remote(
    concept: &OkfConcept,
    base_url: &str,
    bundle: &str,
    api_key: &Option<String>,
) -> Result<()> {
    let client = crate::utils::http::http_client_builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let url = format!(
        "{}/okf/bundles/{}/concepts",
        base_url.trim_end_matches('/'),
        bundle
    );

    let mut req = client.post(&url).json(concept);

    if let Some(key) = api_key
        && !key.trim().is_empty()
    {
        req = req.bearer_auth(key.trim());
    }

    let resp = req.send().await?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("Remote OKF returned {}: {}", status, body))
    }
}

async fn write_concept_locally(concept: &OkfConcept, cfg: &Config) -> Result<()> {
    use std::fs;

    if cfg.okf.knowledge_bundles.is_empty() {
        return Err(anyhow!("No knowledge_bundles configured for local write."));
    }

    let base_dir = shellexpand::tilde(&cfg.okf.knowledge_bundles[0]).to_string();
    let base = PathBuf::from(base_dir);
    fs::create_dir_all(&base)?;

    // Build markdown with frontmatter
    let mut md = String::new();
    md.push_str("---\n");
    md.push_str(&format!("type: {}\n", concept.r#type));
    md.push_str(&format!("title: {}\n", concept.title));
    if !concept.description.is_empty() {
        md.push_str(&format!("description: {}\n", concept.description));
    }
    if let Some(res) = &concept.resource {
        md.push_str(&format!("resource: {}\n", res));
    }
    if !concept.tags.is_empty() {
        md.push_str(&format!("tags: {:?}\n", concept.tags));
    }
    if let Some(ts) = &concept.timestamp {
        md.push_str(&format!("timestamp: {}\n", ts));
    }
    md.push_str("---\n\n");
    md.push_str(&concept.body);

    let file_path = base.join(format!("{}.md", concept.id));
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&file_path, md)?;
    tracing::info!("Wrote local OKF concept to {:?}", file_path);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OkfConfig;

    fn test_concept() -> OkfConcept {
        OkfConcept {
            id: "concept-1".into(),
            r#type: "Note".into(),
            title: "Test Concept".into(),
            description: String::new(),
            resource: None,
            tags: vec![],
            timestamp: None,
            body: "concept body".into(),
            source_path: PathBuf::from("test.md"),
            bundle_name: "test".into(),
        }
    }

    // ── 480.2 contract: concept push targets the v1 path ─────────────────

    #[tokio::test]
    async fn concept_push_targets_v1_path() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/okf/bundles/test-b/concepts")
            .with_status(201)
            .create_async()
            .await;

        push_concept_to_remote(&test_concept(), &server.url(), "test-b", &None)
            .await
            .expect("concept push works");
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn concept_push_sends_bearer_auth() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/okf/bundles/test-b/concepts")
            .match_header("authorization", mockito::Matcher::Regex("Bearer secret-token".to_string()))
            .with_status(201)
            .create_async()
            .await;

        push_concept_to_remote(
            &test_concept(),
            &server.url(),
            "test-b",
            &Some("secret-token".to_string()),
        )
        .await
        .expect("concept push works");
        mock.assert_async().await;
    }

    // ── 480.3 contract: remote fetch with local fallback ─────────────────

    #[tokio::test]
    async fn remote_fetch_manifest_and_knowledge_docs() {
        let mut server = mockito::Server::new_async().await;
        let manifest = serde_json::json!({
            "id": "remote-bundle",
            "name": "Remote Bundle",
            "version": "1.0.0",
            "knowledge": [
                {"id": "k1", "title": "Inline Doc", "content": "inline body"},
                {"id": "k2", "title": "Doc Two"}
            ]
        });
        let _m1 = server
            .mock("GET", "/okf/manifest.json")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(manifest.to_string())
            .create_async()
            .await;
        let _m2 = server
            .mock("GET", "/okf/knowledge/k2")
            .with_status(200)
            .with_body("fetched body two")
            .create_async()
            .await;

        let bundle = fetch_remote_bundle(&server.url(), None)
            .await
            .expect("remote fetch works");
        assert_eq!(bundle.name, "Remote Bundle");
        assert_eq!(bundle.concepts.len(), 2);
        assert_eq!(bundle.concepts[0].body, "inline body");
        assert_eq!(bundle.concepts[1].body, "fetched body two");
        assert_eq!(bundle.get_by_id("k1").unwrap().title, "Inline Doc");
    }

    #[tokio::test]
    async fn remote_fetch_honors_etag_304() {
        let mut server = mockito::Server::new_async().await;
        let manifest = serde_json::json!({
            "id": "etag-bundle",
            "knowledge": [{"id": "k1", "content": "cached"}]
        });
        let _m1 = server
            .mock("GET", "/okf/manifest.json")
            .with_status(200)
            .with_header("ETag", "\"abc123\"")
            .with_body(manifest.to_string())
            .create_async()
            .await;

        let b1 = fetch_remote_bundle(&server.url(), None)
            .await
            .expect("first fetch");
        assert_eq!(b1.concepts[0].body, "cached");

        // Second round: server sees If-None-Match and answers 304.
        let _m2 = server
            .mock("GET", "/okf/manifest.json")
            .match_header("If-None-Match", "\"abc123\"")
            .with_status(304)
            .create_async()
            .await;

        let b2 = fetch_remote_bundle(&server.url(), None)
            .await
            .expect("304 fetch");
        assert_eq!(b2.concepts[0].body, "cached");
    }

    #[test]
    fn offline_remote_returns_none_for_fallback() {
        // Nothing listens on port 9 → fetch fails → caller falls back local.
        let mut cfg = OkfConfig::default();
        cfg.remote_url = Some("http://127.0.0.1:9".to_string());
        assert!(load_remote_bundle_sync(&cfg).is_none());
    }
}
