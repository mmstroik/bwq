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
    id: String,
    labels: Option<HashMap<String, WikiDataLabel>>,
    descriptions: Option<HashMap<String, WikiDataDescription>>,
    sitelinks: Option<HashMap<String, WikiDataSitelink>>,
}

#[derive(Debug, Deserialize)]
struct WikiDataLabel {
    language: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct WikiDataDescription {
    language: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct WikiDataSitelink {
    site: String,
    title: String,
    url: Option<String>,
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
        if let Some(cached) = self.cache.get(entity_id) {
            if cached.cached_at.elapsed() < self.cache_ttl {
                tracing::debug!("Returning cached entity info for: {}", entity_id);
                return Ok(Some(cached.entity.clone()));
            }
        }

        // Clean the entity ID - remove Q prefix if present, and validate it's numeric
        let clean_id = if let Some(stripped) = entity_id.strip_prefix('Q') {
            stripped
        } else {
            entity_id
        };

        if !clean_id.chars().all(|c| c.is_ascii_digit()) {
            tracing::warn!("Invalid entity ID format: {}", entity_id);
            return Ok(None);
        }

        let wikidata_id = format!("Q{clean_id}");

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

    /// Extract entity ID from entityId field pattern (e.g., "entityId:123" -> "123")
    pub fn extract_entity_id_from_text(text: &str, position: usize) -> Option<String> {
        // Look for entityId:NUMBER pattern around the cursor position
        let start = position.saturating_sub(20);
        let end = (position + 20).min(text.len());
        let search_text = &text[start..end];

        for (i, _) in search_text.match_indices("entityId:") {
            let field_start = start + i;
            let value_start = field_start + 9;

            // Find the end of the entity ID (until whitespace or special chars)
            let remaining = &text[value_start..];
            let entity_id_end = remaining
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(remaining.len());

            let field_end = value_start + entity_id_end;

            // Check if cursor is within this entityId field (including the field name and value)
            if position >= field_start && position <= field_end {
                let entity_id = &text[value_start..field_end];
                if !entity_id.is_empty() && entity_id.chars().all(|c| c.is_ascii_digit()) {
                    return Some(entity_id.to_string());
                }
            }
        }

        None
    }

    pub fn cleanup_cache(&mut self) {
        let now = Instant::now();
        self.cache
            .retain(|_, cached| now.duration_since(cached.cached_at) < self.cache_ttl);
    }
}

impl Default for WikiDataClient {
    fn default() -> Self {
        Self::new().expect("Failed to create WikiData client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_entity_id_from_text() {
        let text = "This is a query with entityId:29 in the middle";
        let position = 32; // somewhere in the "29" (position of "9")

        let result = WikiDataClient::extract_entity_id_from_text(text, position);
        assert_eq!(result, Some("29".to_string()));
    }

    #[test]
    fn test_extract_entity_id_at_start() {
        let text = "entityId:123 AND something";
        let position = 5; // in the field name

        let result = WikiDataClient::extract_entity_id_from_text(text, position);
        assert_eq!(result, Some("123".to_string()));
    }

    #[test]
    fn test_extract_entity_id_no_match() {
        let text = "This has no entity field";
        let position = 10;

        let result = WikiDataClient::extract_entity_id_from_text(text, position);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_entity_id_invalid_format() {
        let text = "entityId:abc123";
        let position = 10;

        let result = WikiDataClient::extract_entity_id_from_text(text, position);
        assert_eq!(result, None);
    }
}
