mod dao;
mod llm_api;
mod logger;

use dao::{SQLITE_POOL, init_sqlite_pool, init_db};
use dao::cache::{init_global_cache};
use logger::init_dev_logger;
use tracing::{info, error, warn, debug};
use crate::llm_api::ollama::client;

#[tokio::main]
async fn main() {
    //* 
    //* Initialize logger
    //* 
    if let Err(e) = init_dev_logger() {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    }
    info!("Logger initialized successfully");

    //* 
    //* Initialize database
    //* 
    info!("Initializing database...");
    // Initialize the SQLite connection pool
    init_sqlite_pool("sqlite://data/app.db").await;
    // Get a reference to the connection pool
    let pool = SQLITE_POOL.get().unwrap().clone();
    // Initialize the database using the SQL script
    match init_db("data/init.sql").await {
        Ok(_) => info!("Database initialized successfully"),
        Err(e) => {
            error!("DB init failed: {}", e);
            std::process::exit(1);
        }
    }
    //*
    //* Test data for Provider Key Pool
    //*
    // use dao::provider_key_pool::{create_provider_key_pool_from_raw_key};
    
    // create_provider_key_pool_from_raw_key(
    //     &pool,
    //     uuid::Uuid::new_v4().to_string(),
    //     "openai".to_string(),
    //     "sk-test-openai-1234567890".to_string().as_str(),
    //     true,
    //     Some(60),
    //     Some(3600),
    // ).await.expect("Failed to create provider key pool entry");

    

    //* 
    //* Initialize memory cache
    //* 
    info!("Initializing memory cache...");
    // Initialize global cache with 1 hour TTL and max 1000 entries
    match init_global_cache(&pool, 3600, 1000).await {
        Ok(_) => info!("Global cache initialized successfully"),
        Err(e) => {
            error!("Cache init failed: {}", e);
            std::process::exit(1);
        }
    }
    //*
    use dao::provider_key_pool::{get_decrypted_api_key_from_cache};
    let cached_key_pool = get_decrypted_api_key_from_cache("openai","7bf8037c-5a85-4e00-a7b5-098ca4ab8cf7").await.expect("Failed to get decrypted API key from cache");
    debug!("Decrypted API Key from cache: {:?}", cached_key_pool);

    
    info!("Application started successfully!");
    info!("Database and cache have been initialized.");
    debug!("You can now run the tests with 'cargo test' to test the functionality.");

}
