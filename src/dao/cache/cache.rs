use moka::future::Cache;
use std::time::Duration;
use std::sync::Arc;

#[derive(Clone)]
pub struct CacheService<K, V> {
    cache: Arc<Cache<K, V>>,
}

impl<K, V> CacheService<K, V>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// 新建缓存服务
    pub fn new(ttl: Duration, max_capacity: u64) -> Self {
        let cache = Cache::builder()
            .time_to_live(ttl)
            .max_capacity(max_capacity)
            .build();
        CacheService {
            cache: Arc::new(cache),
        }
    }

    /// 获取缓存，如果没有命中则返回 None
    pub async fn get(&self, key: &K) -> Option<V> {
        self.cache.get(key).await
    }

    /// 获取缓存，如果没有命中，则调用 loader 加载
    pub async fn get_or_load<F, Fut>(&self, key: K, loader: F) -> V
    where
        F: FnOnce(K) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = V> + Send,
    {
        self.cache
            .get_with(key.clone(), async move { loader(key).await })
            .await
    }

    /// 强制写入缓存
    pub async fn insert(&self, key: K, value: V) {
        self.cache.insert(key, value).await;
    }

    /// 删除某个 key
    pub async fn invalidate(&self, key: &K) {
        self.cache.invalidate(key).await;
    }
}
