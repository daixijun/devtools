//! 加密/哈希/JWT 逻辑（与历史版本行为逐字节对齐）。
//!
//! ## API 契约（签名不可擅改）
//!
//! ```ignore
//! pub fn md5_hex(s: &str) -> String
//! pub fn sha1_hex(s: &str) -> String
//! pub fn sha224_hex(s: &str) -> String
//! pub fn sha256_hex(s: &str) -> String
//! pub fn sha384_hex(s: &str) -> String
//! pub fn sha512_hex(s: &str) -> String
//! pub fn sha3_hex(s: &str) -> String                       // Keccak-512（历史 "SHA3" 语义）
//! pub fn {md5,sha1,sha224,sha256,sha384,sha512,sha3}_bytes_hex(data: &[u8]) -> String
//!                                                          // 字节输入版（文件哈希）
//! pub fn pbkdf2_hex(password: &str, salt: &str, iterations: u32, dk_len: usize) -> String
//!
//! pub const AES_BLOCK_SIZE: usize = 16;
//! pub enum AesMode { Cbc, Ecb, Cfb, Ofb, Ctr }
//! pub enum AesPadding { Pkcs7, Iso97971, AnsiX923, Iso10126, Zero, NoPadding }
//! pub struct AesOptions { pub mode: AesMode, pub key_size: usize, pub padding: AesPadding }
//! pub fn aes_encrypt(text: &str, key: &str, iv: &str, opts: &AesOptions) -> Result<String, String>
//! pub fn aes_decrypt_to_bytes(b64: &str, key: &str, iv: &str, opts: &AesOptions) -> Result<Vec<u8>, String>
//! pub fn generate_random_iv() -> String                    // 16 个 base62 字符（约 95 位熵）
//!
//! pub fn sign_jwt(alg: &str, header: serde_json::Value, payload: serde_json::Value, key: &str) -> Result<String, String>
//! pub fn verify_jwt(token: &str, alg: &str, key: &str) -> Result<bool, String>
//! pub fn decode_jwt_segment(seg: &str) -> Result<String, String>
//!
//! pub fn bcrypt_hash(password: &str, rounds: u32) -> Result<String, String>
//! pub fn bcrypt_verify(password: &str, hash: &str) -> bool
//! ```
//!
//! 错误文案必须与历史版本一致。
//!
//! ## 与历史版本逐字节对齐的实现要点
//!
//! - 哈希/PBKDF2 输入按 UTF-8 编码（`&str::as_bytes`），输出小写 hex。
//! - `sha3_hex` 使用 **Keccak-512**（定稿前填充，非 FIPS-202 的 `Sha3_512`），
//!   与历史版本 "SHA3"（`keccak_512`）语义一致。
//! - AES 六种填充的 pad/unpad 语义与历史版本逐分支一致
//!   （含 ZeroPadding 已对齐不补、Iso97971 去尾零后去 0x80 不校验等旧行为）。
//! - 模式：ECB/CBC 手工填充后按块运算；CFB = 128 位全块反馈（cfb-mode crate）；CTR = 全
//!   128 位大端计数（ctr crate 的 `Ctr128BE`）；OFB 手写（block 从 IV 起逐块
//!   `block = E(block)` 生成密钥流后异或，加解密同函数。
//! - `parseKey`/`parseIv` 的错误文案与历史版本一致；IV 取前 16 字节；ECB 使用全零 IV。
//! - JWT：base64url 无 padding；RS* 为 RSASSA-PKCS1-v1_5（openssl，PKCS#1 v1.5 默认填充）；
//!   PEM 私钥兼容 PKCS#1（手工包裹为 PKCS#8）与 PKCS#8；PEM 公钥兼容 PKCS#1
//!   （手工包裹为 SPKI）与 SPKI，DER 长编码 `0x80 | lenBytes` 与历史版本一致。

// cipher::KeyInit/KeyIvInit 即 crypto_common 的同名 trait，同一份导入同时覆盖
// aes 分组密码与 HMAC 的 new_from_slice/new_from_slices。
use aes::cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit, KeyIvInit};
use aes::{Aes128, Aes192, Aes256};
use base64::engine::general_purpose;
use base64::Engine;
use cfb_mode::{Decryptor as CfbDecryptor, Encryptor as CfbEncryptor};
use ctr::cipher::StreamCipher;
use ctr::Ctr128BE;
use hmac::{Hmac, Mac};
use md5::Md5;
// Digest trait 统一经 sha2 的 re-export（RustCrypto 全家桶共用 digest::Digest）
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::{Signer, Verifier};
use serde_json::Value;
use sha1::Sha1;
use sha2::Digest as _;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::Keccak512;

// ---------------------------------------------------------------------------
// 哈希（输出小写 hex，与历史版本一致）
// - `*_hex`：UTF-8 文本输入（文本框场景）
// - `*_bytes_hex`：字节输入（文件哈希；PDF/DOCX 等二进制文件并非合法 UTF-8，
//   必须按字节计算，不得先转 String）
// ---------------------------------------------------------------------------

pub fn md5_hex(s: &str) -> String {
    md5_bytes_hex(s.as_bytes())
}

pub fn md5_bytes_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn sha1_hex(s: &str) -> String {
    sha1_bytes_hex(s.as_bytes())
}

pub fn sha1_bytes_hex(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn sha224_hex(s: &str) -> String {
    sha224_bytes_hex(s.as_bytes())
}

pub fn sha224_bytes_hex(data: &[u8]) -> String {
    let mut hasher = Sha224::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn sha256_hex(s: &str) -> String {
    sha256_bytes_hex(s.as_bytes())
}

pub fn sha256_bytes_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn sha384_hex(s: &str) -> String {
    sha384_bytes_hex(s.as_bytes())
}

pub fn sha384_bytes_hex(data: &[u8]) -> String {
    let mut hasher = Sha384::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn sha512_hex(s: &str) -> String {
    sha512_bytes_hex(s.as_bytes())
}

pub fn sha512_bytes_hex(data: &[u8]) -> String {
    let mut hasher = Sha512::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// 历史版本所称 "SHA3" 实为标准定稿前的 Keccak 填充（非 FIPS-202），
/// 使用 Keccak-512 保持与历史哈希值逐字节兼容。
pub fn sha3_hex(s: &str) -> String {
    sha3_bytes_hex(s.as_bytes())
}

pub fn sha3_bytes_hex(data: &[u8]) -> String {
    let mut hasher = Keccak512::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// 文件流式哈希：64KB 缓冲逐块读取（GB 级文件内存占用恒定，不整体载入）。
pub fn file_digest_hex(
    path: &std::path::Path,
    mut digest: impl sha2::Digest,
) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf)?;
        if n == 0 {
            break;
        }
        digest.update(&buf[..n]);
    }
    Ok(hex::encode(digest.finalize()))
}

/// 文件哈希便捷函数（内部统一走 [`file_digest_hex`] 流式读取）。
pub fn md5_file_hex(path: &std::path::Path) -> std::io::Result<String> {
    file_digest_hex(path, Md5::new())
}

pub fn sha1_file_hex(path: &std::path::Path) -> std::io::Result<String> {
    file_digest_hex(path, Sha1::new())
}

pub fn sha224_file_hex(path: &std::path::Path) -> std::io::Result<String> {
    file_digest_hex(path, Sha224::new())
}

pub fn sha256_file_hex(path: &std::path::Path) -> std::io::Result<String> {
    file_digest_hex(path, Sha256::new())
}

pub fn sha384_file_hex(path: &std::path::Path) -> std::io::Result<String> {
    file_digest_hex(path, Sha384::new())
}

pub fn sha512_file_hex(path: &std::path::Path) -> std::io::Result<String> {
    file_digest_hex(path, Sha512::new())
}

/// 历史 "SHA3"（Keccak-512）文件哈希。
pub fn sha3_file_hex(path: &std::path::Path) -> std::io::Result<String> {
    file_digest_hex(path, Keccak512::new())
}

/// PBKDF2（HMAC-SHA256），与历史版本默认配置一致。
pub fn pbkdf2_hex(password: &str, salt: &str, iterations: u32, dk_len: usize) -> String {
    let mut out = vec![0u8; dk_len];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), iterations, &mut out);
    hex::encode(out)
}

// ---------------------------------------------------------------------------
// AES：填充（逐分支对齐历史版本的 pad / unpad / 去尾零逻辑）
// ---------------------------------------------------------------------------

pub const AES_BLOCK_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AesMode {
    Cbc,
    Ecb,
    Cfb,
    Ofb,
    Ctr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AesPadding {
    Pkcs7,
    Iso97971,
    AnsiX923,
    Iso10126,
    Zero,
    NoPadding,
}

#[derive(Debug, Clone, Copy)]
pub struct AesOptions {
    pub mode: AesMode,
    pub key_size: usize,
    pub padding: AesPadding,
}

/// 随机字节来源（Iso10126 填充与随机 IV）。openssl 随码携带，无需额外依赖。
fn fill_random(buf: &mut [u8]) {
    openssl::rand::rand_bytes(buf).expect("OS 随机数源不可用");
}

/// 按指定策略填充数据到块大小的整数倍。
/// 注意：ZeroPadding 在数据已对齐时不再追加整块零（对齐历史版本）。
fn pad_data(mut data: Vec<u8>, padding: AesPadding) -> Vec<u8> {
    let n = AES_BLOCK_SIZE - (data.len() % AES_BLOCK_SIZE);
    match padding {
        // PKCS #5/7：追加 n 个值为 n 的字节
        AesPadding::Pkcs7 => {
            data.resize(data.len() + n, n as u8);
            data
        }
        // ISO/IEC 9797-1 Method 2：先加 0x80，再补零到块边界
        AesPadding::Iso97971 => {
            data.push(0x80);
            data.resize(data.len() + n - 1, 0);
            data
        }
        // ANSI X.9.23：补零，最后一个字节为填充长度
        AesPadding::AnsiX923 => {
            data.resize(data.len() + n - 1, 0);
            data.push(n as u8);
            data
        }
        // ISO 10126：补随机字节，最后一个字节为填充长度
        AesPadding::Iso10126 => {
            let mut random = vec![0u8; n];
            fill_random(&mut random);
            if n > 0 {
                random[n - 1] = n as u8;
            }
            data.extend_from_slice(&random);
            data
        }
        // 仅当未对齐时补零到块边界；已对齐（含空数据）不加填充
        AesPadding::Zero => {
            if !data.len().is_multiple_of(AES_BLOCK_SIZE) {
                let n = AES_BLOCK_SIZE - (data.len() % AES_BLOCK_SIZE);
                data.resize(data.len() + n, 0);
            }
            data
        }
        AesPadding::NoPadding => data,
    }
}

/// 移除尾零（ZeroPadding 解填充）。
/// 全为零时不裁剪，与历史版本行为一致。
fn strip_trailing_zeros(data: &[u8]) -> &[u8] {
    for i in (0..data.len()).rev() {
        if data[i] != 0 {
            return &data[..=i];
        }
    }
    data
}

/// 移除填充。校验失败时返回与历史版本一致的错误文案。
fn unpad_data(data: &[u8], padding: AesPadding) -> Result<Vec<u8>, String> {
    if padding == AesPadding::NoPadding {
        return Ok(data.to_vec());
    }

    if data.is_empty() {
        // 空密文对应空明文，交由调用方按空结果处理
        return Ok(Vec::new());
    }

    if !data.len().is_multiple_of(AES_BLOCK_SIZE) {
        return Err("解密失败：密文长度无效".to_string());
    }

    match padding {
        AesPadding::Pkcs7 => {
            let n = data[data.len() - 1] as usize;
            if !(1..=AES_BLOCK_SIZE).contains(&n) {
                return Err("解密失败：填充校验错误".to_string());
            }
            for &b in &data[data.len() - n..] {
                if b as usize != n {
                    return Err("解密失败：填充校验错误".to_string());
                }
            }
            Ok(data[..data.len() - n].to_vec())
        }
        // 先去尾零，再去掉 0x80（不额外校验）
        AesPadding::Iso97971 => {
            let stripped = strip_trailing_zeros(data);
            if stripped.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(stripped[..stripped.len() - 1].to_vec())
            }
        }
        // 末字节为填充长度
        AesPadding::AnsiX923 | AesPadding::Iso10126 => {
            let n = data[data.len() - 1] as usize;
            if !(1..=AES_BLOCK_SIZE).contains(&n) {
                return Err("解密失败：填充校验错误".to_string());
            }
            Ok(data[..data.len() - n].to_vec())
        }
        AesPadding::Zero => Ok(strip_trailing_zeros(data).to_vec()),
        AesPadding::NoPadding => unreachable!(),
    }
}

// ---------------------------------------------------------------------------
// AES：模式（对齐历史版本的分组模式实现）
// ---------------------------------------------------------------------------

/// 裸 AES 分组密码（三种密钥长度），用于手工实现 ECB/CBC/OFB。
enum AesCipher {
    A128(Aes128),
    A192(Aes192),
    A256(Aes256),
}

impl AesCipher {
    /// 调用前 key 长度已由 parse_key 校验（16/24/32 字节）。
    fn new(key: &[u8]) -> Self {
        match key.len() {
            16 => Self::A128(Aes128::new_from_slice(key).expect("密钥长度已校验")),
            24 => Self::A192(Aes192::new_from_slice(key).expect("密钥长度已校验")),
            32 => Self::A256(Aes256::new_from_slice(key).expect("密钥长度已校验")),
            _ => unreachable!("密钥长度已校验"),
        }
    }

    fn encrypt_block(&self, block: &mut [u8; AES_BLOCK_SIZE]) {
        match self {
            Self::A128(c) => BlockCipherEncrypt::encrypt_block(c, block.into()),
            Self::A192(c) => BlockCipherEncrypt::encrypt_block(c, block.into()),
            Self::A256(c) => BlockCipherEncrypt::encrypt_block(c, block.into()),
        }
    }

    fn decrypt_block(&self, block: &mut [u8; AES_BLOCK_SIZE]) {
        match self {
            Self::A128(c) => BlockCipherDecrypt::decrypt_block(c, block.into()),
            Self::A192(c) => BlockCipherDecrypt::decrypt_block(c, block.into()),
            Self::A256(c) => BlockCipherDecrypt::decrypt_block(c, block.into()),
        }
    }
}

/// ECB：逐块直接加解密（数据已填充对齐）。
fn ecb_apply(cipher: &AesCipher, data: &mut [u8], decrypt: bool) {
    for chunk in data.chunks_mut(AES_BLOCK_SIZE) {
        let block: &mut [u8; AES_BLOCK_SIZE] = chunk.try_into().expect("块长为 16");
        if decrypt {
            cipher.decrypt_block(block);
        } else {
            cipher.encrypt_block(block);
        }
    }
}

/// CBC：手工链接（数据已填充对齐）。
fn cbc_apply(cipher: &AesCipher, iv: &[u8; AES_BLOCK_SIZE], data: &mut [u8], decrypt: bool) {
    let mut prev = *iv;
    for chunk in data.chunks_mut(AES_BLOCK_SIZE) {
        let block: &mut [u8; AES_BLOCK_SIZE] = chunk.try_into().expect("块长为 16");
        if decrypt {
            let mut tmp = *block;
            cipher.decrypt_block(&mut tmp);
            for i in 0..AES_BLOCK_SIZE {
                tmp[i] ^= prev[i];
            }
            prev = *block;
            *block = tmp;
        } else {
            for i in 0..AES_BLOCK_SIZE {
                block[i] ^= prev[i];
            }
            cipher.encrypt_block(block);
            prev = *block;
        }
    }
}

/// OFB：用 ECB 加密递推的密钥流块
/// （block_i = E(block_{i-1})，起始为 IV），再与数据异或。加解密同函数。
fn ofb_apply(cipher: &AesCipher, iv: &[u8; AES_BLOCK_SIZE], data: &mut [u8]) {
    let mut block = *iv;
    for chunk in data.chunks_mut(AES_BLOCK_SIZE) {
        cipher.encrypt_block(&mut block);
        for (i, b) in chunk.iter_mut().enumerate() {
            *b ^= block[i];
        }
    }
}

/// CFB（128 位全块反馈，与历史版本语义一致）。
fn cfb_apply(key: &[u8], iv: &[u8; AES_BLOCK_SIZE], data: &mut [u8], decrypt: bool) {
    let expect = "密钥/IV 长度已校验";
    match key.len() {
        16 => {
            if decrypt {
                CfbDecryptor::<Aes128>::new_from_slices(key, iv)
                    .expect(expect)
                    .decrypt(data);
            } else {
                CfbEncryptor::<Aes128>::new_from_slices(key, iv)
                    .expect(expect)
                    .encrypt(data);
            }
        }
        24 => {
            if decrypt {
                CfbDecryptor::<Aes192>::new_from_slices(key, iv)
                    .expect(expect)
                    .decrypt(data);
            } else {
                CfbEncryptor::<Aes192>::new_from_slices(key, iv)
                    .expect(expect)
                    .encrypt(data);
            }
        }
        32 => {
            if decrypt {
                CfbDecryptor::<Aes256>::new_from_slices(key, iv)
                    .expect(expect)
                    .decrypt(data);
            } else {
                CfbEncryptor::<Aes256>::new_from_slices(key, iv)
                    .expect(expect)
                    .encrypt(data);
            }
        }
        _ => unreachable!("密钥长度已校验"),
    }
}

/// CTR（全 128 位大端计数）。加解密同函数。
fn ctr_apply(key: &[u8], iv: &[u8; AES_BLOCK_SIZE], data: &mut [u8]) {
    let expect = "密钥/IV 长度已校验";
    match key.len() {
        16 => Ctr128BE::<Aes128>::new_from_slices(key, iv)
            .expect(expect)
            .apply_keystream(data),
        24 => Ctr128BE::<Aes192>::new_from_slices(key, iv)
            .expect(expect)
            .apply_keystream(data),
        32 => Ctr128BE::<Aes256>::new_from_slices(key, iv)
            .expect(expect)
            .apply_keystream(data),
        _ => unreachable!("密钥长度已校验"),
    }
}

fn apply_mode(
    mode: AesMode,
    key: &[u8],
    iv: &[u8; AES_BLOCK_SIZE],
    data: &mut [u8],
    decrypt: bool,
) -> Result<(), String> {
    match mode {
        AesMode::Ecb => ecb_apply(&AesCipher::new(key), data, decrypt),
        AesMode::Cbc => cbc_apply(&AesCipher::new(key), iv, data, decrypt),
        AesMode::Ofb => ofb_apply(&AesCipher::new(key), iv, data),
        AesMode::Cfb => cfb_apply(key, iv, data, decrypt),
        AesMode::Ctr => ctr_apply(key, iv, data),
    }
    Ok(())
}

/// 解析密钥：按 UTF-8 编码，长度必须与所选密钥位数匹配。
fn parse_key(key: &str, key_size: usize) -> Result<Vec<u8>, String> {
    let expected = key_size / 8;
    let key = key.as_bytes();
    if key.len() != expected {
        return Err(format!("密钥长度必须为 {} 字符", expected));
    }
    Ok(key.to_vec())
}

/// 解析 IV：按 UTF-8 编码，使用前 16 字节（与历史版本行为一致）。
fn parse_iv(iv: &str) -> Result<[u8; AES_BLOCK_SIZE], String> {
    let iv = iv.as_bytes();
    if iv.len() < AES_BLOCK_SIZE {
        return Err("IV长度必须为16字符 (128位)".to_string());
    }
    let mut out = [0u8; AES_BLOCK_SIZE];
    out.copy_from_slice(&iv[..AES_BLOCK_SIZE]);
    Ok(out)
}

/// AES 加密，返回标准 Base64 密文（与历史版本输出一致）。
pub fn aes_encrypt(text: &str, key: &str, iv: &str, opts: &AesOptions) -> Result<String, String> {
    let key = parse_key(key, opts.key_size)?;
    let iv = if opts.mode == AesMode::Ecb {
        [0u8; AES_BLOCK_SIZE]
    } else {
        parse_iv(iv)?
    };
    let mut data = pad_data(text.as_bytes().to_vec(), opts.padding);
    // ECB/CBC 为块模式：NoPadding 时明文必须已对齐（与解密侧校验对称），
    // 否则 ecb/cbc_apply 的末块 try_into 会 panic
    if matches!(opts.mode, AesMode::Ecb | AesMode::Cbc)
        && opts.padding == AesPadding::NoPadding
        && !data.len().is_multiple_of(AES_BLOCK_SIZE)
    {
        return Err("加密失败：NoPadding 模式下明文长度必须为 16 字节的整数倍".to_string());
    }
    apply_mode(opts.mode, &key, &iv, &mut data, false)?;
    Ok(general_purpose::STANDARD.encode(&data))
}

/// AES 解密，返回原始字节（调用方自行按 UTF-8 解码，便于保留原有错误提示逻辑）。
pub fn aes_decrypt_to_bytes(
    base64_text: &str,
    key: &str,
    iv: &str,
    opts: &AesOptions,
) -> Result<Vec<u8>, String> {
    let key = parse_key(key, opts.key_size)?;
    let iv = if opts.mode == AesMode::Ecb {
        [0u8; AES_BLOCK_SIZE]
    } else {
        parse_iv(iv)?
    };
    let mut data = general_purpose::STANDARD
        .decode(base64_text)
        .map_err(|_| "密文不是有效的Base64格式".to_string())?;
    // ECB/CBC 为块模式：非对齐密文直接报错；统一给出与
    // unpad 一致的中文文案（流模式继续走 unpad 的长度校验）。
    if matches!(opts.mode, AesMode::Ecb | AesMode::Cbc)
        && !data.len().is_multiple_of(AES_BLOCK_SIZE)
    {
        return Err("解密失败：密文长度无效".to_string());
    }
    apply_mode(opts.mode, &key, &iv, &mut data, true)?;
    unpad_data(&data, opts.padding)
}

/// 生成随机 IV 文本：16 个 base62 字符（约 95 位熵）。
///
/// parse_iv 按字节取前 16 字符作 IV。历史版本生成器输出 32 字符 hex，
/// 实际仅前 16 字符（8 字节熵）参与运算，且与 OpenSSL 等工具的 32 字符
/// hex IV 互相误读；改为恰好 16 字符后生成值即完整 IV，与页面「16 字符」
/// 契约一致。解析语义未变，历史 32 字符 IV 的既有密文仍按原样解密。
pub fn generate_random_iv() -> String {
    const ALPHABET: &[u8; 62] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    // 248 = 62 * 4：拒绝区间消除模偏（每个字符恰有 4 个字节前像）
    const UNBIASED_MAX: u8 = 248;
    let mut out = String::with_capacity(AES_BLOCK_SIZE);
    while out.len() < AES_BLOCK_SIZE {
        let mut buf = [0u8; AES_BLOCK_SIZE];
        fill_random(&mut buf);
        for &b in &buf {
            if b < UNBIASED_MAX {
                out.push(ALPHABET[(b % 62) as usize] as char);
                if out.len() == AES_BLOCK_SIZE {
                    break;
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// JWT/JWS
// ---------------------------------------------------------------------------

/// JWT 三段解码结果（header/payload 为 pretty 化 JSON 文本，签名段保留原文）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedJwt {
    pub header: String,
    pub payload: String,
    pub signature: String,
    pub algorithm: String,
}

/// JWT 解码错误（结构化：静态文案由壳层经 i18n 翻译，动态详情随变体携带）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JwtDecodeError {
    /// 段数不为 3
    Malformed,
    /// base64url 段解码失败（携带完整错误文案）
    InvalidBase64(String),
    /// Header 不是有效 JSON（携带 serde 错误详情）
    InvalidHeaderJson(String),
    /// Payload 不是有效 JSON（携带 serde 错误详情）
    InvalidPayloadJson(String),
}

/// 解码 JWT 的三段结构：base64url 解码 → JSON 校验 → pretty 化（2 空格缩进）。
/// alg 缺失时 `algorithm` 为 "Unknown"；签名段不解码、保留原文。
pub fn decode_jwt(token: &str) -> Result<DecodedJwt, JwtDecodeError> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Err(JwtDecodeError::Malformed);
    }
    let header_bytes = base64_url_to_bytes(parts[0]).map_err(JwtDecodeError::InvalidBase64)?;
    let header_raw = String::from_utf8_lossy(&header_bytes).into_owned();
    let header: Value = serde_json::from_str(&header_raw)
        .map_err(|e| JwtDecodeError::InvalidHeaderJson(e.to_string()))?;
    let payload_bytes = base64_url_to_bytes(parts[1]).map_err(JwtDecodeError::InvalidBase64)?;
    let payload_raw = String::from_utf8_lossy(&payload_bytes).into_owned();
    let payload: Value = serde_json::from_str(&payload_raw)
        .map_err(|e| JwtDecodeError::InvalidPayloadJson(e.to_string()))?;
    let algorithm = header
        .get("alg")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .to_string();
    Ok(DecodedJwt {
        header: serde_json::to_string_pretty(&header).unwrap_or(header_raw),
        payload: serde_json::to_string_pretty(&payload).unwrap_or(payload_raw),
        signature: parts[2].to_string(),
        algorithm,
    })
}

/// 校验 PEM 外壳：`-----BEGIN [RSA ]<label>-----` / `-----END ...-----` 标记
/// 存在、正文非空且仅含 base64 字符与空白。
/// 只做外壳校验（不解析密钥内容）；`label` 如 "PUBLIC KEY" / "PRIVATE KEY"。
pub fn validate_pem(pem: &str, label: &str) -> bool {
    let begin = |rsa: bool| {
        if rsa {
            format!("-----BEGIN RSA {label}-----")
        } else {
            format!("-----BEGIN {label}-----")
        }
    };
    let end = |rsa: bool| {
        if rsa {
            format!("-----END RSA {label}-----")
        } else {
            format!("-----END {label}-----")
        }
    };
    let (begin_tok, end_tok) = if pem.contains(&begin(true)) {
        (begin(true), end(true))
    } else {
        (begin(false), end(false))
    };
    if !pem.contains(begin_tok.as_str()) || !pem.contains(end_tok.as_str()) {
        return false;
    }
    let body = pem
        .replacen(begin_tok.as_str(), "", 1)
        .replacen(end_tok.as_str(), "", 1);
    let content = body.trim();
    if content.is_empty() {
        return false;
    }
    content
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=') || c.is_whitespace())
}

/// alg -> openssl 摘要（HS/RS × 256/384/512）。
fn alg_message_digest(alg: &str) -> Option<MessageDigest> {
    match alg {
        "HS256" | "RS256" => Some(MessageDigest::sha256()),
        "HS384" | "RS384" => Some(MessageDigest::sha384()),
        "HS512" | "RS512" => Some(MessageDigest::sha512()),
        _ => None,
    }
}

// ---------- base64 / base64url ----------

fn bytes_to_base64_url(bytes: &[u8]) -> String {
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Base64URL 解码（历史版本语义）：`-`→`+`、`_`→`/`，再按原长度补 `=` 到 4 的倍数，
/// 最后按标准字母表解码。
fn base64_url_to_bytes(text: &str) -> Result<Vec<u8>, String> {
    let base64 = text.replace('-', "+").replace('_', "/");
    let padded_len = base64.len().div_ceil(4) * 4;
    let mut padded = base64;
    while padded.len() < padded_len {
        padded.push('=');
    }
    general_purpose::STANDARD
        .decode(padded.as_bytes())
        .map_err(|e| format!("无效的base64url编码: {}", e))
}

/// 解码 JWT 的 base64url 段为 UTF-8 文本（header/payload 均为 JSON）。
/// 解码采用替换语义（与历史版本一致）：非法 UTF-8 序列替换为 U+FFFD 而非报错。
pub fn decode_jwt_segment(seg: &str) -> Result<String, String> {
    let bytes = base64_url_to_bytes(seg)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

// ---------- DER / PEM ----------

const RSA_OID: [u8; 11] = [
    0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01,
];
const NULL_PARAMS: [u8; 2] = [0x05, 0x00];

fn der_concat(parts: &[&[u8]]) -> Vec<u8> {
    let total: usize = parts.iter().map(|p| p.len()).sum();
    let mut out = Vec::with_capacity(total);
    for part in parts {
        out.extend_from_slice(part);
    }
    out
}

/// 构造单个 DER TLV 结构（长编码 `0x80 | lenBytes`）。
fn der_wrap(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    if content.len() < 128 {
        out.push(content.len() as u8);
    } else {
        let mut length_bytes = Vec::new();
        let mut len = content.len();
        while len > 0 {
            length_bytes.insert(0, (len & 0xff) as u8);
            len >>= 8;
        }
        out.push(0x80 | length_bytes.len() as u8);
        out.extend_from_slice(&length_bytes);
    }
    out.extend_from_slice(content);
    out
}

/// 移除首个 PEM 标记（等价于 JS 的 `replace(/-----BEGIN [^-]+-----/, '')`）：
/// 标记正文必须为一段不含 `-` 的非空文本，其后紧跟 `-----`。
fn remove_pem_marker(s: &str, head: &str) -> String {
    let Some(start) = s.find(head) else {
        return s.to_string();
    };
    let after = start + head.len();
    let bytes = s.as_bytes();
    let mut i = after;
    while i < bytes.len() && bytes[i] != b'-' {
        i += 1;
    }
    if i == after || i + 5 > bytes.len() || &bytes[i..i + 5] != b"-----" {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..start]);
    out.push_str(&s[i + 5..]);
    out
}

/// 提取 PEM 中 base64 正文并解码为 DER 字节。
fn pem_body_to_der(pem: &str) -> Result<Vec<u8>, String> {
    let body = remove_pem_marker(pem, "-----BEGIN ");
    let body = remove_pem_marker(&body, "-----END ");
    // JS 的 /\s+/g 替换全部空白
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    general_purpose::STANDARD
        .decode(compact.as_bytes())
        .map_err(|e| format!("PEM Base64 解码失败: {}", e))
}

/// PKCS#1 RSAPrivateKey → PKCS#8 PrivateKeyInfo
/// PKCS#8 = SEQUENCE { version 0, AlgorithmIdentifier, OCTET STRING { PKCS#1 } }
fn pkcs1_private_key_to_pkcs8(pkcs1: &[u8]) -> Vec<u8> {
    der_wrap(
        0x30,
        &der_concat(&[
            &[0x02, 0x01, 0x00],
            &der_wrap(0x30, &der_concat(&[&RSA_OID, &NULL_PARAMS])),
            &der_wrap(0x04, pkcs1),
        ]),
    )
}

/// PKCS#1 RSAPublicKey → SPKI SubjectPublicKeyInfo
/// SPKI = SEQUENCE { AlgorithmIdentifier, BIT STRING { 0x00, PKCS#1 } }
fn pkcs1_public_key_to_spki(pkcs1: &[u8]) -> Vec<u8> {
    der_wrap(
        0x30,
        &der_concat(&[
            &der_wrap(0x30, &der_concat(&[&RSA_OID, &NULL_PARAMS])),
            &der_wrap(0x03, &der_concat(&[&[0x00], pkcs1])),
        ]),
    )
}

/// 导入 PEM 私钥（兼容 PKCS#1 与 PKCS#8）。
/// 标记缺失 / Base64 正文非法 / DER 解析失败均返回精确文案「无法识别的私钥格式」。
fn import_rsa_private_key(pem: &str) -> Result<PKey<openssl::pkey::Private>, String> {
    if !pem.contains("-----BEGIN RSA PRIVATE KEY-----")
        && !pem.contains("-----BEGIN PRIVATE KEY-----")
    {
        return Err("无法识别的私钥格式".to_string());
    }
    let der = pem_body_to_der(pem).map_err(|_| "无法识别的私钥格式".to_string())?;
    let der = if pem.contains("-----BEGIN RSA PRIVATE KEY-----") {
        pkcs1_private_key_to_pkcs8(&der)
    } else {
        der
    };
    PKey::private_key_from_pkcs8(&der).map_err(|_| "无法识别的私钥格式".to_string())
}

/// 导入 PEM 公钥（兼容 PKCS#1 与 SPKI）。
/// 标记缺失 / Base64 正文非法 / DER 解析失败均返回精确文案「无法识别的公钥格式」。
fn import_rsa_public_key(pem: &str) -> Result<PKey<openssl::pkey::Public>, String> {
    if !pem.contains("-----BEGIN RSA PUBLIC KEY-----")
        && !pem.contains("-----BEGIN PUBLIC KEY-----")
    {
        return Err("无法识别的公钥格式".to_string());
    }
    let der = pem_body_to_der(pem).map_err(|_| "无法识别的公钥格式".to_string())?;
    let der = if pem.contains("-----BEGIN RSA PUBLIC KEY-----") {
        pkcs1_public_key_to_spki(&der)
    } else {
        der
    };
    PKey::public_key_from_der(&der).map_err(|_| "无法识别的公钥格式".to_string())
}

/// HMAC（HS256/HS384/HS512），key 为 UTF-8 编码的 Secret Key。
fn hmac_sign(alg: &str, key: &[u8], data: &[u8]) -> Option<Vec<u8>> {
    fn finish<D: Mac>(mut mac: D, data: &[u8]) -> Vec<u8> {
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }
    match alg {
        "HS256" => Some(finish(
            <Hmac<Sha256> as KeyInit>::new_from_slice(key).expect("HMAC 接受任意长度密钥"),
            data,
        )),
        "HS384" => Some(finish(
            <Hmac<Sha384> as KeyInit>::new_from_slice(key).expect("HMAC 接受任意长度密钥"),
            data,
        )),
        "HS512" => Some(finish(
            <Hmac<Sha512> as KeyInit>::new_from_slice(key).expect("HMAC 接受任意长度密钥"),
            data,
        )),
        _ => None,
    }
}

/// 生成 JWS 签名的 JWT 令牌。
/// - `alg`：HS256/HS384/HS512/RS256/RS384/RS512
/// - HS* 传 Secret Key 字符串；RS* 传 PEM 私钥文本
pub fn sign_jwt(alg: &str, header: Value, payload: Value, key: &str) -> Result<String, String> {
    let digest = alg_message_digest(alg).ok_or_else(|| format!("不支持的签名算法：{}", alg))?;

    let header_json =
        serde_json::to_string(&header).map_err(|e| format!("JSON序列化失败: {}", e))?;
    let payload_json =
        serde_json::to_string(&payload).map_err(|e| format!("JSON序列化失败: {}", e))?;
    let signing_input = format!(
        "{}.{}",
        bytes_to_base64_url(header_json.as_bytes()),
        bytes_to_base64_url(payload_json.as_bytes())
    );

    let signature = if alg.starts_with("HS") {
        hmac_sign(alg, key.as_bytes(), signing_input.as_bytes())
            .ok_or_else(|| format!("不支持的签名算法：{}", alg))?
    } else {
        let rsa_key = import_rsa_private_key(key)?;
        let mut signer =
            Signer::new(digest, &rsa_key).map_err(|e| format!("RSA签名初始化失败: {}", e))?;
        signer
            .update(signing_input.as_bytes())
            .map_err(|e| format!("RSA签名计算失败: {}", e))?;
        signer
            .sign_to_vec()
            .map_err(|e| format!("RSA签名计算失败: {}", e))?
    };

    Ok(format!(
        "{}.{}",
        signing_input,
        bytes_to_base64_url(&signature)
    ))
}

/// 验证 JWS 签名的 JWT 令牌。
/// - `token`：三段式 JWT 令牌；`alg`：令牌头部声明的算法
/// - HS* 传 Secret Key 字符串；RS* 传 PEM 公钥文本
/// - 返回签名是否有效；签名段解码失败（含长度非法）一律视为验签失败返回
///   `Ok(false)`；密钥无法解析时向上返回 Err（由调用方决定提示文案）。
pub fn verify_jwt(token: &str, alg: &str, key: &str) -> Result<bool, String> {
    let digest = alg_message_digest(alg).ok_or_else(|| format!("不支持的签名算法：{}", alg))?;

    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Ok(false);
    }

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    // 签名段解码失败（非法 base64url 字符等）按验签失败处理，不向上抛 Err
    let signature = match base64_url_to_bytes(parts[2]) {
        Ok(sig) => sig,
        Err(_) => return Ok(false),
    };

    if alg.starts_with("HS") {
        let expected = hmac_sign(alg, key.as_bytes(), signing_input.as_bytes())
            .ok_or_else(|| format!("不支持的签名算法：{}", alg))?;
        // openssl::memcmp::eq 要求等长输入（不等长会 assert! panic），
        // 长度不等（空签名 / 截断签名等）先按验签失败处理，再做常量时间比较
        if expected.len() != signature.len() {
            return Ok(false);
        }
        Ok(openssl::memcmp::eq(&expected, &signature))
    } else {
        // 空签名 / 长度与模长不符的签名由 RSA_verify 判 false；空签名提前拦截，
        // 保证「长度非法一律 Ok(false)」不依赖 openssl 对边界输入的行为
        if signature.is_empty() {
            return Ok(false);
        }
        let rsa_key = import_rsa_public_key(key)?;
        let mut verifier =
            Verifier::new(digest, &rsa_key).map_err(|e| format!("RSA公钥解析失败: {}", e))?;
        verifier
            .update(signing_input.as_bytes())
            .map_err(|e| format!("RSA签名验证失败: {}", e))?;
        // RSA_verify 对长度非法的签名返回 false，不会向上抛错
        verifier
            .verify(&signature)
            .map_err(|e| format!("RSA签名验证失败: {}", e))
    }
}

// ---------------------------------------------------------------------------
// 密码哈希（bcrypt；PBKDF2 见 pbkdf2_hex）
// ---------------------------------------------------------------------------

/// bcrypt 哈希（cost 范围由调用方 UI 校验，core 直接透传给 bcrypt crate）。
pub fn bcrypt_hash(password: &str, rounds: u32) -> Result<String, String> {
    bcrypt::hash(password, rounds).map_err(|e| format!("bcrypt哈希计算失败: {}", e))
}

/// 验证侧允许的最大 cost。cost 每加 1 耗时翻倍（20 在现代机器约数十秒级），
/// 超过该值的哈希直接按验证失败处理——验证页的 cost 取自粘贴的哈希本身，
/// 不设上限时粘贴 `$2b$31$…` 会让后台 worker 满载数十分钟以上
///（加密侧由页面限制 4-20）。
const BCRYPT_VERIFY_MAX_COST: u32 = 20;

/// 解析 bcrypt 哈希内嵌的 cost（`$2b$NN$`）。
fn bcrypt_hash_cost(hash: &str) -> Option<u32> {
    hash.split('$').nth(2)?.parse().ok()
}

/// bcrypt 验证；哈希格式非法或 cost 超限（见 [`BCRYPT_VERIFY_MAX_COST`]）时
/// 按验证失败处理。
pub fn bcrypt_verify(password: &str, hash: &str) -> bool {
    if let Some(cost) = bcrypt_hash_cost(hash) {
        if cost > BCRYPT_VERIFY_MAX_COST {
            return false;
        }
    }
    bcrypt::verify(password, hash).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 黄金向量：对历史实现求值生成；
    /// 哈希另与 RFC 1321 / RFC 6234 / RFC 7914 标准向量交叉核对。
    const UI_TEXT: &str = "这是一段需要加密的示例文本";
    const UI_KEY: &str = "my-secret-key-16";
    const UI_IV: &str = "initialization-ve"; // 17 字符，取前 16 字节

    fn opts(mode: AesMode, key_size: usize, padding: AesPadding) -> AesOptions {
        AesOptions {
            mode,
            key_size,
            padding,
        }
    }

    // ---------- 哈希 ----------

    #[test]
    fn md5_rfc1321_vectors() {
        assert_eq!(md5_hex(""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5_hex("abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            md5_hex("message digest"),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            md5_hex("The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
        assert_eq!(
            md5_hex("Hello, World! 你好，世界！"),
            "df0149e54b132c1522de16859d759603"
        );
    }

    #[test]
    fn sha_family_rfc6234_vectors() {
        // RFC 6234 "abc" 向量 + 黄金向量（含空串与 UTF-8 中文）
        assert_eq!(sha1_hex(""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(sha1_hex("abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(
            sha224_hex("abc"),
            "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7"
        );
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha384_hex("abc"),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
        );
        assert_eq!(
            sha512_hex("abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            sha224_hex(""),
            "d14a028c2a3a2bc9476102bb288234c415a2b01f828ea62ac5b3e42f"
        );
        assert_eq!(
            sha384_hex("The quick brown fox jumps over the lazy dog"),
            "ca737f1014a48f4c0b6dd43cb177b0afd9e5169367544c494011e3317dbf9a509cb1e5dc1e85a941bbee3d7f2afbc9b1"
        );
        assert_eq!(
            sha512_hex("Hello, World! 你好，世界！"),
            "df44474eb62ed1ac6f8ce798c20a2308d73d368e7ab68bcb78461e7afb2c0b53715f6ca1dd4008696ac4651f572aaf08e3d7ebfd832a2ac3489a438a651ad32d"
        );
    }

    #[test]
    fn sha3_is_keccak512_not_fips202() {
        // Keccak-512 已知向量；FIPS-202 的 SHA3-512("") 是 a69f73cc…，二者必须不同
        assert_eq!(
            sha3_hex(""),
            "0eab42de4c3ceb9235fc91acffe746b29c29a8c366b7c60e4e67c466f36a4304c00fa9caf9d87976ba469bcbe06713b435f091ef2769fb160cdab33d3670680e"
        );
        assert_eq!(
            sha3_hex("abc"),
            "18587dc2ea106b9a1563e32b3312421ca164c7f1f07bc922a9c83d77cea3a1e5d0c69910739025372dc14ac9642629379540c17e2a65b19d77aa511a9d00bb96"
        );
        assert_ne!(sha3_hex(""), sha256_hex(""));
    }

    #[test]
    fn bytes_hex_matches_text_hex() {
        // 字节版与文本版等价（`*_hex` 委托 `*_bytes_hex`）
        for text in ["", "abc", "Hello, World! 你好，世界！"] {
            assert_eq!(md5_bytes_hex(text.as_bytes()), md5_hex(text));
            assert_eq!(sha1_bytes_hex(text.as_bytes()), sha1_hex(text));
            assert_eq!(sha224_bytes_hex(text.as_bytes()), sha224_hex(text));
            assert_eq!(sha256_bytes_hex(text.as_bytes()), sha256_hex(text));
            assert_eq!(sha384_bytes_hex(text.as_bytes()), sha384_hex(text));
            assert_eq!(sha512_bytes_hex(text.as_bytes()), sha512_hex(text));
            assert_eq!(sha3_bytes_hex(text.as_bytes()), sha3_hex(text));
        }
        // 非法 UTF-8 场景的可靠性：RFC 1321 / FIPS 180-2 标准向量按字节输入交叉验证
        assert_eq!(md5_bytes_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(
            sha256_bytes_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // 单个高位字节（SHA-256(b"\x80")）
        assert_eq!(
            sha256_bytes_hex(&[0x80]),
            "76be8b528d0075f7aae98d6fa57a6d3c83ae480a8469e668d7b0af968995ac71"
        );
        // 含高位字节（PDF/DOCX 二进制特征）可哈希且输出长度正确
        let bin = [0x25u8, 0x50, 0x44, 0x46, 0x80, 0xff];
        assert_eq!(md5_bytes_hex(&bin).len(), 32);
        assert_eq!(sha256_bytes_hex(&bin).len(), 64);
        assert_eq!(sha3_bytes_hex(&bin).len(), 128);
    }

    #[test]
    fn file_hash_path_handles_binary() {
        // 复现 UI 文件哈希路径（file_digest_hex 流式读取 → *_file_hex）：
        // 二进制文件（非法 UTF-8）不再触发 read_to_string 的
        // "stream did not contain valid UTF-8" 错误，且与全量字节哈希一致
        let mut path = std::env::temp_dir();
        path.push(format!("devtools-bin-hash-{}.bin", std::process::id()));
        let data = [0x25u8, 0x50, 0x44, 0x46, 0x2d, 0x80, 0xff, 0x00];
        std::fs::write(&path, data).unwrap();

        assert_eq!(
            md5_file_hex(&path).unwrap(),
            md5_bytes_hex(&data),
            "流式 MD5 应与字节版一致"
        );
        assert_eq!(
            sha256_file_hex(&path).unwrap(),
            sha256_bytes_hex(&data),
            "流式 SHA-256 应与字节版一致"
        );
        assert_eq!(sha1_file_hex(&path).unwrap().len(), 40);
        assert_eq!(sha224_file_hex(&path).unwrap().len(), 56);
        assert_eq!(sha384_file_hex(&path).unwrap().len(), 96);
        assert_eq!(sha512_file_hex(&path).unwrap().len(), 128);
        assert_eq!(sha3_file_hex(&path).unwrap().len(), 128);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn pbkdf2_hmac_sha256_vectors() {
        // RFC 7914 §11.4（PBKDF2-HMAC-SHA-256，P="passwd" S="salt"）
        assert_eq!(
            pbkdf2_hex("password", "salt", 1, 32),
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );
        assert_eq!(
            pbkdf2_hex("password", "salt", 2, 32),
            "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"
        );
        assert_eq!(
            pbkdf2_hex("password", "salt", 4096, 32),
            "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a"
        );
        // dkLen=64 黄金向量 + UTF-8 密码/盐值黄金向量（PasswordHasher 用法）
        assert_eq!(
            pbkdf2_hex("password", "salt", 4096, 64),
            "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134af7ad98c1b458ce3fd74ca35beba3cda7b8d1038d6a87071b918f837405f3fe77"
        );
        assert_eq!(
            pbkdf2_hex("密码", "盐值", 1000, 32),
            "863277d63a112fd47720548ee8171073237ddaab1bce55ded4c34b71c83c6fa2"
        );
    }

    // ---------- AES 黄金向量（与历史版本逐字节一致） ----------

    #[test]
    fn aes_golden_ui_example_all_modes() {
        let cases = [
            (
                AesMode::Cbc,
                "BPEe/syDXFqSVThRDUEVJNwIuXI1azq4YRMV8EITZ3rFjqz0eahZmzszOorOTzVV",
            ),
            (
                AesMode::Ecb,
                "V/AMYXBsC3hK/nhFNjsv9exYRxsLKhxVxx5oYtlQ04ELVxtV/jZSfbNwT4Xt9eWS",
            ),
            (
                AesMode::Cfb,
                "T7F4RLk7LJpoSi0WnEAX3bMNny9qsTeY71NYs/0J+WcwNfkLmNmSDiC4J8E1iupc",
            ),
            (
                AesMode::Ofb,
                "T7F4RLk7LJpoSi0WnEAX3RPgtqWpToYqms1grW10rCl92/w4J6JPGjkWQGFeeZ7X",
            ),
            (
                AesMode::Ctr,
                "T7F4RLk7LJpoSi0WnEAX3QA+9G+iyFM2lW8VkpeK8x7TngmofAYHwiyVke9GJvhk",
            ),
        ];
        for (mode, expected) in cases {
            let out =
                aes_encrypt(UI_TEXT, UI_KEY, UI_IV, &opts(mode, 128, AesPadding::Pkcs7)).unwrap();
            assert_eq!(out, expected, "mode {:?}", mode);
            // 解密还原
            let plain =
                aes_decrypt_to_bytes(&out, UI_KEY, UI_IV, &opts(mode, 128, AesPadding::Pkcs7))
                    .unwrap();
            assert_eq!(String::from_utf8(plain).unwrap(), UI_TEXT);
        }
    }

    #[test]
    fn aes_golden_key_sizes_and_paddings() {
        type GoldenCase<'a> = (
            &'a str,
            &'a str,
            &'a str,
            &'a str,
            AesMode,
            usize,
            AesPadding,
            &'a str,
        );
        let cases: &[GoldenCase] = &[
            ("aes|cbc128_zero_ui", UI_TEXT, UI_KEY, UI_IV, AesMode::Cbc, 128, AesPadding::Zero,
             "BPEe/syDXFqSVThRDUEVJNwIuXI1azq4YRMV8EITZ3qHroZMOBVXIEvq4M3mA+1J"),
            ("aes|cbc192_zero", "The quick brown fox jumps over the lazy dog",
             "012345678901234567890123", "0000000000000000", AesMode::Cbc, 192, AesPadding::Zero,
             "3FPY0nEqpFFvToacP1a8f1ktQVScIVCkkwIP7wbCEGlpA65eC2CtpPDa94m2bQoL"),
            ("aes|ecb192_zero", "The quick brown fox jumps over the lazy dog",
             "012345678901234567890123", "", AesMode::Ecb, 192, AesPadding::Zero,
             "Sw8re4NdaflEwqr+jfkYyhymOYjkmiC0U3Eb4G/q9gKZ/P+mCCMAw9X2XTKSjnFX"),
            ("aes|cbc256_pkcs7", "Attack at dawn! Attack at dawn!",
             "01234567890123456789012345678901", "11112222333344445555666677778888",
             AesMode::Cbc, 256, AesPadding::Pkcs7, "rZDDjNG4ZKuinpv9wy4Twbvk3wlTAqcjbcgvOPvmJL8="),
            ("aes|ecb256_pkcs7", "Attack at dawn! Attack at dawn!",
             "01234567890123456789012345678901", "", AesMode::Ecb, 256, AesPadding::Pkcs7,
             "wW6GabdnFfe/v8FSviGU4MIfYqXJ2+wKxqXCphiXl10="),
            ("aes|cbc128_nopad_aligned", "0123456789ABCDEF", UI_KEY, UI_IV,
             AesMode::Cbc, 128, AesPadding::NoPadding, "gp1ZAWh8wcU5Nrcvp70Vtw=="),
            ("aes|cbc128_ansix923", UI_TEXT, UI_KEY, UI_IV, AesMode::Cbc, 128, AesPadding::AnsiX923,
             "BPEe/syDXFqSVThRDUEVJNwIuXI1azq4YRMV8EITZ3olnmIxfOb7KPhkbytqAujA"),
            ("aes|cbc128_iso97971", UI_TEXT, UI_KEY, UI_IV, AesMode::Cbc, 128, AesPadding::Iso97971,
             "BPEe/syDXFqSVThRDUEVJNwIuXI1azq4YRMV8EITZ3pKBbcdJbrTfmwS2p5Dfl6k"),
            ("aes|ofb256_long", "012345678901234567890123456789012345678901234567890123456789",
             "01234567890123456789012345678901", "0123456789abcdef", AesMode::Ofb, 256,
             AesPadding::Zero,
             "v0iAj+2yQ31UxT8hTj4EqhMKiNegfymaEjuwQs5p35m7QFJ5oDFcZ/LudS758brn1prPEQsWk8NGgLvFE48RKw=="),
            ("aes|ctr192_empty_pkcs7", "", "012345678901234567890123", "9876543210fedcba",
             AesMode::Ctr, 192, AesPadding::Pkcs7, "D85VSTWL3OEWa5Namc6ekA=="),
        ];
        for (label, text, key, iv, mode, key_size, padding, expected) in cases {
            let out = aes_encrypt(text, key, iv, &opts(*mode, *key_size, *padding))
                .unwrap_or_else(|e| panic!("{}: {}", label, e));
            assert_eq!(out, *expected, "{}", label);
        }
    }

    #[test]
    fn aes_roundtrip_all_mode_keysize_padding_combos() {
        let modes = [
            AesMode::Cbc,
            AesMode::Ecb,
            AesMode::Cfb,
            AesMode::Ofb,
            AesMode::Ctr,
        ];
        let paddings = [
            AesPadding::Pkcs7,
            AesPadding::Iso97971,
            AesPadding::AnsiX923,
            AesPadding::Iso10126,
            AesPadding::Zero,
        ];
        let keys = [
            (128, "0123456789abcdef"),
            (192, "0123456789abcdef01234567"),
            (256, "0123456789abcdef0123456789abcdef"),
        ];
        let texts = [
            "",
            "a",
            "0123456789ABCDEF",                            // 已对齐
            "这是一段需要加密的示例文本",                  // 多字节 UTF-8
            "The quick brown fox jumps over the lazy dog", // 多块
        ];
        for mode in modes {
            for (key_size, key) in keys {
                for padding in paddings {
                    for text in texts {
                        let enc = aes_encrypt(
                            text,
                            key,
                            "0123456789abcdef",
                            &opts(mode, key_size, padding),
                        )
                        .unwrap_or_else(|e| panic!("{:?}/{}: {}", mode, key_size, e));
                        let dec = aes_decrypt_to_bytes(
                            &enc,
                            key,
                            "0123456789abcdef",
                            &opts(mode, key_size, padding),
                        )
                        .unwrap_or_else(|e| panic!("{:?}/{}: {}", mode, key_size, e));
                        assert_eq!(
                            String::from_utf8(dec).unwrap(),
                            text,
                            "{:?}/{}:{:?}",
                            mode,
                            key_size,
                            padding
                        );
                    }
                }
                // NoPadding 仅对 16 倍数长度（已对齐）的明文做往返：
                // 旧行为下 NoPadding 本就要求对齐，非对齐明文/密文会报「密文长度无效」
                for text in ["0123456789ABCDEF", "0123456789ABCDEF0123456789ABCDEF"] {
                    let enc = aes_encrypt(
                        text,
                        key,
                        "0123456789abcdef",
                        &opts(mode, key_size, AesPadding::NoPadding),
                    )
                    .unwrap();
                    let dec = aes_decrypt_to_bytes(
                        &enc,
                        key,
                        "0123456789abcdef",
                        &opts(mode, key_size, AesPadding::NoPadding),
                    )
                    .unwrap();
                    assert_eq!(
                        String::from_utf8(dec).unwrap(),
                        text,
                        "NoPadding {:?}/{}",
                        mode,
                        key_size
                    );
                }
            }
        }
    }

    #[test]
    fn aes_encrypt_no_padding_rejects_unaligned_block_modes() {
        // NoPadding + 块模式（ECB/CBC）遇到非对齐明文此前会在末块 try_into panic，
        // 现在与解密侧对称地返回错误；流模式（CTR 等）不受影响
        let cbc = AesOptions {
            mode: AesMode::Cbc,
            key_size: 128,
            padding: AesPadding::NoPadding,
        };
        let err = aes_encrypt("hello", "0123456789abcdef", "0123456789abcdef01", &cbc).unwrap_err();
        assert!(err.contains("NoPadding"));

        let ecb = AesOptions {
            mode: AesMode::Ecb,
            key_size: 128,
            padding: AesPadding::NoPadding,
        };
        assert!(aes_encrypt("hello", "0123456789abcdef", "", &ecb).is_err());

        // 对齐文本在块模式下正常往返
        let ct = aes_encrypt(
            "0123456789abcdef",
            "0123456789abcdef",
            "0123456789abcdef01",
            &cbc,
        )
        .unwrap();
        let pt = aes_decrypt_to_bytes(&ct, "0123456789abcdef", "0123456789abcdef01", &cbc).unwrap();
        assert_eq!(pt, b"0123456789abcdef");

        // 流模式 + NoPadding 的非对齐数据保持可用
        let ctr = AesOptions {
            mode: AesMode::Ctr,
            key_size: 128,
            padding: AesPadding::NoPadding,
        };
        let ct2 = aes_encrypt("hello", "0123456789abcdef", "0123456789abcdef01", &ctr).unwrap();
        let pt2 =
            aes_decrypt_to_bytes(&ct2, "0123456789abcdef", "0123456789abcdef01", &ctr).unwrap();
        assert_eq!(pt2, b"hello");
    }

    #[test]
    fn aes_parse_key_errors() {
        let err = aes_encrypt(
            "x",
            "short",
            "0123456789abcdef",
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "密钥长度必须为 16 字符");
        let err = aes_encrypt(
            "x",
            "0123456789abcdef",
            "0123456789abcdef",
            &opts(AesMode::Cbc, 192, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "密钥长度必须为 24 字符");
        let err = aes_encrypt(
            "x",
            "0123456789abcdef01234567",
            "0123456789abcdef",
            &opts(AesMode::Cbc, 256, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "密钥长度必须为 32 字符");
        // 多字节 UTF-8 按字节数计
        let err = aes_encrypt(
            "x",
            "密钥",
            "0123456789abcdef",
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "密钥长度必须为 16 字符");
    }

    #[test]
    fn aes_parse_iv_errors() {
        let err = aes_encrypt(
            "x",
            UI_KEY,
            "short",
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "IV长度必须为16字符 (128位)");
        let err = aes_decrypt_to_bytes(
            "AAAA",
            UI_KEY,
            "",
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "IV长度必须为16字符 (128位)");
    }

    #[test]
    fn aes_iv_takes_first_16_bytes_and_ecb_ignores_iv() {
        // IV 超过 16 字节时截断取前 16 字节：17 字符与 16 字符结果一致
        let a = aes_encrypt(
            UI_TEXT,
            UI_KEY,
            UI_IV,
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap();
        let b = aes_encrypt(
            UI_TEXT,
            UI_KEY,
            "initialization-v",
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap();
        assert_eq!(a, b);
        // 更长 IV 同理
        let c = aes_encrypt(
            UI_TEXT,
            UI_KEY,
            "initialization-vEXTRA",
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap();
        assert_eq!(a, c);
        // ECB 使用全零 IV，任何 IV 输入都不影响结果
        let d1 = aes_encrypt(
            UI_TEXT,
            UI_KEY,
            "whatever-iv-here",
            &opts(AesMode::Ecb, 128, AesPadding::Pkcs7),
        )
        .unwrap();
        let d2 = aes_encrypt(
            UI_TEXT,
            UI_KEY,
            "different-values",
            &opts(AesMode::Ecb, 128, AesPadding::Pkcs7),
        )
        .unwrap();
        assert_eq!(d1, d2);
        assert_eq!(
            d1,
            "V/AMYXBsC3hK/nhFNjsv9exYRxsLKhxVxx5oYtlQ04ELVxtV/jZSfbNwT4Xt9eWS"
        );
    }

    #[test]
    fn aes_unpad_error_messages() {
        // 构造解密后末字节为 0x00 的密文（NoPadding 加密对齐块），Pkcs7 校验必失败
        let ct = aes_encrypt(
            "0123456789ABCDEF",
            UI_KEY,
            UI_IV,
            &opts(AesMode::Cbc, 128, AesPadding::NoPadding),
        )
        .unwrap();
        let err = aes_decrypt_to_bytes(
            &ct,
            UI_KEY,
            UI_IV,
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "解密失败：填充校验错误");
        // 流模式（CTR）：非对齐密文走 unpad 长度校验
        let ct = aes_encrypt(
            "0123456789ABCDEF",
            UI_KEY,
            UI_IV,
            &opts(AesMode::Ctr, 128, AesPadding::NoPadding),
        )
        .unwrap();
        let trimmed = &ct[..ct.len() - 4]; // 12 字节密文
        let err = aes_decrypt_to_bytes(
            trimmed,
            UI_KEY,
            UI_IV,
            &opts(AesMode::Ctr, 128, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "解密失败：密文长度无效");
        // ECB：非对齐密文直接报长度错误
        let err = aes_decrypt_to_bytes(
            trimmed,
            UI_KEY,
            UI_IV,
            &opts(AesMode::Ecb, 128, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "解密失败：密文长度无效");
    }

    #[test]
    fn aes_decrypt_invalid_base64() {
        let err = aes_decrypt_to_bytes(
            "not-base64!!!",
            UI_KEY,
            UI_IV,
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap_err();
        assert_eq!(err, "密文不是有效的Base64格式");
    }

    #[test]
    fn aes_zero_and_iso97971_unpad_edge_cases() {
        // 全零块 + ZeroPadding：全零不裁剪，返回原 16 字节
        let zeros_ct = aes_encrypt(
            "\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            UI_KEY,
            UI_IV,
            &opts(AesMode::Cbc, 128, AesPadding::NoPadding),
        )
        .unwrap();
        let out = aes_decrypt_to_bytes(
            &zeros_ct,
            UI_KEY,
            UI_IV,
            &opts(AesMode::Cbc, 128, AesPadding::Zero),
        )
        .unwrap();
        assert_eq!(out, vec![0u8; 16]);
        // 空明文 + Pkcs7：填充整块 0x10，解密得到空明文
        let empty_ct = aes_encrypt(
            "",
            UI_KEY,
            UI_IV,
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap();
        let out = aes_decrypt_to_bytes(
            &empty_ct,
            UI_KEY,
            UI_IV,
            &opts(AesMode::Cbc, 128, AesPadding::Pkcs7),
        )
        .unwrap();
        assert!(out.is_empty());
        // 空密文 + ZeroPadding：空明文（unpad 的空分支）
        let out = aes_decrypt_to_bytes(
            "",
            UI_KEY,
            UI_IV,
            &opts(AesMode::Ctr, 128, AesPadding::Zero),
        )
        .unwrap();
        assert_eq!(out, Vec::<u8>::new());
        // 非 16 倍数的密文 + ZeroPadding：走长度校验分支
        let one_zero_ct = aes_encrypt(
            "\0",
            UI_KEY,
            UI_IV,
            &opts(AesMode::Ctr, 128, AesPadding::NoPadding),
        )
        .unwrap();
        let raw = general_purpose::STANDARD.decode(&one_zero_ct).unwrap();
        assert_eq!(raw.len(), 1);
        let err = aes_decrypt_to_bytes(
            &one_zero_ct,
            UI_KEY,
            UI_IV,
            &opts(AesMode::Ctr, 128, AesPadding::Zero),
        )
        .unwrap_err();
        assert_eq!(err, "解密失败：密文长度无效");
    }

    #[test]
    fn generate_random_iv_format() {
        for _ in 0..8 {
            let iv = generate_random_iv();
            assert_eq!(iv.len(), 16);
            assert!(iv.chars().all(|c| c.is_ascii_alphanumeric()));
        }
        assert_ne!(generate_random_iv(), generate_random_iv());
    }

    #[test]
    fn generated_iv_round_trips_through_encrypt_decrypt() {
        // 生成的 IV 必须被 parse_iv 完整消费（16 字符即完整 IV）：
        // 同一 IV 加密后解密应还原明文
        let iv = generate_random_iv();
        let opts = AesOptions {
            mode: AesMode::Cbc,
            key_size: 128,
            padding: AesPadding::Pkcs7,
        };
        let key = "0123456789abcdef";
        let cipher = aes_encrypt("生成的 IV 回读验证", key, &iv, &opts).unwrap();
        let plain = aes_decrypt_to_bytes(&cipher, key, &iv, &opts).unwrap();
        assert_eq!(String::from_utf8(plain).unwrap(), "生成的 IV 回读验证");
    }

    // ---------- JWT ----------

    const RS256_PRIVATE_PKCS8_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDLHPVK05COc1PR
E954jOIBqnW2dPOlg0oHTsE89TLFPZJRwEoCzU8U3wJqDt9x4++WHynN3iV9RkeW
9C75/81XtuA3Ih1HQXz590p6dvIoUErwtiElktsqR0VQUKpBy7bCpLPQwI9XML70
UnjVFwGcPbRDKft8lEr3EXh6oHl6xMmJxkuNa35w3370ggjgs/GE6zt3BxQszBqi
BufReV6VeCkfCN4Yr0DO2YnLWNdf4BK0BmJ2N+ZkOQpVFS1D6oOvFmUoVCBoYYvC
X/vkUrun+4q9yRa2udjTRq2CODzNEw87Oe1F4cQX+hPNXLyqOUKVPkBzDPlKa6yT
OD913Y8RAgMBAAECggEALK/QlNeiqoCbnHiRXfbrA4BQGSY2c8AEocJ1N9bfhm8z
ObrDXcXzY+1jr9dZWEqUi/QLw8AgAeWcOw06p6SctjNlNaQTCnoRR8KdN3/ja7Ss
pDSB6u0aPH1FNfQlXUf3S/AmIqO29bEjFghuDr3azBf0SMGcE67hpmWI2vvi9jwg
FlTlEupEPQbov9VXYWq6iUByLP1z8gqEquzA/9p2sbNiDvmM+p9kGsnabUNp0saR
iAOTPUPXN4NnNmSACYa4Js5oaQFKOTzp2bdtWi6tWB/3LFYWinD7U+6soAE1sMWY
0xvDB9UAIvGFpv6nlPqIZuOViD1VIXiQto778h0+dQKBgQD82UYlpUxvzpbS/WjR
PUSXu3/bVcr+0PrcsB5t7CSIqbxMM0JisLw/syXqjVklx2T2cZ6/DubLNW+PJG4F
UANyIhmqbVYU9QG8ZAOiqCYMeuMCFS3l5b4hUiw+T6aiJdn2nud154mvkGZ2Oab/
guEZI9NLzrp8P/11bbByHzrKPQKBgQDNpQAUuT0QctT5GJYs3rzvW1mIimJnUHR7
xao2P6isGw65S1sIG9Cr8d4z6KRE3f8rJSyhcKnJXhDIy3uuOxOUrWSSlG8/sp0q
BGN1Raw6lbbPstLl6vQYZJyGs5cius0jIGpO7j2XPq14FC+h06/T4DdLoAYHwceI
CgC4WugpZQKBgQCq4SASIxN0YHb34sDq3/AyZ0oyblnBOnS/ntsByNyr66JRETdl
QTIIDseqK5q6+HWYRBNX5fc+ED/zylf8iSW7Baa40imiAi8JJcJ7NA560rYGW0/o
lgqXtFQoTGY+ITyb+qpMuY0ItJs6fR4KgmYkOlz7V5waiph9uwLLLaXqtQKBgAgu
+MHaXUO6Th9bix5d/sbHq9hzKVdTLE+AnjVqcxvmN7/H7+dys+sjxOPr7h2PeAgP
gqcqBJeu1pwm/NUCJNVV0RrELWkrgnWZlxujmib6rju1sg7UOsJ9vnzdyLXpGSC8
uX5KnMjYBa8xnnOiKO0Vjx3DUHoejAIt47lyZKrBAoGBAKLrHuWT5vxI+ZPiuA+K
QzSG8Lc1pErfNY5J1wBlJbHj4tOQZpAOtkgHVznTbKFDqQkgatO6BsiHuXuo+BLp
QTM2LRBQGwInyyCBA5HuAd1/LHyL7GuBa7irKWUyKif3l89xnmSaxowH58UG4fEV
rDwhmFCtp/vSVk0we59fwOaq
-----END PRIVATE KEY-----";

    /// 仅测试用 2048-bit RSA 密钥对（本机 openssl 生成，非生产密钥）。
    const RS256_PRIVATE_PKCS1_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAyxz1StOQjnNT0RPeeIziAap1tnTzpYNKB07BPPUyxT2SUcBK
As1PFN8Cag7fcePvlh8pzd4lfUZHlvQu+f/NV7bgNyIdR0F8+fdKenbyKFBK8LYh
JZLbKkdFUFCqQcu2wqSz0MCPVzC+9FJ41RcBnD20Qyn7fJRK9xF4eqB5esTJicZL
jWt+cN9+9III4LPxhOs7dwcULMwaogbn0XlelXgpHwjeGK9AztmJy1jXX+AStAZi
djfmZDkKVRUtQ+qDrxZlKFQgaGGLwl/75FK7p/uKvckWtrnY00atgjg8zRMPOznt
ReHEF/oTzVy8qjlClT5Acwz5Smuskzg/dd2PEQIDAQABAoIBACyv0JTXoqqAm5x4
kV326wOAUBkmNnPABKHCdTfW34ZvMzm6w13F82PtY6/XWVhKlIv0C8PAIAHlnDsN
OqeknLYzZTWkEwp6EUfCnTd/42u0rKQ0gertGjx9RTX0JV1H90vwJiKjtvWxIxYI
bg692swX9EjBnBOu4aZliNr74vY8IBZU5RLqRD0G6L/VV2FquolAciz9c/IKhKrs
wP/adrGzYg75jPqfZBrJ2m1DadLGkYgDkz1D1zeDZzZkgAmGuCbOaGkBSjk86dm3
bVourVgf9yxWFopw+1PurKABNbDFmNMbwwfVACLxhab+p5T6iGbjlYg9VSF4kLaO
+/IdPnUCgYEA/NlGJaVMb86W0v1o0T1El7t/21XK/tD63LAebewkiKm8TDNCYrC8
P7Ml6o1ZJcdk9nGevw7myzVvjyRuBVADciIZqm1WFPUBvGQDoqgmDHrjAhUt5eW+
IVIsPk+moiXZ9p7ndeeJr5Bmdjmm/4LhGSPTS866fD/9dW2wch86yj0CgYEAzaUA
FLk9EHLU+RiWLN6871tZiIpiZ1B0e8WqNj+orBsOuUtbCBvQq/HeM+ikRN3/KyUs
oXCpyV4QyMt7rjsTlK1kkpRvP7KdKgRjdUWsOpW2z7LS5er0GGSchrOXIrrNIyBq
Tu49lz6teBQvodOv0+A3S6AGB8HHiAoAuFroKWUCgYEAquEgEiMTdGB29+LA6t/w
MmdKMm5ZwTp0v57bAcjcq+uiURE3ZUEyCA7Hqiuauvh1mEQTV+X3PhA/88pX/Ikl
uwWmuNIpogIvCSXCezQOetK2BltP6JYKl7RUKExmPiE8m/qqTLmNCLSbOn0eCoJm
JDpc+1ecGoqYfbsCyy2l6rUCgYAILvjB2l1Duk4fW4seXf7Gx6vYcylXUyxPgJ41
anMb5je/x+/ncrPrI8Tj6+4dj3gID4KnKgSXrtacJvzVAiTVVdEaxC1pK4J1mZcb
o5om+q47tbIO1DrCfb583ci16RkgvLl+SpzI2AWvMZ5zoijtFY8dw1B6HowCLeO5
cmSqwQKBgQCi6x7lk+b8SPmT4rgPikM0hvC3NaRK3zWOSdcAZSWx4+LTkGaQDrZI
B1c502yhQ6kJIGrTugbIh7l7qPgS6UEzNi0QUBsCJ8sggQOR7gHdfyx8i+xrgWu4
qyllMion95fPcZ5kmsaMB+fFBuHxFaw8IZhQraf70lZNMHufX8Dmqg==
-----END RSA PRIVATE KEY-----";

    /// 仅测试用。
    const RS256_PUBLIC_SPKI_PEM: &str = "-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyxz1StOQjnNT0RPeeIzi
Aap1tnTzpYNKB07BPPUyxT2SUcBKAs1PFN8Cag7fcePvlh8pzd4lfUZHlvQu+f/N
V7bgNyIdR0F8+fdKenbyKFBK8LYhJZLbKkdFUFCqQcu2wqSz0MCPVzC+9FJ41RcB
nD20Qyn7fJRK9xF4eqB5esTJicZLjWt+cN9+9III4LPxhOs7dwcULMwaogbn0Xle
lXgpHwjeGK9AztmJy1jXX+AStAZidjfmZDkKVRUtQ+qDrxZlKFQgaGGLwl/75FK7
p/uKvckWtrnY00atgjg8zRMPOzntReHEF/oTzVy8qjlClT5Acwz5Smuskzg/dd2P
EQIDAQAB
-----END PUBLIC KEY-----";

    /// 仅测试用。
    const RS256_PUBLIC_PKCS1_PEM: &str = "-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEAyxz1StOQjnNT0RPeeIziAap1tnTzpYNKB07BPPUyxT2SUcBKAs1P
FN8Cag7fcePvlh8pzd4lfUZHlvQu+f/NV7bgNyIdR0F8+fdKenbyKFBK8LYhJZLb
KkdFUFCqQcu2wqSz0MCPVzC+9FJ41RcBnD20Qyn7fJRK9xF4eqB5esTJicZLjWt+
cN9+9III4LPxhOs7dwcULMwaogbn0XlelXgpHwjeGK9AztmJy1jXX+AStAZidjfm
ZDkKVRUtQ+qDrxZlKFQgaGGLwl/75FK7p/uKvckWtrnY00atgjg8zRMPOzntReHE
F/oTzVy8qjlClT5Acwz5Smuskzg/dd2PEQIDAQAB
-----END RSA PUBLIC KEY-----";

    /// 黄金向量用 header/payload（键为字母序，等于 serde_json BTreeMap 序列化顺序）。
    fn golden_header() -> Value {
        json!({"alg": "HS256", "typ": "JWT"})
    }

    fn golden_payload() -> Value {
        json!({"exp": 1700003600, "iat": 1700000000, "iss": "example.com", "sub": "1234567890"})
    }

    const GOLDEN_SIGNING_INPUT: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE3MDAwMDM2MDAsImlhdCI6MTcwMDAwMDAwMCwiaXNzIjoiZXhhbXBsZS5jb20iLCJzdWIiOiIxMjM0NTY3ODkwIn0";
    const GOLDEN_HS256: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE3MDAwMDM2MDAsImlhdCI6MTcwMDAwMDAwMCwiaXNzIjoiZXhhbXBsZS5jb20iLCJzdWIiOiIxMjM0NTY3ODkwIn0.Vi3ryRa0ARvjNNfNEW9UCxQBJAsRI3lPbwaxGAsOeXg";
    const GOLDEN_HS384: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE3MDAwMDM2MDAsImlhdCI6MTcwMDAwMDAwMCwiaXNzIjoiZXhhbXBsZS5jb20iLCJzdWIiOiIxMjM0NTY3ODkwIn0.o_l-L594XccVPk11wn0qekfSSEYizypnZNnJHoDIWJdjcCiEWR0hOgyBGp6Nj4Gd";
    const GOLDEN_HS512: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE3MDAwMDM2MDAsImlhdCI6MTcwMDAwMDAwMCwiaXNzIjoiZXhhbXBsZS5jb20iLCJzdWIiOiIxMjM0NTY3ODkwIn0.rJKxisiqf_lXIYmr_RQnKC2LeR7nqkmSfEKg1_yoEPzBg6r689IvFf7WeNOvyU2ICFQeL2hwRHiPqHeMnIMyNw";
    const GOLDEN_RS256: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE3MDAwMDM2MDAsImlhdCI6MTcwMDAwMDAwMCwiaXNzIjoiZXhhbXBsZS5jb20iLCJzdWIiOiIxMjM0NTY3ODkwIn0.wrODx8nJmLKs3SEjcrN8z52B9_uvrNmyeXNPWFM-exSHw5BiEN_HOnidw6awjcZ_nbBPxdxzURWIxUPi5v6xpOtxIzPPwSwZ24PdLliRUSWmLg4YDaPfVcaC6jXqaA0ImDHbcxedU0kvcHOOXXTa6Z8CDmqbUYNCu2HydCw9pkm4A7U1FgxzqPd--QU0AcfRmBRrD9plN9FK5ypwuTwWsuumSgbQ3vmjQI_zeo6ztW1o2XNjCis-_Jt-7EIrbWh6E2Z4neUC-rof5MJ-XU82eSU8upXPR1uY-j4VUgVTcTJikDpAc0LFk_xa-nsZYySO0TYRZ1mSCkhPVUvkNOuZjQ";

    #[test]
    fn decode_jwt_parses_and_reports_structured_errors() {
        let token = sign_jwt(
            "HS256",
            json!({ "alg": "HS256", "typ": "JWT" }),
            json!({ "sub": "u", "exp": 1700003600 }),
            "k",
        )
        .unwrap();
        let d = decode_jwt(&token).unwrap();
        assert_eq!(d.algorithm, "HS256");
        assert!(d.header.contains("\"alg\""));
        assert!(d.payload.contains("\"sub\""));
        assert_eq!(d.signature, token.split('.').nth(2).unwrap());

        // 段数不为 3
        assert_eq!(decode_jwt("a.b"), Err(JwtDecodeError::Malformed));
        // base64url 段非法
        assert!(matches!(
            decode_jwt("!!!.e30.x"),
            Err(JwtDecodeError::InvalidBase64(_))
        ));
        // Header 非法 JSON（base64("xyz") → "eHl6"）
        assert!(matches!(
            decode_jwt("eHl6.e30.x"),
            Err(JwtDecodeError::InvalidHeaderJson(_))
        ));
        // Payload 非法 JSON
        assert!(matches!(
            decode_jwt("e30.eHl6.x"),
            Err(JwtDecodeError::InvalidPayloadJson(_))
        ));
        // alg 缺失 → "Unknown"
        let no_alg = decode_jwt("e30.e30.x").unwrap();
        assert_eq!(no_alg.algorithm, "Unknown");
    }

    #[test]
    fn validate_pem_checks_markers_and_body() {
        assert!(validate_pem(
            "-----BEGIN PUBLIC KEY-----\nabc\n-----END PUBLIC KEY-----",
            "PUBLIC KEY"
        ));
        // RSA 前缀变体
        assert!(validate_pem(
            "-----BEGIN RSA PUBLIC KEY-----\nabc\n-----END RSA PUBLIC KEY-----",
            "PUBLIC KEY"
        ));
        // 标记缺失
        assert!(!validate_pem("no markers", "PUBLIC KEY"));
        // 空正文
        assert!(!validate_pem(
            "-----BEGIN PUBLIC KEY-----\n-----END PUBLIC KEY-----",
            "PUBLIC KEY"
        ));
        // 正文含非 base64 字符
        assert!(!validate_pem(
            "-----BEGIN PUBLIC KEY-----\nabc!\n-----END PUBLIC KEY-----",
            "PUBLIC KEY"
        ));
    }

    #[test]
    fn jwt_hs256_sign_golden_vector() {
        let token = sign_jwt(
            "HS256",
            golden_header(),
            golden_payload(),
            "your-256-bit-secret",
        )
        .unwrap();
        // 签名输入（header.payload）与完整令牌均与黄金向量逐字节一致
        assert!(token.starts_with(GOLDEN_SIGNING_INPUT));
        assert_eq!(token, GOLDEN_HS256);
    }

    #[test]
    fn jwt_hs384_hs512_golden_vectors() {
        let t384 = sign_jwt(
            "HS384",
            golden_header(),
            golden_payload(),
            "your-256-bit-secret",
        )
        .unwrap();
        assert_eq!(t384, GOLDEN_HS384);
        let t512 = sign_jwt(
            "HS512",
            golden_header(),
            golden_payload(),
            "your-256-bit-secret",
        )
        .unwrap();
        assert_eq!(t512, GOLDEN_HS512);
    }

    #[test]
    fn jwt_hs256_verify_and_tamper() {
        assert!(verify_jwt(GOLDEN_HS256, "HS256", "your-256-bit-secret").unwrap());
        assert!(!verify_jwt(GOLDEN_HS256, "HS256", "wrong-secret").unwrap());
        // 篡改 payload 末位字符
        let mut tampered = GOLDEN_HS256.to_string();
        let last = tampered.pop().unwrap();
        tampered.push(if last == 'g' { 'h' } else { 'g' });
        assert!(!verify_jwt(&tampered, "HS256", "your-256-bit-secret").unwrap());
    }

    #[test]
    fn jwt_hs_verify_rejects_malformed_signature_lengths() {
        // openssl::memcmp::eq 要求等长：签名段非摘要长度（空 / 截断 / 超长）时
        // 必须返回 Ok(false) 而非 panic（此前直接 assert! 崩溃）
        let token = sign_jwt(
            "HS256",
            json!({ "alg": "HS256", "typ": "JWT" }),
            json!({ "sub": "u" }),
            "k",
        )
        .unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        let header_payload = format!("{}.{}", parts[0], parts[1]);

        // 空签名段
        assert_eq!(
            verify_jwt(&format!("{header_payload}."), "HS256", "k"),
            Ok(false)
        );
        // 1 字节与 33 字节签名（32 字节摘要的两侧）
        assert_eq!(
            verify_jwt(&format!("{header_payload}.x"), "HS256", "k"),
            Ok(false)
        );
        assert_eq!(
            verify_jwt(
                &format!("{header_payload}.{}", "x".repeat(33)),
                "HS256",
                "k"
            ),
            Ok(false)
        );
        // HS512（64 字节摘要）同样覆盖
        assert_eq!(
            verify_jwt(&format!("{header_payload}."), "HS512", "k"),
            Ok(false)
        );
        // 完整令牌不受影响
        assert_eq!(verify_jwt(&token, "HS256", "k"), Ok(true));
    }

    #[test]
    fn jwt_rs256_sign_golden_vector_both_private_formats() {
        // PKCS#8 私钥
        let token = sign_jwt(
            "RS256",
            golden_header(),
            golden_payload(),
            RS256_PRIVATE_PKCS8_PEM,
        )
        .unwrap();
        assert_eq!(token, GOLDEN_RS256);
        // PKCS#1 私钥（手工包裹 PKCS#8 路径）签名结果一致（RSASSA-PKCS1-v1_5 确定性）
        let token_pkcs1 = sign_jwt(
            "RS256",
            golden_header(),
            golden_payload(),
            RS256_PRIVATE_PKCS1_PEM,
        )
        .unwrap();
        assert_eq!(token_pkcs1, GOLDEN_RS256);
    }

    #[test]
    fn jwt_rs256_verify_both_public_formats() {
        assert!(verify_jwt(GOLDEN_RS256, "RS256", RS256_PUBLIC_SPKI_PEM).unwrap());
        // PKCS#1 公钥（手工包裹 SPKI 路径）
        assert!(verify_jwt(GOLDEN_RS256, "RS256", RS256_PUBLIC_PKCS1_PEM).unwrap());
        // 其他公钥验证必失败（另一把密钥）
        let (_wrong_priv, wrong_pub) = other_test_key();
        assert!(!verify_jwt(GOLDEN_RS256, "RS256", &wrong_pub).unwrap());
    }

    #[test]
    fn jwt_rs256_tampered_signature_fails() {
        let mut tampered = GOLDEN_RS256.to_string();
        let last = tampered.pop().unwrap();
        tampered.push(if last == 'Q' { 'R' } else { 'Q' });
        assert!(!verify_jwt(&tampered, "RS256", RS256_PUBLIC_SPKI_PEM).unwrap());
    }

    /// 生成第二对测试密钥（openssl 内存生成，用于"错误公钥"分支）。
    fn other_test_key() -> (String, String) {
        use openssl::rsa::Rsa;
        let rsa = Rsa::generate(2048).unwrap();
        let pkey = PKey::from_rsa(rsa).unwrap();
        let priv_pem = String::from_utf8(pkey.private_key_to_pem_pkcs8().unwrap()).unwrap();
        let pub_pem = String::from_utf8(pkey.public_key_to_pem().unwrap()).unwrap();
        (priv_pem, pub_pem)
    }

    #[test]
    fn jwt_rs256_sign_verify_self_consistent() {
        let (priv_pem, pub_pem) = other_test_key();
        let token = sign_jwt("RS256", golden_header(), golden_payload(), &priv_pem).unwrap();
        assert!(verify_jwt(&token, "RS256", &pub_pem).unwrap());
        assert!(!verify_jwt(&token, "RS256", RS256_PUBLIC_SPKI_PEM).unwrap());
    }

    #[test]
    fn jwt_unknown_alg_error_message() {
        let err = sign_jwt("HS128", golden_header(), golden_payload(), "k").unwrap_err();
        assert_eq!(err, "不支持的签名算法：HS128");
        let err = verify_jwt(GOLDEN_HS256, "none", "k").unwrap_err();
        assert_eq!(err, "不支持的签名算法：none");
        let err = sign_jwt("", golden_header(), golden_payload(), "k").unwrap_err();
        assert_eq!(err, "不支持的签名算法：");
    }

    #[test]
    fn jwt_verify_non_3_part_tokens() {
        assert!(!verify_jwt("a.b", "HS256", "k").unwrap());
        assert!(!verify_jwt("a.b.c.d", "HS256", "k").unwrap());
        assert!(!verify_jwt("", "HS256", "k").unwrap());
        // 未知 alg 在段数检查之前抛错
        assert!(verify_jwt("a.b", "bad-alg", "k").is_err());
    }

    #[test]
    fn jwt_unrecognized_pem_error_messages() {
        let err = sign_jwt("RS256", golden_header(), golden_payload(), "not a pem").unwrap_err();
        assert_eq!(err, "无法识别的私钥格式");
        let err = verify_jwt(
            GOLDEN_RS256,
            "RS256",
            "-----BEGIN FOO-----\nAAAA\n-----END FOO-----",
        )
        .unwrap_err();
        assert_eq!(err, "无法识别的公钥格式");
    }

    #[test]
    fn jwt_decode_segments() {
        assert_eq!(
            decode_jwt_segment("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9").unwrap(),
            r#"{"alg":"HS256","typ":"JWT"}"#
        );
        assert_eq!(
            decode_jwt_segment("eyJleHAiOjE3MDAwMDM2MDAsImlhdCI6MTcwMDAwMDAwMCwiaXNzIjoiZXhhbXBsZS5jb20iLCJzdWIiOiIxMjM0NTY3ODkwIn0").unwrap(),
            r#"{"exp":1700003600,"iat":1700000000,"iss":"example.com","sub":"1234567890"}"#
        );
        // 标准 base64（带 + / 与 =）的输入也能解码：QWQ+ZA== 的 base64url 为 QWQ-ZA
        assert_eq!(
            decode_jwt_segment("QWQ-ZA").unwrap(),
            decode_jwt_segment("QWQ+ZA==").unwrap()
        );
        // 非法 base64 字符
        assert!(decode_jwt_segment("!!").is_err());
        // 非 UTF-8 字节按解码替换语义替换为 U+FFFD
        assert_eq!(decode_jwt_segment("__8").unwrap(), "\u{fffd}\u{fffd}");
    }

    #[test]
    fn jwt_verify_accepts_padded_standard_base64_signature() {
        // base64url 签名补 '=' 到 4 的倍数后仍可验证（历史版本补齐行为）
        let mut parts = GOLDEN_HS256.split('.');
        let header_seg = parts.next().unwrap();
        let payload_seg = parts.next().unwrap();
        let sig = parts.next().unwrap();
        let padded_sig = format!(
            "{}{}",
            sig,
            "=".repeat(sig.len().div_ceil(4) * 4 - sig.len())
        );
        assert_ne!(padded_sig, sig); // 确实补了 '='（43 -> 44）
        let token = format!("{}.{}.{}", header_seg, payload_seg, padded_sig);
        assert!(verify_jwt(&token, "HS256", "your-256-bit-secret").unwrap());
    }

    // ---------- bcrypt ----------

    #[test]
    fn bcrypt_verify_rejects_excessive_cost() {
        // cost 31 的哈希串（只解析 cost，不实际计算——真实计算要数十小时）
        let malicious = format!("$2b$31${}$", "a".repeat(22)) + &"b".repeat(31);
        assert!(!bcrypt_verify("pw", &malicious));
        // 正常 cost 的哈希不受影响
        let hash = bcrypt_hash("pw", 4).unwrap();
        assert!(bcrypt_verify("pw", &hash));
    }

    #[test]
    fn bcrypt_hash_and_verify() {
        let hash = bcrypt_hash("s3cret 密码!", 4).unwrap();
        assert!(hash.starts_with("$2"));
        // "$2b$04$<salt+hash>"：第 3 段为成本
        assert_eq!(
            hash.split('$').nth(2).map(|c| c.parse::<u32>().unwrap()),
            Some(4)
        );
        assert!(bcrypt_verify("s3cret 密码!", &hash));
        assert!(!bcrypt_verify("wrong", &hash));
        // 成本不同的哈希仍可互验（bcrypt 算法自描述）
        let hash12 = bcrypt_hash("s3cret 密码!", 12).unwrap();
        assert!(bcrypt_verify("s3cret 密码!", &hash12));
        // 非法哈希按验证失败处理
        assert!(!bcrypt_verify("s3cret 密码!", "garbage"));
    }

    #[test]
    fn bcrypt_hash_invalid_cost_is_passed_through() {
        // 轮数越界由 UI 校验；core 直接透传 bcrypt crate（cost 超范围 → Err）
        assert!(bcrypt_hash("pw", 2).is_err());
        assert!(bcrypt_hash("pw", 40).is_err());
    }
}
