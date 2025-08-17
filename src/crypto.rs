use anyhow::anyhow;
use chacha20poly1305::{
    aead::{stream, Aead, NewAead},
    XChaCha20Poly1305,
};
use rand::{rngs::OsRng, RngCore};
use std::{
    fs::{self, File},
    io::{Read, Write},
};

pub fn encrypt_main(){
    let mut small_file_key = [0u8; 32];
    let mut small_file_nonce = [0u8; 24];

    small_file_key = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31];
    small_file_nonce = [0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23];

    encrypt_small_file("/home/admin-andrei/Downloads/unencrypted.txt", "/home/admin-andrei/Downloads/encrypted.txt", &small_file_key, &small_file_nonce);
    decrypt_small_file("/home/admin-andrei/Downloads/encrypted.txt", "/home/admin-andrei/Downloads/decrypted.txt", &small_file_key, &small_file_nonce);
}

fn encrypt_small_file(
    filepath: &str,
    dist: &str,
    key: &[u8; 32],
    nonce: &[u8; 24],
) -> Result<(), anyhow::Error> {
    let cipher = XChaCha20Poly1305::new(key.into());

    let file_data = fs::read(filepath)?;

    let encrypted_file = cipher
        .encrypt(nonce.into(), file_data.as_ref())
        .map_err(|err| anyhow!("Encrypting small file: {}", err))?;

    fs::write(&dist, encrypted_file)?;

    Ok(())
}

fn decrypt_small_file(
    encrypted_file_path: &str,
    dist: &str,
    key: &[u8; 32],
    nonce: &[u8; 24],
) -> Result<(), anyhow::Error> {
    let cipher = XChaCha20Poly1305::new(key.into());

    let file_data = fs::read(encrypted_file_path)?;

    let decrypted_file = cipher
        .decrypt(nonce.into(), file_data.as_ref())
        .map_err(|err| anyhow!("Decrypting small file: {}", err))?;

    fs::write(&dist, decrypted_file)?;

    Ok(())
}
