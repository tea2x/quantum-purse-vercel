//! # QuantumPurse KeyVault
//!
//! This module provides a secure authentication interface for managing cryptographic keys in
//! QuantumPurse using WebAssembly. It leverages AES-GCM for encryption, Scrypt for key derivation,
//! and the SPHINCS+ signature scheme for post-quantum transaction signing. Sensitive data, including
//! the BIP39 mnemonic and derived SPHINCS+ private keys, is encrypted and stored in the browser via
//! IndexedDB, with access authenticated by user-provided passwords.

use bip39::{Language, Mnemonic};
use ckb_fips205_utils::{
    ckb_tx_message_all_from_mock_tx::{generate_ckb_tx_message_all_from_mock_tx, ScriptOrIndex},
    Hasher,
};
use ckb_mock_tx_types::{MockTransaction, ReprMockTransaction};
use fips205::{
    traits::{KeyGen, SerDes, Signer},
    *,
};
use hex::encode;
use indexed_db_futures::{
    error::Error as DBError, iter::ArrayMapIter, prelude::*, transaction::TransactionMode,
};
use serde_wasm_bindgen;
use wasm_bindgen::{prelude::*, JsValue};
use web_sys::js_sys::Uint8Array;
use zeroize::Zeroize;

mod constants;
mod db;
mod macros;
mod secure_vec;
mod types;
mod utilities;

use crate::constants::{
    CHILD_KEYS_STORE, KDF_PATH_PREFIX, MULTISIG_RESERVED_FIELD_VALUE, PUBKEY_NUM, REQUIRED_FIRST_N,
    SEED_PHRASE_STORE, THRESHOLD,
};
use secure_vec::SecureVec;
use types::*;
use utilities::*;

////////////////////////////////////////////////////////////////////////////////
///  Key-vault functions
////////////////////////////////////////////////////////////////////////////////
#[wasm_bindgen]
pub struct KeyVault {
    /// The one parameter set chosen for QuantumPurse KeyVault setup in all 12 NIST-approved SPHINCS+ FIPS205 variants
    pub variant: SphincsVariant,
}

#[wasm_bindgen]
impl KeyVault {
    /// Constructs a new `KeyVault` to serve as a namespace in the output js interface.
    ///
    /// **Returns**:
    /// - `KeyVault` - A new instance of the struct.
    #[wasm_bindgen(constructor)]
    pub fn new(variant: SphincsVariant) -> Self {
        KeyVault { variant: variant }
    }

    /// To derive Sphincs key pair. One master mnemonic seed phrase can derive multiple child index-based sphincs+ key pairs on demand.
    ///
    /// **Parameters**:
    /// - `seed: &[u8]` - The master mnemonic seed phrase from which the child sphincs+ key is derived. MUST carry at least N*3 bytes of entropy or panics.
    /// - `index: u32` - The index of the child sphincs+ key to be derived.
    ///
    /// **Returns**:
    /// - `Result<SecureVec, String>` - Scrypt key on success, or an error message on failure.
    ///
    /// Warning: Proper zeroization of the input seed is the responsibility of the caller.
    fn derive_sphincs_key(
        &self,
        seed: &[u8],
        index: u32,
    ) -> Result<(SecureVec, SecureVec), String> {
        match self.variant {
            SphincsVariant::Sha2128S => sphincs_keygen!(slh_dsa_sha2_128s::KG, slh_dsa_sha2_128s::N, seed, index),
            SphincsVariant::Sha2128F => sphincs_keygen!(slh_dsa_sha2_128f::KG, slh_dsa_sha2_128f::N, seed, index),
            SphincsVariant::Sha2192S => sphincs_keygen!(slh_dsa_sha2_192s::KG, slh_dsa_sha2_192s::N, seed, index),
            SphincsVariant::Sha2192F => sphincs_keygen!(slh_dsa_sha2_192f::KG, slh_dsa_sha2_192f::N, seed, index),
            SphincsVariant::Sha2256S => sphincs_keygen!(slh_dsa_sha2_256s::KG, slh_dsa_sha2_256s::N, seed, index),
            SphincsVariant::Sha2256F => sphincs_keygen!(slh_dsa_sha2_256f::KG, slh_dsa_sha2_256f::N, seed, index),
            SphincsVariant::Shake128S => sphincs_keygen!(slh_dsa_shake_128s::KG, slh_dsa_shake_128s::N, seed, index),
            SphincsVariant::Shake128F => sphincs_keygen!(slh_dsa_shake_128f::KG, slh_dsa_shake_128f::N, seed, index),
            SphincsVariant::Shake192S => sphincs_keygen!(slh_dsa_shake_192s::KG, slh_dsa_shake_192s::N, seed, index),
            SphincsVariant::Shake192F => sphincs_keygen!(slh_dsa_shake_192f::KG, slh_dsa_shake_192f::N, seed, index),
            SphincsVariant::Shake256S => sphincs_keygen!(slh_dsa_shake_256s::KG, slh_dsa_shake_256s::N, seed, index),
            SphincsVariant::Shake256F => sphincs_keygen!(slh_dsa_shake_256f::KG, slh_dsa_shake_256f::N, seed, index),
        }
    }

    /// Clears all data in the `seed_phrase_store` and `child_keys_store` in IndexedDB.
    ///
    /// **Returns**:
    /// - `Result<(), JsValue>` - A JavaScript Promise that resolves to `undefined` on success,
    ///   or rejects with a JavaScript error on failure.
    ///
    /// **Async**: Yes
    #[wasm_bindgen]
    pub async fn clear_database() -> Result<(), JsValue> {
        let db = db::open_db().await.map_err(|e| e.to_jsvalue())?;
        db::clear_object_store(&db, SEED_PHRASE_STORE)
            .await
            .map_err(|e| e.to_jsvalue())?;
        db::clear_object_store(&db, CHILD_KEYS_STORE)
            .await
            .map_err(|e| e.to_jsvalue())?;
        Ok(())
    }

    /// Retrieves all SPHINCS+ lock script arguments (processed public keys) from the database in the order they get inserted.
    ///
    /// **Returns**:
    /// - `Result<Vec<String>, JsValue>` - A JavaScript Promise that resolves to an array of hex-encoded SPHINCS+ lock script arguments on success,
    ///   or rejects with a JavaScript error on failure.
    ///
    /// **Async**: Yes
    #[wasm_bindgen]
    pub async fn get_all_sphincs_lock_args() -> Result<Vec<String>, JsValue> {
        /// Error conversion helper
        fn map_db_error<T>(result: Result<T, DBError>) -> Result<T, JsValue> {
            result.map_err(|e| JsValue::from_str(&format!("Database error: {}", e)))
        }

        let db = db::open_db().await.map_err(|e| e.to_jsvalue())?;
        let tx = map_db_error(
            db.transaction(CHILD_KEYS_STORE)
                .with_mode(TransactionMode::Readonly)
                .build(),
        )?;
        let store = map_db_error(tx.object_store(CHILD_KEYS_STORE))?;

        // Retrieve all accounts
        let iter: ArrayMapIter<JsValue> = map_db_error(store.get_all().await)?;
        let mut accounts: Vec<SphincsPlusAccount> = Vec::new();
        for result in iter {
            let js_value = map_db_error(result)?;
            let account: SphincsPlusAccount = serde_wasm_bindgen::from_value(js_value)?;
            accounts.push(account);
        }

        // Sort by index
        accounts.sort_by_key(|account| account.index);

        // Extract lock args in sorted order
        let lock_args_array: Vec<String> = accounts
            .into_iter()
            .map(|account| account.lock_args)
            .collect();

        Ok(lock_args_array)
    }

    /// Initializes the mnemonic phrase by generating a BIP39 mnemonic, encrypting it with the provided password, and storing it in IndexedDB.
    ///
    /// **Parameters**:
    /// - `password: Uint8Array` - The password used to encrypt the mnemonic.
    ///
    /// **Returns**:
    /// - `Result<(), JsValue>` - A JavaScript Promise that resolves to `undefined` on success,
    ///   or rejects with a JavaScript error on failure.
    ///
    /// **Async**: Yes
    ///
    /// **Note**: Only effective when the mnemonic phrase is not yet set.
    #[wasm_bindgen]
    pub async fn init_seed_phrase(&self, password: Uint8Array) -> Result<(), JsValue> {
        let stored_seed = db::get_encrypted_mnemonic_seed()
            .await
            .map_err(|e| e.to_jsvalue())?;
        if stored_seed.is_some() {
            debug!("\x1b[37;44m INFO \x1b[0m \x1b[1mkey-vault\x1b[0m: mnemonic phrase exists");
            return Ok(());
        }

        let size = self.variant.bip39_compatible_entropy_size();
        let entropy = get_random_bytes(size).unwrap();
        let password = SecureVec::from_slice(&password.to_vec());
        let encrypted_seed = encrypt(&password, entropy.as_ref())
            .map_err(|e| JsValue::from_str(&format!("Encryption error: {}", e)))?;

        db::set_encrypted_mnemonic_seed(encrypted_seed)
            .await
            .map_err(|e| e.to_jsvalue())?;
        Ok(())
    }

    /// Generates a new SPHINCS+ account - a SPHINCS+ child account derived from the mnemonic phrase,
    /// encrypts the private key with the password, and stores/appends it in IndexedDB.
    ///
    /// **Parameters**:
    /// - `password: Uint8Array` - The password used to decrypt the mnemonic phrase and encrypt the child private key.
    ///
    /// **Returns**:
    /// - `Result<String, JsValue>` - A String Promise that resolves to the hex-encoded SPHINCS+ lock argument (processed SPHINCS+ public key) of the account on success,
    ///   or rejects with a JavaScript error on failure.
    ///
    /// **Async**: Yes
    #[wasm_bindgen]
    pub async fn gen_new_account(&self, password: Uint8Array) -> Result<String, JsValue> {
        let password = SecureVec::from_slice(&password.to_vec());

        // Get and decrypt the mnemonic seed phrase
        let payload = db::get_encrypted_mnemonic_seed()
            .await
            .map_err(|e| e.to_jsvalue())?
            .ok_or_else(|| JsValue::from_str("Mnemonic phrase not found"))?;
        let seed = decrypt(&password, payload)?;

        let index = Self::get_all_sphincs_lock_args().await?.len() as u32;
        let (pub_key, pri_key) = self
            .derive_sphincs_key(&seed, index)
            .map_err(|e| JsValue::from_str(&format!("Key derivation error: {}", e)))?;

        // Calculate lock script args and encrypt corresponding private key
        let lock_script_args = self.get_lock_scrip_arg(&pub_key);
        let encrypted_pri = encrypt(&password, &pri_key)?;

        // Store to DB
        let account = SphincsPlusAccount {
            index: 0, // Init to 0; Will be set correctly in add_account
            lock_args: encode(lock_script_args),
            pri_enc: encrypted_pri,
        };

        db::add_account(account).await.map_err(|e| e.to_jsvalue())?;

        Ok(encode(lock_script_args))
    }

    /// Imports a mnemonic by encrypting it with the provided password and storing it as the mnemonic phrase.
    ///
    /// **Parameters**:
    /// - `seed_phrase: Uint8Array` - The mnemonic phrase as a UTF-8 encoded Uint8Array to import.
    /// - `password: Uint8Array` - The password used to encrypt the mnemonic.
    ///
    /// **Returns**:
    /// - `Result<(), JsValue>` - A JavaScript Promise that resolves to `undefined` on success,
    ///   or rejects with a JavaScript error on failure.
    ///
    /// **Async**: Yes
    ///
    /// **Warning**: Handle the mnemonic in JavaScript side carefully.
    #[wasm_bindgen]
    pub async fn import_seed_phrase(
        &self,
        seed_phrase: Uint8Array,
        password: Uint8Array,
    ) -> Result<(), JsValue> {
        let password = SecureVec::from_slice(&password.to_vec());

        let seed_phrase_bytes = seed_phrase.to_vec();
        let seed_phrase_str = String::from_utf8(seed_phrase_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid UTF-8: {}", e)))?;

        let words: Vec<&str> = seed_phrase_str.split_whitespace().collect();
        let word_count = words.len();
        if word_count != 48 && word_count != 72 {
            return Err(JsValue::from_str("Mnemonic must have 48 or 72 words"));
        }

        let mut combined_entropy = Vec::new();
        for chunk in words.chunks(24) {
            let chunk_str = chunk.join(" ");
            let mnemonic = Mnemonic::parse_in(Language::English, &chunk_str)
                .map_err(|e| JsValue::from_str(&format!("Invalid mnemonic chunk: {}", e)))?;
            let entropy = mnemonic.to_entropy();
            combined_entropy.extend_from_slice(&entropy);
        }

        if combined_entropy.len() < self.variant.bip39_compatible_entropy_size() {
            return Err(JsValue::from(
                format!(
                    "Insufficient entropy: the input seed phrase got {} bytes, but at least {} bytes are required for the chosen SPHINCS+ parameter set {}.",
                    combined_entropy.len(),
                    self.variant.bip39_compatible_entropy_size(),
                    self.variant
                )
            ));
        }

        let encrypted_seed = encrypt(&password, &combined_entropy)?;
        db::set_encrypted_mnemonic_seed(encrypted_seed)
            .await
            .map_err(|e| e.to_jsvalue())?;
        Ok(())
    }

    /// Exports the mnemonic phrase by decrypting it with the provided password.
    ///
    /// **Parameters**:
    /// - `password: Uint8Array` - The password used to decrypt the mnemonic.
    ///
    /// **Returns**:
    /// - `Result<Uint8Array, JsValue>` - A JavaScript Promise that resolves to the mnemonic as a UTF-8 encoded `Uint8Array` on success,
    ///   or rejects with a JavaScript error on failure.
    ///
    /// **Async**: Yes
    ///
    /// **Warning**: Exporting the mnemonic exposes it in JavaScript, which may pose a security risk.
    /// Proper zeroization of exported seed phrase is the responsibility of the caller.
    #[wasm_bindgen]
    pub async fn export_seed_phrase(password: Uint8Array) -> Result<Uint8Array, JsValue> {
        let password = SecureVec::from_slice(&password.to_vec());
        let payload = db::get_encrypted_mnemonic_seed()
            .await
            .map_err(|e| e.to_jsvalue())?
            .ok_or_else(|| JsValue::from_str("Mnemonic phrase not found"))?;

        let entropy = decrypt(&password, payload)?;
        let chunks = entropy.chunks(32);
        let mut mnemonics = Vec::new();
        for chunk in chunks {
            let mnemonic = Mnemonic::from_entropy_in(Language::English, chunk).unwrap();
            mnemonics.push(mnemonic.to_string());
        }
        let combined_mnemonics = mnemonics.join(" ");

        Ok(Uint8Array::from(combined_mnemonics.as_ref()))
    }

    /// Signs a message using the SPHINCS+ private key after decrypting it with the provided password.
    ///
    /// **Parameters**:
    /// - `password: Uint8Array` - The password used to decrypt the private key.
    /// - `lock_args: String` - The hex-encoded lock script's arguments corresponding to the SPHINCS+ public key of the account that signs.
    /// - `message: Uint8Array` - The message to be signed.
    ///
    /// **Returns**:
    /// - `Result<Uint8Array, JsValue>` - The signature as a `Uint8Array` on success,
    ///   or a JavaScript error on failure.
    ///
    /// **Async**: Yes
    #[wasm_bindgen]
    pub async fn sign(
        &self,
        password: Uint8Array,
        lock_args: String,
        message: Uint8Array,
    ) -> Result<Uint8Array, JsValue> {
        let password = SecureVec::from_slice(&password.to_vec());
        let account = db::get_account(&lock_args)
            .await
            .map_err(|e| e.to_jsvalue())?
            .ok_or_else(|| JsValue::from_str("Account not found"))?;

        let pri_key = decrypt(&password, account.pri_enc)?;
        let message_vec = message.to_vec();

        match self.variant {
            SphincsVariant::Sha2128S => sphincs_sign!(slh_dsa_sha2_128s, pri_key, &message_vec, self.variant),
            SphincsVariant::Sha2128F => sphincs_sign!(slh_dsa_sha2_128f, pri_key, &message_vec, self.variant),
            SphincsVariant::Shake128S => sphincs_sign!(slh_dsa_shake_128s, pri_key, &message_vec, self.variant),
            SphincsVariant::Shake128F => sphincs_sign!(slh_dsa_shake_128f, pri_key, &message_vec, self.variant),
            SphincsVariant::Sha2192S => sphincs_sign!(slh_dsa_sha2_192s, pri_key, &message_vec, self.variant),
            SphincsVariant::Sha2192F => sphincs_sign!(slh_dsa_sha2_192f, pri_key, &message_vec, self.variant),
            SphincsVariant::Shake192S => sphincs_sign!(slh_dsa_shake_192s, pri_key, &message_vec, self.variant),
            SphincsVariant::Shake192F => sphincs_sign!(slh_dsa_shake_192f, pri_key, &message_vec, self.variant),
            SphincsVariant::Sha2256S => sphincs_sign!(slh_dsa_sha2_256s, pri_key, &message_vec, self.variant),
            SphincsVariant::Sha2256F => sphincs_sign!(slh_dsa_sha2_256f, pri_key, &message_vec, self.variant),
            SphincsVariant::Shake256S => sphincs_sign!(slh_dsa_shake_256s, pri_key, &message_vec, self.variant),
            SphincsVariant::Shake256F => sphincs_sign!(slh_dsa_shake_256f, pri_key, &message_vec, self.variant),
        }
    }

    /// Supporting wallet recovery - derives a list of lock script arguments (processed public keys) from the seed phrase starting from a given index.
    ///
    /// **Parameters**:
    /// - `password: Uint8Array` - The password used to decrypt the mnemonic.
    /// - `start_index: u32` - The starting index for derivation.
    /// - `count: u32` - The number of sequential lock scripts arguments to derive.
    ///
    /// **Returns**:
    /// - `Result<Vec<String>, JsValue>` - A list of lock script arguments on success,
    ///   or a JavaScript error on failure.
    #[wasm_bindgen]
    pub async fn try_gen_account_batch(
        &self,
        password: Uint8Array,
        start_index: u32,
        count: u32,
    ) -> Result<Vec<String>, JsValue> {
        let password = SecureVec::from_slice(&password.to_vec());
        // Get and decrypt the mnemonic seed phrase
        let payload = db::get_encrypted_mnemonic_seed()
            .await
            .map_err(|e| e.to_jsvalue())?
            .ok_or_else(|| JsValue::from_str("Mnemonic phrase not found"))?;
        let seed = decrypt(&password, payload)?;
        let mut lock_args_array: Vec<String> = Vec::new();
        for i in start_index..(start_index + count) {
            let (pub_key, _) = self
                .derive_sphincs_key(&seed, i)
                .map_err(|e| JsValue::from_str(&format!("Key derivation error: {}", e)))?;

            // Calculate lock script args
            let lock_script_args = self.get_lock_scrip_arg(&pub_key);
            lock_args_array.push(encode(lock_script_args));
        }
        Ok(lock_args_array)
    }

    /// Supporting wallet recovery - Recovers the wallet by deriving and storing private keys for the first N accounts.
    ///
    /// **Parameters**:
    /// - `password: Uint8Array` - The password used to decrypt the seed phrase.
    /// - `count: u32` - The number of accounts to recover (from index 0 to count-1).
    ///
    /// **Returns**:
    /// - `Result<(), JsValue>` - A list of newly generated sphincs+ lock script arguments (processed public keys) on success, or a JavaScript error on failure.
    ///
    /// **Async**: Yes
    #[wasm_bindgen]
    pub async fn recover_accounts(
        &self,
        password: Uint8Array,
        count: u32,
    ) -> Result<Vec<String>, JsValue> {
        let password = SecureVec::from_slice(&password.to_vec());
        // Get and decrypt the mnemonic seed phrase
        let payload = db::get_encrypted_mnemonic_seed()
            .await
            .map_err(|e| e.to_jsvalue())?
            .ok_or_else(|| JsValue::from_str("Mnemonic phrase not found"))?;
        let mut lock_args_array: Vec<String> = Vec::new();
        let seed = decrypt(&password, payload)?;
        for i in 0..count {
            let (pub_key, pri_key) = self
                .derive_sphincs_key(&seed, i)
                .map_err(|e| JsValue::from_str(&format!("Key derivation error: {}", e)))?;

            // Calculate lock script args and encrypt corresponding private key
            let lock_script_args = self.get_lock_scrip_arg(&pub_key);
            let encrypted_pri = encrypt(&password, &pri_key)?;
            // Store to DB
            let account = SphincsPlusAccount {
                index: 0, // Init to 0; Will be set correctly in add_account
                lock_args: encode(lock_script_args),
                pri_enc: encrypted_pri,
            };
            lock_args_array.push(encode(lock_script_args));

            db::add_account(account).await.map_err(|e| e.to_jsvalue())?;
        }
        Ok(lock_args_array)
    }

    /// Building CKB lockscript for SPHINCS+ public key
    ///
    /// **Parameters**:
    /// - `public_key: &SecureVec` - The SPHINCS+ public key to be used in the lock script.
    ///
    /// **Returns**:
    /// - `[u8; 32]` - The lock script arguments as a byte array.
    fn get_lock_scrip_arg(&self, public_key: &SecureVec) -> [u8; 32] {
        let all_in_one_config: [u8; 4] = [
            MULTISIG_RESERVED_FIELD_VALUE,
            REQUIRED_FIRST_N,
            THRESHOLD,
            PUBKEY_NUM,
        ];
        let sign_flag: u8 = self.variant << 1;
        let mut script_args_hasher = Hasher::script_args_hasher();
        script_args_hasher.update(&all_in_one_config);
        script_args_hasher.update(&[sign_flag]);
        script_args_hasher.update(&public_key);
        script_args_hasher.hash()
    }
}

////////////////////////////////////////////////////////////////////////////////
///  Key-vault utility functions
////////////////////////////////////////////////////////////////////////////////
#[wasm_bindgen]
pub struct Util;

#[wasm_bindgen]
impl Util {
    /// https://github.com/xxuejie/rfcs/blob/cighash-all/rfcs/0000-ckb-tx-message-all/0000-ckb-tx-message-all.md.
    ///
    /// **Parameters**:
    /// - `serialized_mock_tx: Uint8Array` - serialized CKB mock transaction.
    ///
    /// **Returns**:
    /// - `Result<Uint8Array, JsValue>` - The CKB transaction message all hash digest as a `Uint8Array` on success,
    ///   or a JavaScript error on failure.
    ///
    /// **Async**: no
    #[wasm_bindgen]
    pub fn get_ckb_tx_message_all(serialized_mock_tx: Uint8Array) -> Result<Uint8Array, JsValue> {
        let serialized_bytes = serialized_mock_tx.to_vec();
        let repr_mock_tx: ReprMockTransaction = serde_json::from_slice(&serialized_bytes)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;
        let mock_tx: MockTransaction = repr_mock_tx.into();
        let mut message_hasher = Hasher::message_hasher();
        let _ = generate_ckb_tx_message_all_from_mock_tx(
            &mock_tx,
            ScriptOrIndex::Index(0),
            &mut message_hasher,
        )
        .map_err(|e| JsValue::from_str(&format!("CKB_TX_MESSAGE_ALL error: {:?}", e)))?;
        let message = message_hasher.hash();
        Ok(Uint8Array::from(message.as_slice()))
    }

    /// Measure bit strength of a password
    ///
    /// **Parameters**:
    /// - `password: Uint8Array` - utf8 serialized password.
    ///
    /// **Returns**:
    /// - `Result<u16, JsValue>` - The strength of the password measured in bit on success,
    ///   or a JavaScript error on failure.
    ///
    /// **Async**: no
    #[wasm_bindgen]
    pub fn password_checker(password: Uint8Array) -> Result<u32, JsValue> {
        let password = SecureVec::from_slice(&password.to_vec());
        let password_str =
            std::str::from_utf8(&password).map_err(|e| JsValue::from_str(&e.to_string()))?;

        if password_str.is_empty() {
            return Ok(0);
        }

        let mut has_lowercase = false;
        let mut has_uppercase = false;
        let mut has_digit = false;
        let mut has_punctuation = false;
        let mut has_space = false;
        let mut has_other = false;

        for c in password_str.chars() {
            if c == ' ' {
                has_space = true;
            } else if c.is_ascii_lowercase() {
                has_lowercase = true;
            } else if c.is_ascii_uppercase() {
                has_uppercase = true;
            } else if c.is_ascii_digit() {
                has_digit = true;
            } else if c.is_ascii_punctuation() {
                has_punctuation = true;
            } else {
                has_other = true;
            }
        }

        if !has_uppercase {
            return Err(JsValue::from_str(
                "Password must contain at least one uppercase letter!",
            ));
        }
        if !has_lowercase {
            return Err(JsValue::from_str(
                "Password must contain at least one lowercase letter!",
            ));
        }
        if !has_digit {
            return Err(JsValue::from_str(
                "Password must contain at least one digit!",
            ));
        }
        if !has_punctuation {
            return Err(JsValue::from_str(
                "Password must contain at least one symbol!",
            ));
        }

        let character_set_size = if has_other {
            256
        } else {
            let mut size = 0;
            if has_lowercase {
                size += 26;
            } // a-z
            if has_uppercase {
                size += 26;
            } // A-Z
            if has_digit {
                size += 10;
            } // 0-9
            if has_punctuation {
                size += 32;
            } // ASCII punctuation
            if has_space {
                size += 1;
            } // Space character
            size
        };

        if character_set_size == 0 {
            return Ok(0);
        }

        let entropy = (password_str.len() as f64) * (character_set_size as f64).log2();
        let rounded_entropy = entropy.round() as u32;

        if rounded_entropy < 256 {
            return Err(JsValue::from_str(
                "Password entropy must be at least 256 bit. Consider lengthening your password!",
            ));
        }
        Ok(rounded_entropy)
    }
}
