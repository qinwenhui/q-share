use std::time::Duration;

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;

use crate::error::Result;
use crate::fs::reader::{read_dir_listing, SortKey, SortOrder};
use crate::state::AppState;

const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 1000;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// URL-encoded path starting with "/". Empty / "/" means root.
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub order: Option<String>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<crate::fs::reader::DirListing>> {
    let url_path = q.path;
    let abs = state.sandbox.resolve(&url_path)?;
    let ttl = Duration::from_secs(state.config.cache_ttl_secs.max(1));
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let offset = q.offset.unwrap_or(0);
    let sort = SortKey::parse(q.sort.as_deref().unwrap_or("name"));
    let order = SortOrder::parse(q.order.as_deref().unwrap_or("asc"));

    let cache_key = abs.clone();
    if let Some(snap) = state.dir_cache.get(&cache_key, ttl) {
        // paginate from cached listing
        let total = snap.listing.entries.len();
        let end = (offset + limit).min(total);
        let entries = if offset < total {
            snap.listing.entries[offset..end].to_vec()
        } else {
            Vec::new()
        };
        let mut listing = snap.listing;
        listing.entries = entries;
        listing.total = total;
        return Ok(Json(listing));
    }

    let listing = read_dir_listing(
        &abs,
        &url_path,
        sort,
        order,
        0,
        MAX_LIMIT, // over-fetch for cache hit rate
        state.config.show_hidden,
    )
    .await?;

    // Return only the requested page; keep full listing in cache.
    let total = listing.entries.len();
    let end = (offset + limit).min(total);
    let page_entries = if offset < total {
        listing.entries[offset..end].to_vec()
    } else {
        Vec::new()
    };

    let cache_listing = crate::fs::reader::DirListing {
        path: listing.path.clone(),
        parent: listing.parent.clone(),
        entries: listing.entries.clone(),
        total,
    };
    state.dir_cache.put(cache_key, cache_listing);

    let mut page = listing;
    page.entries = page_entries;
    page.total = total;
    Ok(Json(page))
}
