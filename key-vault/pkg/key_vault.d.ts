/* tslint:disable */
/* eslint-disable */
export enum SphincsVariant {
  Sha2128F = 48,
  Sha2128S = 49,
  Sha2192F = 50,
  Sha2192S = 51,
  Sha2256F = 52,
  Sha2256S = 53,
  Shake128F = 54,
  Shake128S = 55,
  Shake192F = 56,
  Shake192S = 57,
  Shake256F = 58,
  Shake256S = 59,
}
export class KeyVault {
  free(): void;
  /**
   * Constructs a new `KeyVault` to serve as a namespace in the output js interface.
   *
   * **Returns**:
   * - `KeyVault` - A new instance of the struct.
   */
  constructor(variant: SphincsVariant);
  /**
   * Clears all data in the `seed_phrase_store` and `child_keys_store` in IndexedDB.
   *
   * **Returns**:
   * - `Result<(), JsValue>` - A JavaScript Promise that resolves to `undefined` on success,
   *   or rejects with a JavaScript error on failure.
   *
   * **Async**: Yes
   */
  static clear_database(): Promise<void>;
  /**
   * Retrieves all SPHINCS+ public keys from the database in the order they get inserted.
   *
   * **Returns**:
   * - `Result<Vec<String>, JsValue>` - A JavaScript Promise that resolves to an array of hex-encoded SPHINCS+ public keys on success,
   *   or rejects with a JavaScript error on failure.
   *
   * **Async**: Yes
   */
  static get_all_sphincs_pub(): Promise<string[]>;
  /**
   * Initializes the mnemonic phrase by generating a BIP39 mnemonic, encrypting it with the provided password, and storing it in IndexedDB.
   *
   * **Parameters**:
   * - `password: Uint8Array` - The password used to encrypt the mnemonic.
   *
   * **Returns**:
   * - `Result<(), JsValue>` - A JavaScript Promise that resolves to `undefined` on success,
   *   or rejects with a JavaScript error on failure.
   *
   * **Async**: Yes
   *
   * **Note**: Only effective when the mnemonic phrase is not yet set.
   */
  static init_seed_phrase(password: Uint8Array): Promise<void>;
  /**
   * Generates a new SPHINCS+ key pair - a SPHINCS+ child key pair derived from the mnemonic phrase,
   * encrypts the private key with the password, and stores/appends it in IndexedDB.
   *
   * **Parameters**:
   * - `password: Uint8Array` - The password used to decrypt the mnemonic phrase and encrypt the child private key.
   *
   * **Returns**:
   * - `Result<String, JsValue>` - A String Promise that resolves to the hex-encoded SPHINCS+ public key from the key pair on success,
   *   or rejects with a JavaScript error on failure.
   *
   * **Async**: Yes
   */
  gen_new_key_pair(password: Uint8Array): Promise<string>;
  /**
   * Imports a mnemonic by encrypting it with the provided password and storing it as the mnemonic phrase.
   *
   * **Parameters**:
   * - `seed_phrase: Uint8Array` - The mnemonic phrase as a UTF-8 encoded Uint8Array to import.
   * - `password: Uint8Array` - The password used to encrypt the mnemonic.
   *
   * **Returns**:
   * - `Result<(), JsValue>` - A JavaScript Promise that resolves to `undefined` on success,
   *   or rejects with a JavaScript error on failure.
   *
   * **Async**: Yes
   *
   * **Warning**: This method is not recommended as it may expose the mnemonic in JavaScript.
   */
  static import_seed_phrase(seed_phrase: Uint8Array, password: Uint8Array): Promise<void>;
  /**
   * Exports the mnemonic phrase by decrypting it with the provided password.
   *
   * **Parameters**:
   * - `password: Uint8Array` - The password used to decrypt the mnemonic.
   *
   * **Returns**:
   * - `Result<Uint8Array, JsValue>` - A JavaScript Promise that resolves to the mnemonic as a UTF-8 encoded `Uint8Array` on success,
   *   or rejects with a JavaScript error on failure.
   *
   * **Async**: Yes
   *
   * **Warning**: Exporting the mnemonic exposes it in JavaScript, which may pose a security risk.
   * Proper zeroization of exported seed phrase is the responsibility of the caller.
   */
  static export_seed_phrase(password: Uint8Array): Promise<Uint8Array>;
  /**
   * Signs a message using the SPHINCS+ private key after decrypting it with the provided password.
   *
   * **Parameters**:
   * - `password: Uint8Array` - The password used to decrypt the private key.
   * - `sphincs_plus_pub: String` - The SPHINCS+ public key identifying the private key to use for signing.
   * - `message: Uint8Array` - The message to be signed.
   *
   * **Returns**:
   * - `Result<Uint8Array, JsValue>` - The signature as a `Uint8Array` on success,
   *   or a JavaScript error on failure.
   *
   * **Async**: Yes
   */
  sign(password: Uint8Array, sphincs_plus_pub: string, message: Uint8Array): Promise<Uint8Array>;
  /**
   * Supporting wallet recovery - derives a list of public keys from the seed phrase starting from a given index.
   *
   * **Parameters**:
   * - `password: Uint8Array` - The password used to decrypt the mnemonic.
   * - `start_index: u32` - The starting index for derivation.
   * - `count: u32` - The number of public keys to derive.
   *
   * **Returns**:
   * - `Result<Vec<String>, JsValue>` - A list of public keys as strings on success,
   *   or a JavaScript error on failure.
   */
  gen_account_batch(password: Uint8Array, start_index: number, count: number): Promise<string[]>;
  /**
   * Supporting wallet recovery - Recovers the wallet by deriving and storing private keys for the first N accounts.
   *
   * **Parameters**:
   * - `password: Uint8Array` - The password used to decrypt the seed phrase.
   * - `count: u32` - The number of accounts to recover (from index 0 to count-1).
   *
   * **Returns**:
   * - `Result<(), JsValue>` - A list of newly generated sphincs+ public keys on success, or a JavaScript error on failure.
   *
   * **Async**: Yes
   */
  recover_accounts(password: Uint8Array, count: number): Promise<string[]>;
  sphincs_plus_variant: SphincsVariant;
}
export class Util {
  private constructor();
  free(): void;
  /**
   * https://github.com/xxuejie/rfcs/blob/cighash-all/rfcs/0000-ckb-tx-message-all/0000-ckb-tx-message-all.md.
   *
   * **Parameters**:
   * - `serialized_mock_tx: Uint8Array` - serialized CKB mock transaction.
   *
   * **Returns**:
   * - `Result<Uint8Array, JsValue>` - The CKB transaction message all hash digest as a `Uint8Array` on success,
   *   or a JavaScript error on failure.
   *
   * **Async**: no
   */
  static get_ckb_tx_message_all(serialized_mock_tx: Uint8Array): Uint8Array;
  /**
   * Measure bit strength of a password
   *
   * **Parameters**:
   * - `password: Uint8Array` - utf8 serialized password.
   *
   * **Returns**:
   * - `Result<u16, JsValue>` - The strength of the password measured in bit on success,
   *   or a JavaScript error on failure.
   *
   * **Async**: no
   */
  static password_checker(password: Uint8Array): number;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_keyvault_free: (a: number, b: number) => void;
  readonly __wbg_get_keyvault_sphincs_plus_variant: (a: number) => number;
  readonly __wbg_set_keyvault_sphincs_plus_variant: (a: number, b: number) => void;
  readonly __wbg_util_free: (a: number, b: number) => void;
  readonly util_get_ckb_tx_message_all: (a: any) => [number, number, number];
  readonly util_password_checker: (a: any) => [number, number, number];
  readonly keyvault_new: (a: number) => number;
  readonly keyvault_clear_database: () => any;
  readonly keyvault_get_all_sphincs_pub: () => any;
  readonly keyvault_init_seed_phrase: (a: any) => any;
  readonly keyvault_gen_new_key_pair: (a: number, b: any) => any;
  readonly keyvault_import_seed_phrase: (a: any, b: any) => any;
  readonly keyvault_export_seed_phrase: (a: any) => any;
  readonly keyvault_sign: (a: number, b: any, c: number, d: number, e: any) => any;
  readonly keyvault_gen_account_batch: (a: number, b: any, c: number, d: number) => any;
  readonly keyvault_recover_accounts: (a: number, b: any, c: number) => any;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_4: WebAssembly.Table;
  readonly __wbindgen_export_5: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly closure72_externref_shim_multivalue_shim: (a: number, b: number, c: any) => [number, number];
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h2b93dcc0cdfb007d: (a: number, b: number) => void;
  readonly closure40_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure48_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
