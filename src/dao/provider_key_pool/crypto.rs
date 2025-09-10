use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce, Key
};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use rand::Rng;
use anyhow::{Result, anyhow};

/// 固定的加密密钥 - 在生产环境中应该从环境变量或配置文件中读取
const ENCRYPTION_KEY: &[u8; 32] = b"my_very_secure_32_byte_secret_k!";

/// 从原始API密钥生成SHA-256哈希
/// 
/// # Arguments
/// * `api_key` - 原始API密钥字符串
/// 
/// # Returns
/// * SHA-256哈希的十六进制字符串
pub fn generate_key_hash(api_key: &str) -> String {
    let mut hasher = Sha256::default();
    hasher.update(api_key.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

/// 使用AES-256-GCM加密API密钥
/// 
/// # Arguments
/// * `api_key` - 原始API密钥字符串
/// 
/// # Returns
/// * `Ok(String)` - Base64编码的加密数据(包含nonce)
/// * `Err(anyhow::Error)` - 加密失败
pub fn encrypt_api_key(api_key: &str) -> Result<String> {
    // 创建AES-256-GCM实例
    let key = Key::<Aes256Gcm>::from_slice(ENCRYPTION_KEY);
    let cipher = Aes256Gcm::new(key);
    
    // 生成随机nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    
    // 加密
    let ciphertext = cipher
        .encrypt(nonce, api_key.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;
    
    // 将nonce和密文组合并进行Base64编码
    let mut encrypted_data = nonce_bytes.to_vec();
    encrypted_data.extend_from_slice(&ciphertext);
    
    Ok(general_purpose::STANDARD.encode(&encrypted_data))
}

/// 使用AES-256-GCM解密API密钥
/// 
/// # Arguments
/// * `encrypted_data` - Base64编码的加密数据(包含nonce)
/// 
/// # Returns
/// * `Ok(String)` - 解密后的原始API密钥
/// * `Err(anyhow::Error)` - 解密失败
pub fn decrypt_api_key(encrypted_data: &str) -> Result<String> {
    // Base64解码
    let encrypted_bytes = general_purpose::STANDARD
        .decode(encrypted_data)
        .map_err(|e| anyhow!("Base64 decode failed: {}", e))?;
    
    if encrypted_bytes.len() < 12 {
        return Err(anyhow!("Invalid encrypted data: too short"));
    }
    
    // 分离nonce和密文
    let (nonce_bytes, ciphertext) = encrypted_bytes.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);
    
    // 创建AES-256-GCM实例
    let key = Key::<Aes256Gcm>::from_slice(ENCRYPTION_KEY);
    let cipher = Aes256Gcm::new(key);
    
    // 解密
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    
    String::from_utf8(plaintext)
        .map_err(|e| anyhow!("UTF-8 conversion failed: {}", e))
}

/// 从原始API密钥创建ProviderKeyPool所需的加密数据
/// 
/// # Arguments
/// * `api_key` - 原始API密钥字符串
/// 
/// # Returns
/// * `Ok((key_hash, encrypted_key_value))` - 哈希和加密后的密钥值
/// * `Err(anyhow::Error)` - 处理失败
pub fn process_api_key(api_key: &str) -> Result<(String, String)> {
    let key_hash = generate_key_hash(api_key);
    let encrypted_value = encrypt_api_key(api_key)?;
    Ok((key_hash, encrypted_value))
}

/// 验证解密后的密钥是否与原始哈希匹配
/// 
/// # Arguments
/// * `decrypted_key` - 解密后的API密钥
/// * `stored_hash` - 存储的密钥哈希
/// 
/// # Returns
/// * `bool` - 是否匹配
pub fn verify_key_integrity(decrypted_key: &str, stored_hash: &str) -> bool {
    let computed_hash = generate_key_hash(decrypted_key);
    computed_hash == stored_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_hash_generation() {
        let api_key = "sk-1234567890abcdef";
        let hash1 = generate_key_hash(api_key);
        let hash2 = generate_key_hash(api_key);
        
        // 相同输入应该产生相同哈希
        assert_eq!(hash1, hash2);
        
        // 哈希应该是64个字符(SHA-256的十六进制表示)
        assert_eq!(hash1.len(), 64);
        
        // 不同输入应该产生不同哈希
        let different_hash = generate_key_hash("different-key");
        assert_ne!(hash1, different_hash);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let original_key = "sk-1234567890abcdef";
        
        // 加密
        let encrypted = encrypt_api_key(original_key).expect("Encryption failed");
        
        // 解密
        let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
        
        // 验证往返过程
        assert_eq!(original_key, decrypted);
    }

    #[test]
    fn test_encrypt_produces_different_outputs() {
        let api_key = "sk-1234567890abcdef";
        
        let encrypted1 = encrypt_api_key(api_key).expect("Encryption 1 failed");
        let encrypted2 = encrypt_api_key(api_key).expect("Encryption 2 failed");
        
        // 由于使用随机nonce，每次加密应该产生不同的输出
        assert_ne!(encrypted1, encrypted2);
        
        // 但解密结果应该相同
        let decrypted1 = decrypt_api_key(&encrypted1).expect("Decryption 1 failed");
        let decrypted2 = decrypt_api_key(&encrypted2).expect("Decryption 2 failed");
        assert_eq!(decrypted1, decrypted2);
        assert_eq!(decrypted1, api_key);
    }

    #[test]
    fn test_process_api_key() {
        let api_key = "sk-1234567890abcdef";
        
        let (hash, encrypted) = process_api_key(api_key).expect("Process failed");
        
        // 验证哈希
        let expected_hash = generate_key_hash(api_key);
        assert_eq!(hash, expected_hash);
        
        // 验证加密
        let decrypted = decrypt_api_key(&encrypted).expect("Decryption failed");
        assert_eq!(decrypted, api_key);
    }

    #[test]
    fn test_verify_key_integrity() {
        let api_key = "sk-1234567890abcdef";
        let hash = generate_key_hash(api_key);
        
        // 正确的密钥应该验证通过
        assert!(verify_key_integrity(api_key, &hash));
        
        // 错误的密钥应该验证失败
        assert!(!verify_key_integrity("wrong-key", &hash));
    }

    #[test]
    fn test_decrypt_invalid_data() {
        // 测试无效的Base64数据
        assert!(decrypt_api_key("invalid-base64!").is_err());
        
        // 测试太短的数据
        let short_data = general_purpose::STANDARD.encode(b"short");
        assert!(decrypt_api_key(&short_data).is_err());
        
        // 测试有效Base64但无效加密数据
        let invalid_encrypted = general_purpose::STANDARD.encode(b"this_is_exactly_12_bytes_but_invalid_ciphertext");
        assert!(decrypt_api_key(&invalid_encrypted).is_err());
    }
}
