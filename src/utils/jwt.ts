/**
 * JWT/JWS 签名与验证工具
 * 基于浏览器原生 Web Crypto API 实现，替代已停止维护的 jsrsasign。
 * 支持 HS256/384/512 与 RS256/384/512，PEM 私钥兼容 PKCS#1 与 PKCS#8 两种格式，
 * PEM 公钥兼容 PKCS#1 与 SPKI 两种格式。
 */

type RsaJwsAlg = 'RS256' | 'RS384' | 'RS512'
type HsJwsAlg = 'HS256' | 'HS384' | 'HS512'
export type JwsAlg = RsaJwsAlg | HsJwsAlg

const ALG_HASH: Record<JwsAlg, 'SHA-256' | 'SHA-384' | 'SHA-512'> = {
  HS256: 'SHA-256',
  HS384: 'SHA-384',
  HS512: 'SHA-512',
  RS256: 'SHA-256',
  RS384: 'SHA-384',
  RS512: 'SHA-512',
}

const RSA_OID = Uint8Array.of(0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01)
const NULL_PARAMS = Uint8Array.of(0x05, 0x00)

// ---------- base64 / base64url ----------

const bytesToBase64Url = (bytes: Uint8Array): string => {
  let binary = ''
  for (const byte of bytes) {
    binary += String.fromCharCode(byte)
  }
  return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
}

const base64UrlToBytes = (text: string): Uint8Array => {
  const base64 = text
    .replace(/-/g, '+')
    .replace(/_/g, '/')
    .padEnd(Math.ceil(text.length / 4) * 4, '=')
  return Uint8Array.from(atob(base64), (char) => char.charCodeAt(0))
}

const utf8ToBytes = (text: string): Uint8Array =>
  new TextEncoder().encode(text)

/**
 * 解码 JWT 的 base64url 段为 UTF-8 文本（header/payload 均为 JSON）
 */
export const decodeJwtSegment = (segment: string): string =>
  new TextDecoder().decode(base64UrlToBytes(segment))

// ---------- DER / PEM ----------

/** 构造单个 DER TLV 结构 */
const derWrap = (tag: number, content: Uint8Array): Uint8Array => {
  if (content.length < 128) {
    return Uint8Array.of(tag, content.length, ...content)
  }
  const lengthBytes: number[] = []
  for (let len = content.length; len > 0; len >>>= 8) {
    lengthBytes.unshift(len & 0xff)
  }
  return Uint8Array.of(tag, 0x80 | lengthBytes.length, ...lengthBytes, ...content)
}

/** 提取 PEM 中 base64 正文并解码为 DER 字节 */
const pemBodyToDer = (pem: string): Uint8Array => {
  const body = pem
    .replace(/-----BEGIN [^-]+-----/, '')
    .replace(/-----END [^-]+-----/, '')
    .replace(/\s+/g, '')
  return Uint8Array.from(atob(body), (char) => char.charCodeAt(0))
}

/**
 * PKCS#1 RSAPrivateKey → PKCS#8 PrivateKeyInfo
 * PKCS#8 = SEQUENCE { version 0, AlgorithmIdentifier, OCTET STRING { PKCS#1 } }
 */
const pkcs1PrivateKeyToPkcs8 = (pkcs1: Uint8Array): Uint8Array =>
  derWrap(
    0x30,
    concat(
      Uint8Array.of(0x02, 0x01, 0x00),
      derWrap(0x30, concat(RSA_OID, NULL_PARAMS)),
      derWrap(0x04, pkcs1),
    ),
  )

/**
 * PKCS#1 RSAPublicKey → SPKI SubjectPublicKeyInfo
 * SPKI = SEQUENCE { AlgorithmIdentifier, BIT STRING { 0x00, PKCS#1 } }
 */
const pkcs1PublicKeyToSpki = (pkcs1: Uint8Array): Uint8Array =>
  derWrap(
    0x30,
    concat(derWrap(0x30, concat(RSA_OID, NULL_PARAMS)), derWrap(0x03, concat(Uint8Array.of(0x00), pkcs1))),
  )

const concat = (...parts: Uint8Array[]): Uint8Array => {
  const total = parts.reduce((sum, part) => sum + part.length, 0)
  const out = new Uint8Array(total)
  let offset = 0
  for (const part of parts) {
    out.set(part, offset)
    offset += part.length
  }
  return out
}

const importRsaPrivateKey = async (pem: string, hash: 'SHA-256' | 'SHA-384' | 'SHA-512') => {
  let der = pemBodyToDer(pem)
  if (/-----BEGIN RSA PRIVATE KEY-----/.test(pem)) {
    der = pkcs1PrivateKeyToPkcs8(der)
  } else if (!/-----BEGIN PRIVATE KEY-----/.test(pem)) {
    throw new Error('无法识别的私钥格式')
  }
  return crypto.subtle.importKey(
    'pkcs8',
    der,
    { name: 'RSASSA-PKCS1-v1_5', hash },
    false,
    ['sign'],
  )
}

const importRsaPublicKey = async (pem: string, hash: 'SHA-256' | 'SHA-384' | 'SHA-512') => {
  let der = pemBodyToDer(pem)
  if (/-----BEGIN RSA PUBLIC KEY-----/.test(pem)) {
    der = pkcs1PublicKeyToSpki(der)
  } else if (!/-----BEGIN PUBLIC KEY-----/.test(pem)) {
    throw new Error('无法识别的公钥格式')
  }
  return crypto.subtle.importKey(
    'spki',
    der,
    { name: 'RSASSA-PKCS1-v1_5', hash },
    false,
    ['verify'],
  )
}

// ---------- JWS 签名与验证 ----------

/**
 * 生成 JWS 签名的 JWT 令牌
 * @param alg 签名算法（HS256/HS384/HS512/RS256/RS384/RS512）
 * @param header JWT 头部对象
 * @param payload JWT 载荷对象
 * @param key HS* 传 Secret Key 字符串；RS* 传 PEM 私钥文本
 * @returns 三段式 JWT 令牌
 */
export const signJwt = async (
  alg: string,
  header: object,
  payload: object,
  key: string,
): Promise<string> => {
  const hash = ALG_HASH[alg as JwsAlg]
  if (!hash) {
    throw new Error(`不支持的签名算法：${alg}`)
  }

  const signingInput = [
    bytesToBase64Url(utf8ToBytes(JSON.stringify(header))),
    bytesToBase64Url(utf8ToBytes(JSON.stringify(payload))),
  ].join('.')

  let signature: ArrayBuffer
  if (alg.startsWith('HS')) {
    const hmacKey = await crypto.subtle.importKey(
      'raw',
      utf8ToBytes(key),
      { name: 'HMAC', hash },
      false,
      ['sign'],
    )
    signature = await crypto.subtle.sign('HMAC', hmacKey, utf8ToBytes(signingInput))
  } else {
    const rsaKey = await importRsaPrivateKey(key, hash)
    signature = await crypto.subtle.sign('RSASSA-PKCS1-v1_5', rsaKey, utf8ToBytes(signingInput))
  }

  return `${signingInput}.${bytesToBase64Url(new Uint8Array(signature))}`
}

/**
 * 验证 JWS 签名的 JWT 令牌
 * @param token 三段式 JWT 令牌
 * @param alg 令牌头部声明的算法
 * @param key HS* 传 Secret Key 字符串；RS* 传 PEM 公钥文本
 * @returns 签名是否有效；密钥无法解析时抛出异常（由调用方决定提示文案）
 */
export const verifyJwt = async (
  token: string,
  alg: string,
  key: string,
): Promise<boolean> => {
  const hash = ALG_HASH[alg as JwsAlg]
  if (!hash) {
    throw new Error(`不支持的签名算法：${alg}`)
  }

  const parts = token.split('.')
  if (parts.length !== 3) {
    return false
  }

  const signingInput = `${parts[0]}.${parts[1]}`
  const signature = base64UrlToBytes(parts[2])

  if (alg.startsWith('HS')) {
    const hmacKey = await crypto.subtle.importKey(
      'raw',
      utf8ToBytes(key),
      { name: 'HMAC', hash },
      false,
      ['verify'],
    )
    return crypto.subtle.verify('HMAC', hmacKey, signature, utf8ToBytes(signingInput))
  }

  const rsaKey = await importRsaPublicKey(key, hash)
  return crypto.subtle.verify('RSASSA-PKCS1-v1_5', rsaKey, signature, utf8ToBytes(signingInput))
}
