mod provider_key_pool;
pub mod preload;
pub mod crypto;

pub use provider_key_pool::{
    ProviderKeyPool, 
    create_provider_key_pool, 
    get_provider_key_pool_by_id,
    list_provider_key_pools,
    list_provider_key_pools_by_provider,
    list_active_provider_key_pools,
    update_provider_key_pool,
    update_key_pool_usage,
    delete_provider_key_pool,
    toggle_provider_key_pool_active,
    create_provider_key_pool_from_raw_key
};

pub use preload::{
    CachedProviderKeyPool,
    preload_provider_key_pools_to_cache,
    get_provider_key_pool_from_cache,
    insert_provider_key_pool_to_cache,
    insert_cached_provider_key_pool_to_cache,
    get_decrypted_api_key_from_cache,
    get_api_key_round_robin,
    reload_provider_api_keys,
    reset_round_robin_counter,
    get_round_robin_counter,
    get_active_key_count
};

pub use crypto::{
    generate_key_hash,
    encrypt_api_key,
    decrypt_api_key,
    process_api_key,
    verify_key_integrity
};
