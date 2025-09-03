use anyhow::anyhow;
use argon2::Argon2;
use chacha20poly1305::{
    XChaCha20Poly1305,
    aead::{Aead, NewAead},
};
use std::fs;

pub fn derive_save_nonce(key: &[u8; 32], base_nonce: &[u8; 24], counter: u64) -> [u8; 24] {
    let mut hasher = blake3::Hasher::new_keyed(key);
    hasher.update(base_nonce);
    hasher.update(&counter.to_le_bytes());
    let out = hasher.finalize();
    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&out.as_bytes()[..24]);

    nonce
}

pub fn derivate_crypto_params(passphrase: String) -> ([u8; 32], [u8; 24]) {
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 24];
    let salt = b"saltyMcSaltface";
    let mut argon2_output = [0u8; 56];

    Argon2::default()
        .hash_password_into(passphrase.as_bytes(), salt, &mut argon2_output)
        .expect("Failed to hash password");

    key.copy_from_slice(&argon2_output[..32]);
    nonce.copy_from_slice(&argon2_output[32..56]);
    (key, nonce)
}

pub fn encrypt_data_to_file(
    file_data: &Vec<u8>,
    dist: &str,
    key: &[u8; 32],
    nonce: &[u8; 24],
) -> Result<(), anyhow::Error> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let encrypted_file = cipher
        .encrypt(nonce.into(), file_data.as_ref())
        .map_err(|err| anyhow!("Encrypting small file: {}", err))?;

    fs::write(dist, encrypted_file)?;

    Ok(())
}

pub fn decrypt_file(
    encrypted_file_path: &str,
    key: &[u8; 32],
    nonce: &[u8; 24],
) -> (Result<(), anyhow::Error>, Vec<u8>) {
    let cipher = XChaCha20Poly1305::new(key.into());

    let file_data = match fs::read(encrypted_file_path) {
        Ok(data) => data,
        Err(err) => {
            return (
                Err(anyhow!("Failed to read encrypted file: {}", err)),
                Vec::new(),
            );
        }
    };

    let decrypted_data = match cipher.decrypt(nonce.into(), file_data.as_ref()) {
        Ok(data) => data,
        Err(err) => return (Err(anyhow!("Decrypting small file: {}", err)), Vec::new()),
    };

    (Ok(()), decrypted_data)
}
