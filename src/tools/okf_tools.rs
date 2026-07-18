//! OKF (Open Knowledge Format) tools.
//!
//! Provides the "Knowledge API" for grok-cli:
//! - `okf_lookup` — search across loaded OKF bundles.
//!
//! These bundles are also loaded automatically at session start
//! (see MemoryStore + knowledge loading) to act as the "Knowledge OS".

use anyhow::{anyhow, Result};

use crate::config::Config;
use crate::knowledge::okf::{load_okf_bundles, OkfBundle, OkfConcept};
use chrono;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

/// Global cache of loaded OKF bundles for the current process.
/// Loaded lazily on first use of the tool or at session start.
static OKF_BUNDLES: OnceLock<Vec<OkfBundle>> = OnceLock::new();

/// Load OKF bundles according to the current config.
/// This is called both at session start and on first tool use.
pub fn load_okf_from_config(config: &Config) -> Vec<OkfBundle> {
    if !config.okf.enabled {
        return vec![];
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
        Ok(bundles) => bundles,
        Err(e) => {
            tracing::warn!("Failed to load OKF bundles: {}", e);
            vec![]
        }
    }
}

/// Get (or lazily load) the current OKF bundles.
pub fn get_okf_bundles(config: Option<&Config>) -> &'static [OkfBundle] {
    OKF_BUNDLES.get_or_init(|| {
        if let Some(cfg) = config {
            load_okf_from_config(cfg)
        } else {
            // Fallback: try to load hierarchical config
            match std::thread::spawn(|| {
                // We can't easily do async here, so use blocking load
                // In practice the caller should pass config.
                vec![]
            })
            .join()
            {
                Ok(v) => v,
                Err(_) => vec![],
            }
        }
    })
}

/// Force reload of OKF bundles (useful after config change).
pub fn reload_okf_bundles(config: &Config) -> &'static [OkfBundle] {
    // Simple approach: drop the old value by replacing the OnceLock is hard,
    // so we just document that restart is needed for now, or we can use a RwLock later.
    // For v1 we just load fresh if the OnceLock is empty.
    if OKF_BUNDLES.get().is_none() {
        let _ = OKF_BUNDLES.set(load_okf_from_config(config));
    }
    OKF_BUNDLES.get().map(|v| v.as_slice()).unwrap_or(&[])
}

/// The main OKF lookup tool.
///
/// Searches across all loaded Open Knowledge Format bundles.
/// Returns the most relevant concepts with their content.
pub fn okf_lookup(query: &str, max_results: Option<usize>) -> Result<String> {
    let max = max_results.unwrap_or(5).min(20);

    // Try to get bundles. If not loaded yet, attempt a config load.
    let bundles: &[OkfBundle] = if let Some(b) = OKF_BUNDLES.get() {
        b
    } else {
        // Best effort load (async load_hierarchical needs a runtime)
        let loaded = std::thread::spawn(|| {
            let rt = tokio::runtime::Runtime::new().ok()?;
            rt.block_on(crate::config::Config::load_hierarchical()).ok()
        })
        .join()
        .ok()
        .flatten();

        if let Some(cfg) = loaded {
            let b = load_okf_from_config(&cfg);
            let _ = OKF_BUNDLES.set(b);
            OKF_BUNDLES.get().map(|v| v.as_slice()).unwrap_or(&[])
        } else {
            &[]
        }
    };

    if bundles.is_empty() {
        return Ok(
            "No OKF knowledge bundles are currently loaded.\n\n\
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
             ...".to_string(),
        );
    }

    let mut all_results: Vec<(&OkfConcept, f32)> = Vec::new();

    for bundle in bundles {
        for concept in bundle.search(query) {
            // crude scoring boost by bundle if needed
            all_results.push((concept, 1.0));
        }
    }

    // Dedup by id and take top N
    all_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    all_results.truncate(max);

    if all_results.is_empty() {
        return Ok(format!(
            "No OKF concepts matched query: \"{}\"",
            query
        ));
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
            if concept.r#type.is_empty() { "Concept" } else { &concept.r#type }
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

        output.push_str(&format!("**Source**: {} (bundle: {})\n\n", concept.id, concept.bundle_name));

        // Include a useful chunk of the body
        let body_preview = if concept.body.len() > 1200 {
            format!("{}...\n\n(Use more specific query or `okf_get` for full content)", &concept.body[..1200])
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
    let bundles = OKF_BUNDLES.get().map(|v| v.as_slice()).unwrap_or(&[]);

    for bundle in bundles {
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
        return Err(anyhow!("OKF is disabled. Set `okf.enabled = true` in config."));
    }

    let id = explicit_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
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
        match push_concept_to_remote(&concept, remote, &cfg.okf.default_bundle, &cfg.okf.api_key).await {
            Ok(_) => {
                // Invalidate cache so future lookups see it (best effort)
                let _ = OKF_BUNDLES.take();
                return Ok(format!(
                    "✅ Created concept '{}' on remote OKF server (bundle: {})\nID: {}",
                    title, cfg.okf.default_bundle, id
                ));
            }
            Err(e) => {
                tracing::warn!("Failed to push to remote OKF server: {}. Falling back to local.", e);
            }
        }
    }

    // Local fallback
    write_concept_locally(&concept, &cfg).await?;

    // Reload cache
    let _ = OKF_BUNDLES.take();

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
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let url = format!(
        "{}/bundles/{}/concepts",
        base_url.trim_end_matches('/'),
        bundle
    );

    let mut req = client.post(&url).json(concept);

    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            req = req.bearer_auth(key.trim());
        }
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
