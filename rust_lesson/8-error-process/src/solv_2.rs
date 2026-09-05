fn xor_encrypt_decrypt(input: &[u8], key: &[u8]) -> Vec<u8> {
    input.iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key.len()])
        .collect()
}

fn main() {
    let plaintext = "Hello, Rust Crypto!";
    let key = b"secret";
    
    let encrypted = xor_encrypt_decrypt(plaintext.as_bytes(), key);
    println!("暗号文 (hex): {}", hex_encode(&encrypted));
    
    let decrypted = xor_encrypt_decrypt(&encrypted, key);
    println!("復号文: {}", String::from_utf8_lossy(&decrypted));
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};

fn main() {
    // 1. 鍵の生成（32バイト = 256ビット）
    let key = Aes256Gcm::generate_key(&mut OsRng);
    let cipher = Aes256Gcm::new(&key);
    
    // 2. ノンスの生成（12バイト：毎回異なる値を使う必がある）
    let nonce = Nonce::from_slice(b"unique nonce"); // 実際は毎回ランダム生成
    
    let plaintext = b"機密情報: PPTPは推奨されません";
    
    // 3. 暗号化
    let ciphertext = cipher.encrypt(nonce, plaintext.as_ref())
        .expect("暗号化に失敗");
    
    println!("暗号文 (hex): {}", hex::encode(&ciphertext));
    
    // 4. 復号
    let decrypted = cipher.decrypt(nonce, ciphertext.as_ref())
        .expect("復号に失敗");
    
    println!("復号文: {}", String::from_utf8_lossy(&decrypted));
}

[dependencies]
aes-gcm = "0.10"
hex = "0.4"
rand = "0.8"

fn caesar_encrypt(text: &str, shift: u8) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                let base = if c.is_ascii_uppercase() { b'A' } else { b'a' };
                (((c as u8 - base + shift) % 26) + base) as char
            } else {
                c
            }
        })
        .collect()
}

fn caesar_decrypt(text: &str, shift: u8) -> String {
    caesar_encrypt(text, 26 - (shift % 26))
}

fn main() {
    let text = "Hello World";
    let shift = 3;
    
    let enc = caesar_encrypt(text, shift);
    println!("暗号文: {}", enc);
    
    let dec = caesar_decrypt(&enc, shift);
    println!("復号文: {}", dec);
}