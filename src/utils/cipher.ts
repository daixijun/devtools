/**
 * 哈希与对称加密工具
 * 哈希基于 @noble/hashes，AES 基于 @noble/ciphers + Web Crypto 随机数，
 * 填充与模式行为与原 crypto-js 实现保持逐字节兼容。
 */
import { cbc, cfb, ctr, ecb } from '@noble/ciphers/aes.js'
import { md5, sha1 } from '@noble/hashes/legacy.js'
import { pbkdf2 } from '@noble/hashes/pbkdf2.js'
import { sha224, sha256, sha384, sha512 } from '@noble/hashes/sha2.js'
import { keccak_512 } from '@noble/hashes/sha3.js'
import { bytesToHex, concatBytes, utf8ToBytes } from '@noble/hashes/utils.js'

export type AesMode = 'CBC' | 'ECB' | 'CFB' | 'OFB' | 'CTR'
export type AesPadding =
  | 'Pkcs7'
  | 'Iso97971'
  | 'AnsiX923'
  | 'Iso10126'
  | 'ZeroPadding'
  | 'NoPadding'

export interface AesOptions {
  mode: AesMode
  keySize: 128 | 192 | 256
  padding: AesPadding
}

const BLOCK_SIZE = 16

// ---------- 基础编解码 ----------

const bytesToBase64 = (bytes: Uint8Array): string => {
  let binary = ''
  for (const byte of bytes) {
    binary += String.fromCharCode(byte)
  }
  return btoa(binary)
}

const base64ToBytes = (text: string): Uint8Array =>
  Uint8Array.from(atob(text), (char) => char.charCodeAt(0))

const randomBytes = (length: number): Uint8Array =>
  crypto.getRandomValues(new Uint8Array(length))

const bytesToHexLower = (bytes: Uint8Array): string => {
  let hex = ''
  for (const byte of bytes) {
    hex += byte.toString(16).padStart(2, '0')
  }
  return hex
}

// ---------- 哈希（输入按 UTF-8 编码，输出小写 hex，与 crypto-js 一致） ----------

export const md5Hex = (text: string): string =>
  bytesToHex(md5(utf8ToBytes(text)))

export const sha1Hex = (text: string): string =>
  bytesToHex(sha1(utf8ToBytes(text)))

export const sha224Hex = (text: string): string =>
  bytesToHex(sha224(utf8ToBytes(text)))

export const sha256Hex = (text: string): string =>
  bytesToHex(sha256(utf8ToBytes(text)))

export const sha384Hex = (text: string): string =>
  bytesToHex(sha384(utf8ToBytes(text)))

export const sha512Hex = (text: string): string =>
  bytesToHex(sha512(utf8ToBytes(text)))

export const sha3Hex = (text: string): string =>
  // crypto-js 的 "SHA3" 实为标准定稿前的 Keccak 填充（非 FIPS-202），
  // 使用 keccak_512 保持与历史哈希值逐字节兼容
  bytesToHex(keccak_512(utf8ToBytes(text)))

/**
 * PBKDF2（HMAC-SHA256），与 crypto-js 的 PBKDF2 默认配置一致
 * @param password 密码
 * @param salt 盐值
 * @param iterations 迭代次数
 * @param dkLen 派生密钥长度（字节）
 */
export const pbkdf2Hex = (
  password: string,
  salt: string,
  iterations: number,
  dkLen = 32,
): string =>
  bytesToHex(
    pbkdf2(sha256, password, salt, { c: iterations, dkLen }),
  )

// ---------- AES 填充（与 crypto-js 各填充实现逐一对应） ----------

/**
 * 按指定策略填充数据到块大小的整数倍。
 * 注意：crypto-js 的 ZeroPadding 在数据已对齐时也会追加一整块零。
 */
const padData = (data: Uint8Array, padding: AesPadding): Uint8Array => {
  const n = BLOCK_SIZE - (data.length % BLOCK_SIZE)

  switch (padding) {
    case 'Pkcs7':
      // PKCS #5/7：追加 n 个值为 n 的字节
      return concatBytes(data, new Uint8Array(n).fill(n))

    case 'Iso97971':
      // ISO/IEC 9797-1 Method 2：先加 0x80，再补零到块边界
      {
        const padded = concatBytes(data, new Uint8Array(n))
        padded[data.length] = 0x80
        return padded
      }

    case 'AnsiX923':
      // ANSI X.9.23：补零，最后一个字节为填充长度
      {
        const padded = concatBytes(data, new Uint8Array(n))
        padded[padded.length - 1] = n
        return padded
      }

    case 'Iso10126':
      // ISO 10126：补随机字节，最后一个字节为填充长度
      {
        const padded = concatBytes(data, randomBytes(n))
        padded[padded.length - 1] = n
        return padded
      }

    case 'ZeroPadding':
      // crypto-js：仅当未对齐时补零到块边界；已对齐（含空数据）不加填充
      if (data.length % BLOCK_SIZE === 0) {
        return data
      }
      return concatBytes(data, new Uint8Array(BLOCK_SIZE - (data.length % BLOCK_SIZE)))

    case 'NoPadding':
      return data
  }
}

/**
 * 移除尾零（对应 crypto-js ZeroPadding.unpad）。
 * 全为零时不裁剪，与 crypto-js 行为一致。
 */
const stripTrailingZeros = (data: Uint8Array): Uint8Array => {
  for (let i = data.length - 1; i >= 0; i--) {
    if (data[i] !== 0) {
      return data.slice(0, i + 1)
    }
  }
  return data
}

/**
 * 移除填充。校验失败时抛错（crypto-js 同样会抛错，仅文案不同）。
 */
const unpadData = (data: Uint8Array, padding: AesPadding): Uint8Array => {
  if (padding === 'NoPadding') {
    return data
  }

  if (data.length === 0) {
    // 空密文对应空明文（crypto-js 行为），交由调用方按空结果处理
    return data
  }

  if (data.length % BLOCK_SIZE !== 0) {
    throw new Error('解密失败：密文长度无效')
  }

  switch (padding) {
    case 'Pkcs7': {
      const n = data[data.length - 1]
      if (n < 1 || n > BLOCK_SIZE) {
        throw new Error('解密失败：填充校验错误')
      }
      for (let i = data.length - n; i < data.length; i++) {
        if (data[i] !== n) {
          throw new Error('解密失败：填充校验错误')
        }
      }
      return data.slice(0, data.length - n)
    }

    case 'Iso97971':
      // 先去尾零，再去掉 0x80（与 crypto-js 一致，不额外校验）
      {
        const stripped = stripTrailingZeros(data)
        return stripped.slice(0, stripped.length - 1)
      }

    case 'AnsiX923':
    case 'Iso10126': {
      // 末字节为填充长度
      const n = data[data.length - 1]
      if (n < 1 || n > BLOCK_SIZE) {
        throw new Error('解密失败：填充校验错误')
      }
      return data.slice(0, data.length - n)
    }

    case 'ZeroPadding':
      return stripTrailingZeros(data)
  }
}

// ---------- AES 模式 ----------

interface RawCipher {
  encrypt: (data: Uint8Array) => Uint8Array
  decrypt: (data: Uint8Array) => Uint8Array
}

/**
 * OFB：用 ECB 加密递推的密钥流块（block_i = E(block_{i-1})，起始为 IV），
 * 再与数据异或。加解密同函数。
 */
const createOfb = (key: Uint8Array, iv: Uint8Array): RawCipher => {
  const xorKeystream = (data: Uint8Array): Uint8Array => {
    const stream = new Uint8Array(
      Math.ceil(data.length / BLOCK_SIZE) * BLOCK_SIZE,
    )
    let block = iv
    for (let offset = 0; offset < stream.length; offset += BLOCK_SIZE) {
      // noble 的 cipher 实例有单次使用保护，每个密钥流块使用新实例
      block = ecb(key, { disablePadding: true }).encrypt(block)
      stream.set(block, offset)
    }
    const out = new Uint8Array(data.length)
    for (let i = 0; i < data.length; i++) {
      out[i] = data[i] ^ stream[i]
    }
    return out
  }

  return { encrypt: xorKeystream, decrypt: xorKeystream }
}

const createCipher = (
  mode: AesMode,
  key: Uint8Array,
  iv: Uint8Array,
): RawCipher => {
  switch (mode) {
    case 'ECB':
      return ecb(key, { disablePadding: true })
    case 'CBC':
      return cbc(key, iv, { disablePadding: true })
    case 'CFB':
      // crypto-js 的 CFB 为 128 位全块反馈，与 noble 的 cfb 一致
      return cfb(key, iv)
    case 'CTR':
      return ctr(key, iv)
    case 'OFB':
      return createOfb(key, iv)
  }
}

/**
 * 解析密钥：按 UTF-8 编码，长度必须与所选密钥位数匹配。
 */
const parseKey = (keyStr: string, keySize: 128 | 192 | 256): Uint8Array => {
  const key = utf8ToBytes(keyStr)
  if (key.length !== keySize / 8) {
    throw new Error(`密钥长度必须为 ${keySize / 8} 字符`)
  }
  return key
}

/**
 * 解析 IV：按 UTF-8 编码，使用前 16 字节（与 crypto-js 的行为一致）。
 */
const parseIv = (ivStr: string): Uint8Array => {
  const iv = utf8ToBytes(ivStr)
  if (iv.length < BLOCK_SIZE) {
    throw new Error('IV长度必须为16字符 (128位)')
  }
  return iv.slice(0, BLOCK_SIZE)
}

/**
 * AES 加密，返回 Base64 密文（与 crypto-js 显式密钥模式的输出一致）。
 * @param text 明文
 * @param keyStr 密钥（UTF-8 字符串，长度 = 密钥位数/8）
 * @param ivStr 初始化向量（ECB 模式可传空）
 * @param opts 模式、密钥位数、填充
 */
export const aesEncrypt = (
  text: string,
  keyStr: string,
  ivStr: string,
  opts: AesOptions,
): string => {
  const key = parseKey(keyStr, opts.keySize)
  const iv = opts.mode === 'ECB' ? new Uint8Array(BLOCK_SIZE) : parseIv(ivStr)
  const padded = padData(utf8ToBytes(text), opts.padding)
  const cipher = createCipher(opts.mode, key, iv)
  return bytesToBase64(cipher.encrypt(padded))
}

/**
 * AES 解密，返回原始字节（调用方自行按 UTF-8 解码，便于保留原有错误提示逻辑）。
 * @param base64Text Base64 密文
 */
export const aesDecryptToBytes = (
  base64Text: string,
  keyStr: string,
  ivStr: string,
  opts: AesOptions,
): Uint8Array => {
  const key = parseKey(keyStr, opts.keySize)
  const iv = opts.mode === 'ECB' ? new Uint8Array(BLOCK_SIZE) : parseIv(ivStr)
  const cipher = createCipher(opts.mode, key, iv)
  const decrypted = cipher.decrypt(base64ToBytes(base64Text))
  return unpadData(decrypted, opts.padding)
}

/**
 * 生成 16 字节随机 IV 的十六进制文本。
 * 与原实现的 WordArray.random(16).toString() 输出一致（32 个 hex 字符）。
 */
export const generateRandomIvHex = (): string => bytesToHexLower(randomBytes(16))
