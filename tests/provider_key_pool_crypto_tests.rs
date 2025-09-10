use project_rust_learn::dao::provider_key_pool::crypto::{
    generate_key_hash, encrypt_api_key, decrypt_api_key, process_api_key, verify_key_integrity
};

#[tokio::test]
async fn test_simple_crypto_functions() {
    println!("=== Testing Simple Crypto Functions ===");
    
    // Test 1: Key hash generation
    println!("\nTesting key hash generation...");
    let api_key = "sk-1234567890abcdef";
    let hash1 = generate_key_hash(api_key);
    let hash2 = generate_key_hash(api_key);
    
    println!("✅ Hash 1: {}", hash1);
    println!("✅ Hash 2: {}", hash2);
    assert_eq!(hash1, hash2, "Same input should produce same hash");
    assert_eq!(hash1.len(), 64, "SHA-256 hash should be 64 characters");
    
    // Test 2: Encryption and decryption
    println!("\nTesting encryption and decryption...");
    let encrypted = encrypt_api_key(api_key).expect("Encryption failed");
    println!("✅ Encrypted: {}", encrypted);
    
    let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
    println!("✅ Decrypted: {}", decrypted);
    assert_eq!(api_key, decrypted, "Decrypted key should match original");
    
    // Test 3: Process API key (hash + encrypt)
    println!("\nTesting process_api_key function...");
    let (hash, encrypted) = process_api_key(api_key).expect("Process API key failed");
    println!("✅ Generated hash: {}", hash);
    println!("✅ Generated encrypted: {}", encrypted);
    
    assert_eq!(hash, generate_key_hash(api_key), "Hash should match direct generation");
    let decrypted = decrypt_api_key(&encrypted).expect("Failed to decrypt processed key");
    assert_eq!(decrypted, api_key, "Processed key should decrypt to original");
    
    // Test 4: Key integrity verification
    println!("\nTesting key integrity verification...");
    assert!(verify_key_integrity(&decrypted, &hash), "Key integrity should pass for valid key");
    assert!(!verify_key_integrity("wrong-key", &hash), "Key integrity should fail for wrong key");
    
    println!("✅ All simple crypto function tests passed!");
}

#[test]
fn test_sync_hash_function() {
    println!("=== Testing Sync Hash Function ===");
    
    let test_keys = vec![
        "sk-1234567890",
        "gsk_abcdefghij",
        "sk-ant-api03-xyz",
        "",
        "🔑🔐🗝️",
    ];
    
    for key in test_keys {
        let display_key = if key.len() > 20 { 
            format!("{}...({}chars)", &key[..20], key.len()) 
        } else { 
            key.to_string() 
        };
        println!("\nTesting key: {}", display_key);
        
        // Generate hash multiple times
        let hash1 = generate_key_hash(key);
        let hash2 = generate_key_hash(key);
        let hash3 = generate_key_hash(key);
        
        assert_eq!(hash1, hash2, "Hash should be consistent");
        assert_eq!(hash2, hash3, "Hash should be consistent");
        assert_eq!(hash1.len(), 64, "Hash should be 64 characters");
        
        // Verify hash is valid hex
        assert!(hash1.chars().all(|c| c.is_ascii_hexdigit()), "Hash should be valid hex");
        
        println!("✅ Hash consistency verified: {}", &hash1[..16]);
    }
    
    println!("\n✅ All sync hash function tests passed!");
}

#[test]
fn test_data_integrity() {
    println!("=== Testing Data Integrity ===");
    
    let test_data = vec![
        "simple-key",
        "sk-1234567890abcdefghijklmnopqrstuvwxyz",
        "key with spaces and symbols !@#$%^&*()",
        "",
        " ",
    ];
    
    for (i, original) in test_data.iter().enumerate() {
        println!("\nTesting data integrity for case {}: {} chars", i + 1, original.len());
        
        // Test encrypt/decrypt
        let encrypted = encrypt_api_key(original).expect("Encryption should succeed");
        let decrypted = decrypt_api_key(&encrypted).expect("Decryption should succeed");
        
        assert_eq!(decrypted, *original, "Data should remain intact");
        println!("✅ Data integrity maintained for case {}", i + 1);
    }
    
    println!("\n✅ All data integrity tests passed!");
}
