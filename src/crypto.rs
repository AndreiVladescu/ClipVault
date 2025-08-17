use anyhow::anyhow;
use chacha20poly1305::{
    aead::{stream, Aead, NewAead},
    XChaCha20Poly1305,
};
use std::{
    fs::{self, File},
    io::{Read, Write},
};
use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHash, PasswordHasher, PasswordVerifier, SaltString
    },
    Argon2
};

//https://kerkour.com/rust-file-encryption

pub fn derivate_crypto_params(passphrase: String) -> ([u8; 32], [u8; 24]) {
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 24];
    let salt = b"saltyMcSaltface";
    let mut argon2_output = [0u8; 56];

    Argon2::default().hash_password_into(passphrase.as_bytes(), salt, &mut argon2_output)
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

    fs::write(&dist, encrypted_file)?;

    Ok(())
}

pub fn decrypt_file(
    encrypted_file_path: &str,
    dist: &str,
    key: &[u8; 32],
    nonce: &[u8; 24],
) -> Result<(), anyhow::Error> {
    let cipher = XChaCha20Poly1305::new(key.into());

    let file_data = match fs::read(encrypted_file_path) {
        Ok(data) => data,
        Err(err) => return Err(anyhow!("Failed to read encrypted file: {}", err)),
    };
    let decrypted_file = cipher
        .decrypt(nonce.into(), file_data.as_ref())
        .map_err(|err| anyhow!("Decrypting small file: {}", err))?;

    fs::write(&dist, decrypted_file)?;

    Ok(())
}
