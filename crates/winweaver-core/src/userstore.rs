//! The encrypted passwords of a users file (`docs/spec/02-documents.md` section 11): a
//! portable container under a master password, bound to no machine. PBKDF2-HMAC-SHA256 makes
//! 64 bytes of the master password and a salt: the first 32 encrypt with AES-256-CBC (PKCS#7),
//! the last 32 authenticate the salt, the vector and the ciphertext with HMAC-SHA256. The MAC is
//! checked before anything is decrypted, and a wrong master password and a changed byte give one
//! and the same message. Windows PowerShell 5.1 reads the container with the same steps in the
//! script `Create-Users.runtime.ps1`.

use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use zeroize::{Zeroize, Zeroizing};

pub const KDF: &str = "pbkdf2-sha256";
pub const DEFAULT_ITERATIONS: u32 = 600_000;
pub const MIN_ITERATIONS: u32 = 100_000;
const SALT_SIZE: usize = 32;
const IV_SIZE: usize = 16;
const KEY_SIZE: usize = 32;
/// The one message of a wrong master password and of a changed container.
pub const WRONG_PASSWORD: &str = "Wrong master password or a damaged users file";

/// The container as the users file keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Container {
    pub kdf: String,
    pub iterations: u32,
    pub salt: String,
    pub iv: String,
    pub mac: String,
    pub data: String,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            kdf: KDF.to_owned(),
            iterations: DEFAULT_ITERATIONS,
            salt: String::new(),
            iv: String::new(),
            mac: String::new(),
            data: String::new(),
        }
    }
}

/// The passwords of the accounts by name, in memory while the file is unlocked; the texts are
/// wiped when the value is dropped. The names compare without case, as Windows does.
#[derive(Default)]
pub struct Passwords {
    map: BTreeMap<String, String>,
}

impl std::fmt::Debug for Passwords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Passwords({} entries)", self.map.len())
    }
}

impl Drop for Passwords {
    fn drop(&mut self) {
        for (mut name, mut password) in std::mem::take(&mut self.map) {
            name.zeroize();
            password.zeroize();
        }
    }
}

impl Passwords {
    fn key(&self, name: &str) -> Option<String> {
        let wanted = name.trim();
        self.map
            .keys()
            .find(|k| k.eq_ignore_ascii_case(wanted))
            .cloned()
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        let key = self.key(name)?;
        self.map.get(&key).map(String::as_str)
    }

    pub fn has(&self, name: &str) -> bool {
        self.key(name).is_some()
    }

    /// The names with a password, as they are kept.
    pub fn names(&self) -> Vec<String> {
        self.map.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Sets the password of a name; a name kept in another case is replaced.
    pub fn set(&mut self, name: &str, password: &str) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if let Some(old) = self.key(name)
            && old != name
            && let Some(mut gone) = self.map.remove(&old)
        {
            gone.zeroize();
        }
        self.map.insert(name.to_owned(), password.to_owned());
    }

    /// Removes the password of a name; `true` when there was one.
    pub fn remove(&mut self, name: &str) -> bool {
        match self.key(name) {
            Some(key) => {
                if let Some(mut gone) = self.map.remove(&key) {
                    gone.zeroize();
                }
                true
            }
            None => false,
        }
    }

    /// Gives the password of a name to another name, when it has one.
    pub fn rename(&mut self, old: &str, new: &str) {
        if let Some(key) = self.key(old)
            && let Some(password) = self.map.remove(&key)
        {
            self.map.insert(new.trim().to_owned(), password);
        }
    }

    /// The map a container holds: names in the order of their code points, a password each.
    fn plaintext(&self) -> Result<Zeroizing<Vec<u8>>, String> {
        serde_json::to_vec(&self.map)
            .map(Zeroizing::new)
            .map_err(|e| e.to_string())
    }

    fn from_plaintext(bytes: &[u8]) -> Result<Self, String> {
        let map: BTreeMap<String, String> =
            serde_json::from_slice(bytes).map_err(|_| WRONG_PASSWORD.to_owned())?;
        if map.keys().any(|k| k.trim().is_empty()) {
            return Err(WRONG_PASSWORD.to_owned());
        }
        Ok(Self { map })
    }
}

/// The encryption key and the MAC key of a master password and a salt.
type Key = Zeroizing<[u8; KEY_SIZE]>;

fn keys(master: &[u8], salt: &[u8], iterations: u32) -> (Key, Key) {
    let mut derived = Zeroizing::new([0u8; KEY_SIZE * 2]);
    pbkdf2::pbkdf2_hmac::<Sha256>(master, salt, iterations, derived.as_mut());
    let mut enc = Zeroizing::new([0u8; KEY_SIZE]);
    let mut mac = Zeroizing::new([0u8; KEY_SIZE]);
    enc.copy_from_slice(&derived[..KEY_SIZE]);
    mac.copy_from_slice(&derived[KEY_SIZE..]);
    (enc, mac)
}

fn tag(key: &[u8], salt: &[u8], iv: &[u8], data: &[u8]) -> Hmac<Sha256> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("any key length");
    mac.update(salt);
    mac.update(iv);
    mac.update(data);
    mac
}

/// Encrypts the passwords under a master password with a fresh salt and vector.
pub fn encrypt(
    passwords: &Passwords,
    master: &[u8],
    iterations: u32,
) -> Result<Container, String> {
    let mut salt = [0u8; SALT_SIZE];
    let mut iv = [0u8; IV_SIZE];
    getrandom::fill(&mut salt).map_err(|e| e.to_string())?;
    getrandom::fill(&mut iv).map_err(|e| e.to_string())?;
    encrypt_with(passwords, master, iterations, &salt, &iv)
}

/// [`encrypt`] with the salt and the vector given, for the tests.
pub fn encrypt_with(
    passwords: &Passwords,
    master: &[u8],
    iterations: u32,
    salt: &[u8],
    iv: &[u8],
) -> Result<Container, String> {
    if iterations < MIN_ITERATIONS {
        let why = format!("too few iterations: {iterations} (at least {MIN_ITERATIONS})");
        return Err(why);
    }
    if salt.len() != SALT_SIZE || iv.len() != IV_SIZE {
        return Err("the salt is 32 bytes and the vector 16".to_owned());
    }
    let plain = passwords.plaintext()?;
    let (enc_key, mac_key) = keys(master, salt, iterations);
    let cipher = cbc::Encryptor::<aes::Aes256>::new_from_slices(enc_key.as_ref(), iv)
        .map_err(|e| e.to_string())?;
    let data = cipher.encrypt_padded_vec_mut::<Pkcs7>(&plain);
    let mac = tag(mac_key.as_ref(), salt, iv, &data).finalize().into_bytes();
    Ok(Container {
        kdf: KDF.to_owned(),
        iterations,
        salt: BASE64.encode(salt),
        iv: BASE64.encode(iv),
        mac: BASE64.encode(mac),
        data: BASE64.encode(&data),
    })
}

/// The structure of a container, apart from its secrets: the message names what is wrong.
pub fn check(container: &Container) -> Result<(), String> {
    if container.kdf != KDF {
        return Err(format!("unknown key derivation {}", container.kdf));
    }
    if container.iterations < MIN_ITERATIONS {
        let n = container.iterations;
        return Err(format!("too few iterations: {n} (at least {MIN_ITERATIONS})"));
    }
    let field = |name: &str, text: &str, size: Option<usize>| -> Result<Vec<u8>, String> {
        let bytes = BASE64
            .decode(text)
            .map_err(|_| format!("the passwords have an invalid {name}"))?;
        if size.is_some_and(|s| bytes.len() != s) {
            return Err(format!("the passwords have an invalid {name}"));
        }
        Ok(bytes)
    };
    field("salt", &container.salt, Some(SALT_SIZE))?;
    field("iv", &container.iv, Some(IV_SIZE))?;
    field("mac", &container.mac, Some(KEY_SIZE))?;
    let data = field("data", &container.data, None)?;
    if data.is_empty() || !data.len().is_multiple_of(IV_SIZE) {
        return Err("the passwords have an invalid data length".to_owned());
    }
    Ok(())
}

/// Decrypts a container; the MAC is verified first, and any failure after the structure check
/// gives [`WRONG_PASSWORD`].
pub fn decrypt(container: &Container, master: &[u8]) -> Result<Passwords, String> {
    check(container)?;
    let decode = |text: &str| BASE64.decode(text).unwrap_or_default();
    let (salt, iv, mac, data) = (
        decode(&container.salt),
        decode(&container.iv),
        decode(&container.mac),
        decode(&container.data),
    );
    let (enc_key, mac_key) = keys(master, &salt, container.iterations);
    tag(mac_key.as_ref(), &salt, &iv, &data)
        .verify_slice(&mac)
        .map_err(|_| WRONG_PASSWORD.to_owned())?;
    let cipher = cbc::Decryptor::<aes::Aes256>::new_from_slices(enc_key.as_ref(), &iv)
        .map_err(|_| WRONG_PASSWORD.to_owned())?;
    let plain = cipher
        .decrypt_padded_vec_mut::<Pkcs7>(&data)
        .map(Zeroizing::new)
        .map_err(|_| WRONG_PASSWORD.to_owned())?;
    Passwords::from_plaintext(&plain)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Passwords {
        let mut p = Passwords::default();
        p.set("user1", "p@ss w0rd");
        p.set("admin1", "Password1!");
        p.set("unicode", "Parol-\u{42e}\u{e4}\u{f6}");
        p
    }

    #[test]
    fn the_names_compare_without_case_and_the_text_is_sorted() {
        let mut p = sample();
        assert_eq!(p.get("ADMIN1"), Some("Password1!"));
        assert!(p.has("User1") && !p.has("nobody"));
        p.set("Admin1", "New1!");
        assert_eq!(p.len(), 3);
        assert_eq!(p.get("admin1"), Some("New1!"));
        p.rename("user1", "user2");
        assert_eq!(p.get("user2"), Some("p@ss w0rd"));
        assert!(p.remove("USER2") && !p.remove("user2"));
        let text = sample().plaintext().unwrap();
        let want = concat!(
            "{\"admin1\":\"Password1!\",\"unicode\":\"Parol-\u{42e}\u{e4}\u{f6}\",",
            "\"user1\":\"p@ss w0rd\"}"
        );
        assert_eq!(std::str::from_utf8(&text).unwrap(), want);
    }

    /// The values a Python implementation (hashlib and cryptography) gave for the same inputs.
    #[test]
    fn the_container_matches_the_reference_and_opens_again() {
        let master = "Correct Horse Battery Staple \u{42e}\u{e4}\u{f6}".as_bytes();
        let salt: Vec<u8> = (0u8..32).collect();
        let iv: Vec<u8> = (16u8..32).collect();
        let c = encrypt_with(&sample(), master, MIN_ITERATIONS, &salt, &iv).unwrap();
        assert_eq!(c.salt, "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=");
        assert_eq!(c.iv, "EBESExQVFhcYGRobHB0eHw==");
        assert_eq!(c.data, REFERENCE_DATA);
        assert_eq!(c.mac, REFERENCE_MAC);
        let back = decrypt(&c, master).unwrap();
        assert_eq!(back.get("unicode"), Some("Parol-\u{42e}\u{e4}\u{f6}"));
        assert_eq!(back.names(), vec!["admin1", "unicode", "user1"]);
        assert_eq!(decrypt(&c, b"wrong").unwrap_err(), WRONG_PASSWORD);
        let mut damaged = c.clone();
        let mut bytes = BASE64.decode(&damaged.data).unwrap();
        bytes[5] ^= 1;
        damaged.data = BASE64.encode(bytes);
        assert_eq!(decrypt(&damaged, master).unwrap_err(), WRONG_PASSWORD);
        let mut short = c.clone();
        short.iv = "AAAA".to_owned();
        assert!(decrypt(&short, master).unwrap_err().contains("invalid iv"));
        let fresh = encrypt(&sample(), master, MIN_ITERATIONS).unwrap();
        assert_ne!(fresh.salt, c.salt);
        assert_eq!(decrypt(&fresh, master).unwrap().len(), 3);
    }

    const REFERENCE_DATA: &str = concat!(
        "nInWvk0NrhYGFX6Bl4K1VNjBHtEl7twHjFW0nGazWH6tt9MkNlLI1Z9yifSKlocR",
        "rb6wd8ftc28vR6vSPRxN3lIylYekInfQ2VhV30BUJS0="
    );
    const REFERENCE_MAC: &str = "Ud8F0VLd47QnBsvorhxPyAl+wlxCcHu55U2besdAI9w=";
}
