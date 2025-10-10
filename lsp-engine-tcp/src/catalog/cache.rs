use querent_core::catalog::{CatalogRead, schema};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A cache entry with TTL (time-to-live) support.
#[derive(Clone)]
struct CacheEntry<T> {
    value: T,
    inserted_at: Instant,
}

impl<T> CacheEntry<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            inserted_at: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Option<Duration>) -> bool {
        if let Some(ttl) = ttl {
            self.inserted_at.elapsed() > ttl
        } else {
            false
        }
    }
}

/// A caching wrapper around any CatalogRead implementation using LRU cache with TTL support.
///
/// This struct provides transparent caching of catalog queries to reduce
/// database roundtrips and improve performance for repeated queries.
/// Each cache entry can have a time-to-live (TTL) after which it expires.
pub struct CachedCatalog<T> {
    inner: Arc<T>,
    schemas_cache: Arc<Mutex<Option<CacheEntry<Vec<String>>>>>,
    tables_cache: Arc<Mutex<lru::LruCache<String, CacheEntry<Vec<String>>>>>,
    columns_cache: Arc<Mutex<lru::LruCache<(String, String), CacheEntry<Vec<schema::Column>>>>>,
    table_cache: Arc<Mutex<lru::LruCache<(String, String), CacheEntry<Option<schema::Table>>>>>,
    functions_cache: Arc<Mutex<lru::LruCache<String, CacheEntry<Vec<schema::Function>>>>>,
    ttl: Arc<Mutex<Option<Duration>>>,
}

impl<T> CachedCatalog<T>
where
    T: CatalogRead,
{
    /// Create a new CachedCatalog with default cache sizes (100 entries per cache) and no TTL.
    pub fn new(inner: T) -> Self {
        Self::with_capacity(inner, 100)
    }

    /// Create a new CachedCatalog with a specified cache capacity and no TTL.
    pub fn with_capacity(inner: T, capacity: usize) -> Self {
        Self::with_capacity_and_ttl(inner, capacity, None)
    }

    /// Create a new CachedCatalog with a specified cache capacity and TTL.
    ///
    /// # Arguments
    /// * `inner` - The underlying catalog implementation
    /// * `capacity` - Maximum number of entries per cache
    /// * `ttl` - Time-to-live for cache entries (None for infinite)
    pub fn with_capacity_and_ttl(inner: T, capacity: usize, ttl: Option<Duration>) -> Self {
        Self {
            inner: Arc::new(inner),
            schemas_cache: Arc::new(Mutex::new(None)),
            tables_cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap(),
            ))),
            columns_cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap(),
            ))),
            table_cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap(),
            ))),
            functions_cache: Arc::new(Mutex::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap(),
            ))),
            ttl: Arc::new(Mutex::new(ttl)),
        }
    }

    /// Set the TTL (time-to-live) for all cache entries.
    ///
    /// This does not affect existing entries; they will use the TTL that was active
    /// when they were inserted. New entries will use the updated TTL.
    pub fn set_ttl(&self, ttl: Option<Duration>) {
        if let Ok(mut t) = self.ttl.lock() {
            *t = ttl;
        }
    }

    /// Get the current TTL setting.
    pub fn get_ttl(&self) -> Option<Duration> {
        self.ttl.lock().ok().and_then(|t| *t)
    }

    /// Clear all caches.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.schemas_cache.lock() {
            *cache = None;
        }
        if let Ok(mut cache) = self.tables_cache.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.columns_cache.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.table_cache.lock() {
            cache.clear();
        }
        if let Ok(mut cache) = self.functions_cache.lock() {
            cache.clear();
        }
    }

    /// Invalidate the schemas cache.
    pub fn invalidate_schemas(&self) {
        if let Ok(mut cache) = self.schemas_cache.lock() {
            *cache = None;
        }
    }

    /// Invalidate the tables cache for a specific schema.
    pub fn invalidate_tables(&self, schema: Option<&str>) {
        let key = schema.unwrap_or("public").to_string();
        if let Ok(mut cache) = self.tables_cache.lock() {
            cache.pop(&key);
        }
    }

    /// Invalidate the cache for a specific table.
    pub fn invalidate_table(&self, table: &str, schema: Option<&str>) {
        let schema_str = schema.unwrap_or("public").to_string();
        let key = (schema_str.clone(), table.to_string());

        if let Ok(mut cache) = self.columns_cache.lock() {
            cache.pop(&key);
        }
        if let Ok(mut cache) = self.table_cache.lock() {
            cache.pop(&key);
        }
    }

    /// Invalidate the functions cache for a specific schema.
    pub fn invalidate_functions(&self, schema: Option<&str>) {
        let key = schema.unwrap_or("public").to_string();
        if let Ok(mut cache) = self.functions_cache.lock() {
            cache.pop(&key);
        }
    }
}

impl<T> CatalogRead for CachedCatalog<T>
where
    T: CatalogRead + Send + Sync + 'static,
{
    fn list_schemas(&self) -> BoxedFuture<'_, Vec<String>> {
        let schemas_cache = self.schemas_cache.clone();
        let inner = self.inner.clone();
        let ttl = self.ttl.clone();

        Box::pin(async move {
            let current_ttl = ttl.lock().ok().and_then(|t| *t);

            // Check cache first
            if let Ok(cache) = schemas_cache.lock() {
                if let Some(ref entry) = *cache {
                    let is_expired = entry.is_expired(current_ttl);

                    // Return cached value immediately
                    let cached_value = entry.value.clone();

                    // If expired, spawn background refresh
                    if is_expired {
                        drop(cache); // Release lock before spawning
                        let inner_clone = inner.clone();
                        let cache_clone = schemas_cache.clone();
                        tokio::spawn(async move {
                            let fresh = inner_clone.list_schemas().await;
                            if let Ok(mut cache) = cache_clone.lock() {
                                *cache = Some(CacheEntry::new(fresh));
                            }
                        });
                    }

                    return cached_value;
                }
            }

            // Cache miss - fetch from underlying catalog (blocking)
            let schemas = inner.list_schemas().await;

            // Update cache
            if let Ok(mut cache) = schemas_cache.lock() {
                *cache = Some(CacheEntry::new(schemas.clone()));
            }

            schemas
        })
    }

    fn list_tables(&self, schema: &str) -> BoxedFuture<'_, Vec<String>> {
        let key = schema.to_string();
        let tables_cache = self.tables_cache.clone();
        let inner = self.inner.clone();
        let ttl = self.ttl.clone();

        Box::pin(async move {
            let current_ttl = ttl.lock().ok().and_then(|t| *t);

            // Check cache first
            if let Ok(mut cache) = tables_cache.lock() {
                if let Some(entry) = cache.get(&key) {
                    let is_expired = entry.is_expired(current_ttl);

                    // Return cached value immediately
                    let cached_value = entry.value.clone();

                    // If expired, spawn background refresh
                    if is_expired {
                        let key_clone = key.clone();
                        drop(cache); // Release lock before spawning
                        let inner_clone = inner.clone();
                        let cache_clone = tables_cache.clone();
                        tokio::spawn(async move {
                            let fresh = inner_clone.list_tables(&key_clone).await;
                            if let Ok(mut cache) = cache_clone.lock() {
                                cache.put(key_clone, CacheEntry::new(fresh));
                            }
                        });
                    }

                    return cached_value;
                }
            }

            // Cache miss - fetch from underlying catalog (blocking)
            let tables = inner.list_tables(&key).await;

            // Update cache
            if let Ok(mut cache) = tables_cache.lock() {
                cache.put(key, CacheEntry::new(tables.clone()));
            }

            tables
        })
    }

    fn list_columns(
        &self,
        table: &str,
        schema: &str,
    ) -> BoxedFuture<'_, Vec<schema::Column>> {
        let schema_str = if schema.is_empty() { "public" } else { schema }.to_string();
        let table_str = table.to_string();
        let key = (schema_str.clone(), table_str.clone());
        let columns_cache = self.columns_cache.clone();
        let inner = self.inner.clone();
        let ttl = self.ttl.clone();

        Box::pin(async move {
            let current_ttl = ttl.lock().ok().and_then(|t| *t);

            // Check cache first
            if let Ok(mut cache) = columns_cache.lock() {
                if let Some(entry) = cache.get(&key) {
                    let is_expired = entry.is_expired(current_ttl);

                    // Return cached value immediately
                    let cached_value = entry.value.clone();

                    // If expired, spawn background refresh
                    if is_expired {
                        let key_clone = key.clone();
                        let table_clone = table_str.clone();
                        let schema_clone = schema_str.clone();
                        drop(cache); // Release lock before spawning
                        let inner_clone = inner.clone();
                        let cache_clone = columns_cache.clone();
                        tokio::spawn(async move {
                            let fresh = inner_clone
                                .list_columns(&table_clone, &schema_clone)
                                .await;
                            if let Ok(mut cache) = cache_clone.lock() {
                                cache.put(key_clone, CacheEntry::new(fresh));
                            }
                        });
                    }

                    return cached_value;
                }
            }

            // Cache miss - fetch from underlying catalog (blocking)
            let columns = inner.list_columns(&table_str, &schema_str).await;

            // Update cache
            if let Ok(mut cache) = columns_cache.lock() {
                cache.put(key, CacheEntry::new(columns.clone()));
            }

            columns
        })
    }

    fn get_table(
        &self,
        table: &str,
        schema: &str,
    ) -> BoxedFuture<'_, Option<schema::Table>> {
        let schema_str = if schema.is_empty() { "public" } else { schema }.to_string();
        let table_str = table.to_string();
        let key = (schema_str.clone(), table_str.clone());
        let table_cache = self.table_cache.clone();
        let inner = self.inner.clone();
        let ttl = self.ttl.clone();

        Box::pin(async move {
            let current_ttl = ttl.lock().ok().and_then(|t| *t);

            // Check cache first
            if let Ok(mut cache) = table_cache.lock() {
                if let Some(entry) = cache.get(&key) {
                    let is_expired = entry.is_expired(current_ttl);

                    // Return cached value immediately
                    let cached_value = entry.value.clone();

                    // If expired, spawn background refresh
                    if is_expired {
                        let key_clone = key.clone();
                        let table_clone = table_str.clone();
                        let schema_clone = schema_str.clone();
                        drop(cache); // Release lock before spawning
                        let inner_clone = inner.clone();
                        let cache_clone = table_cache.clone();
                        tokio::spawn(async move {
                            let fresh = inner_clone
                                .get_table(&table_clone, &schema_clone)
                                .await;
                            if let Ok(mut cache) = cache_clone.lock() {
                                cache.put(key_clone, CacheEntry::new(fresh));
                            }
                        });
                    }

                    return cached_value;
                }
            }

            // Cache miss - fetch from underlying catalog (blocking)
            let table_info = inner.get_table(&table_str, &schema_str).await;

            // Update cache
            if let Ok(mut cache) = table_cache.lock() {
                cache.put(key, CacheEntry::new(table_info.clone()));
            }

            table_info
        })
    }

    fn list_functions(&self) -> BoxedFuture<'_, Vec<schema::Function>> {
        let key = "all".to_string();
        let functions_cache = self.functions_cache.clone();
        let inner = self.inner.clone();
        let ttl = self.ttl.clone();

        Box::pin(async move {
            let current_ttl = ttl.lock().ok().and_then(|t| *t);

            // Check cache first
            if let Ok(mut cache) = functions_cache.lock() {
                if let Some(entry) = cache.get(&key) {
                    let is_expired = entry.is_expired(current_ttl);

                    // Return cached value immediately
                    let cached_value = entry.value.clone();

                    // If expired, spawn background refresh
                    if is_expired {
                        let key_clone = key.clone();
                        drop(cache); // Release lock before spawning
                        let inner_clone = inner.clone();
                        let cache_clone = functions_cache.clone();
                        tokio::spawn(async move {
                            let fresh = inner_clone.list_functions().await;
                            if let Ok(mut cache) = cache_clone.lock() {
                                cache.put(key_clone, CacheEntry::new(fresh));
                            }
                        });
                    }

                    return cached_value;
                }
            }

            // Cache miss - fetch from underlying catalog (blocking)
            let functions = inner.list_functions().await;

            // Update cache
            if let Ok(mut cache) = functions_cache.lock() {
                cache.put(key, CacheEntry::new(functions.clone()));
            }

            functions
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use querent_core::catalog::InMemoryCatalog;

    #[tokio::test]
    async fn test_cached_catalog() {
        let mut catalog = InMemoryCatalog::new();
        catalog.add_schema(schema::Schema::new("test_schema".to_string()));

        let cached = CachedCatalog::new(catalog);

        // First call should fetch from underlying catalog
        let schemas = cached.list_schemas().await;
        assert!(schemas.contains(&"test_schema".to_string()));

        // Second call should use cache
        let schemas2 = cached.list_schemas().await;
        assert_eq!(schemas, schemas2);

        // Clear cache
        cached.clear();
    }

    #[tokio::test]
    async fn test_cached_catalog_with_ttl() {
        use std::time::Duration;

        let mut catalog = InMemoryCatalog::new();
        catalog.add_schema(schema::Schema::new("test_schema".to_string()));

        // Create with 100ms TTL
        let cached =
            CachedCatalog::with_capacity_and_ttl(catalog, 100, Some(Duration::from_millis(100)));

        // First call should fetch from underlying catalog
        let schemas = cached.list_schemas().await;
        assert!(schemas.contains(&"test_schema".to_string()));

        // Second call should use cache (within TTL)
        let schemas2 = cached.list_schemas().await;
        assert_eq!(schemas, schemas2);

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_millis(150)).await;

        // This should fetch fresh data (cache expired)
        let schemas3 = cached.list_schemas().await;
        assert_eq!(schemas, schemas3);
    }
}
