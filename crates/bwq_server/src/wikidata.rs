use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// WikiData client for entity search and information retrieval
pub struct WikiDataClient {
    client: Client,
    cache: HashMap<String, CachedEntity>,
    cache_ttl: Duration,
}

#[derive(Debug, Clone)]
struct CachedEntity {
    entity: EntityInfo,
    cached_at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySearchResult {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub concepturi: String,
}

#[derive(Debug, Deserialize)]
struct WikiDataSearchResponse {
    search: Vec<WikiDataSearchItem>,
}

#[derive(Debug, Deserialize)]
struct WikiDataSearchItem {
    id: String,
    label: String,
    description: Option<String>,
    concepturi: String,
}

#[derive(Debug, Deserialize)]
struct WikiDataEntityResponse {
    entities: HashMap<String, WikiDataEntity>,
}

#[derive(Debug, Deserialize)]
struct WikiDataEntity {
    labels: Option<HashMap<String, WikiDataLabel>>,
    descriptions: Option<HashMap<String, WikiDataDescription>>,
}

#[derive(Debug, Deserialize)]
struct WikiDataLabel {
    value: String,
}

#[derive(Debug, Deserialize)]
struct WikiDataDescription {
    value: String,
}

impl WikiDataClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("bwq-language-server/0.4.3")
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(300), // 5 minutes
        })
    }

    pub async fn search_entities(&self, query: &str) -> Result<Vec<EntitySearchResult>> {
        if query.trim().is_empty() {
            return Ok(vec![]);
        }

        let url = format!(
            "https://www.wikidata.org/w/api.php?action=wbsearchentities&search={}&format=json&language=en&limit=10&origin=*",
            urlencoding::encode(query)
        );

        tracing::debug!("Searching WikiData for: {}", query);

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<WikiDataSearchResponse>()
            .await?;

        let results: Vec<EntitySearchResult> = response
            .search
            .into_iter()
            .map(|item| EntitySearchResult {
                id: item.id,
                label: item.label,
                description: item.description,
                concepturi: item.concepturi,
            })
            .collect();

        tracing::debug!(
            "Found {} WikiData entities for query: {}",
            results.len(),
            query
        );
        Ok(results)
    }

    /// Get detailed information about a WikiData entity by ID (with caching)
    pub async fn get_entity_info(&mut self, entity_id: &str) -> Result<Option<EntityInfo>> {
        // clean up expired cache entries
        if self.cache.len() > 10 {
            self.cleanup_cache();
        }

        if let Some(cached) = self.cache.get(entity_id) {
            if cached.cached_at.elapsed() < self.cache_ttl {
                tracing::debug!("Returning cached entity info for: {}", entity_id);
                return Ok(Some(cached.entity.clone()));
            }
        }

        if !entity_id.chars().all(|c| c.is_ascii_digit()) {
            tracing::warn!("Invalid entity ID format: {}", entity_id);
            return Ok(None);
        }

        let wikidata_id = format!("Q{entity_id}");

        let url = format!(
            "https://www.wikidata.org/w/api.php?action=wbgetentities&ids={wikidata_id}&format=json&languages=en&origin=*"
        );

        tracing::debug!("Fetching WikiData entity info for: {}", wikidata_id);

        let response = self
            .client
            .get(&url)
            .send()
            .await?
            .json::<WikiDataEntityResponse>()
            .await?;

        if let Some(entity) = response.entities.get(&wikidata_id) {
            let label = entity
                .labels
                .as_ref()
                .and_then(|labels| labels.get("en"))
                .map(|label| label.value.clone())
                .unwrap_or_else(|| wikidata_id.clone());

            let description = entity
                .descriptions
                .as_ref()
                .and_then(|descriptions| descriptions.get("en"))
                .map(|desc| desc.value.clone());

            let entity_info = EntityInfo {
                id: wikidata_id.clone(),
                label,
                description,
                url: format!("https://www.wikidata.org/wiki/{wikidata_id}"),
            };

            self.cache.insert(
                entity_id.to_string(),
                CachedEntity {
                    entity: entity_info.clone(),
                    cached_at: Instant::now(),
                },
            );

            tracing::debug!("Successfully fetched entity info for: {}", wikidata_id);
            Ok(Some(entity_info))
        } else {
            tracing::warn!("No entity found for ID: {}", wikidata_id);
            Ok(None)
        }
    }

    pub fn cleanup_cache(&mut self) {
        let cache_size_before = self.cache.len();
        let now = Instant::now();
        self.cache
            .retain(|_, cached| now.duration_since(cached.cached_at) < self.cache_ttl);
        let cache_size_after = self.cache.len();

        if cache_size_before > cache_size_after {
            tracing::debug!(
                "WIKIDATA CACHE: Cleaned {} expired entries (cache size: {} -> {})",
                cache_size_before - cache_size_after,
                cache_size_before,
                cache_size_after
            );
        }
    }
}

impl Default for WikiDataClient {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Fallback to a minimal client that will return errors for all operations
            WikiDataClient {
                client: Client::new(),
                cache: HashMap::new(),
                cache_ttl: Duration::from_secs(300),
            }
        })
    }
}
