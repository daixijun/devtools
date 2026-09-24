import React from 'react'
import {
  IconEncoding,
  IconCertificate,
  IconNetwork,
  IconDataFormat,
  IconMedia,
  IconDeveloper,
  IconTime,
  IconSettings,
} from '../components/icons/ToolIcons'


export interface ToolDefinition {
  id: string
  name: string
  description: string
  keywords: string[]
  category: string
  icon: React.ComponentType<{ className?: string }>
  component: React.LazyExoticComponent<React.ComponentType>
  windowSize: { width: number; height: number }
}

export interface ToolCategory {
  id: string
  name: string
  icon: React.ComponentType<{ className?: string }>
  tools: ToolDefinition[]
}


const Base64Converter = React.lazy(() => import('./Base64Converter'))
const UrlEncoderDecoder = React.lazy(() => import('./UrlEncoderDecoder'))
const AesCrypto = React.lazy(() => import('./AesCrypto'))
const Md5Crypto = React.lazy(() => import('./Md5Crypto'))
const ShaCrypto = React.lazy(() => import('./ShaCrypto'))
const JwtEncode = React.lazy(() => import('./JwtEncode'))
const JwtDecode = React.lazy(() => import('./JwtDecode'))
const PasswordGenerator = React.lazy(() => import('./PasswordGenerator'))
const PasswordHasher = React.lazy(() => import('./PasswordHasher'))
const RsaKeyGenerator = React.lazy(() => import('./RsaKeyGenerator'))

const CertificateViewer = React.lazy(() => import('./CertificateViewer'))
const CsrViewer = React.lazy(() => import('./CsrViewer'))
const CsrGenerator = React.lazy(() => import('./CsrGenerator'))
const PemToPfxConverter = React.lazy(() => import('./PemToPfxConverter'))
const PfxToPemConverter = React.lazy(() => import('./PfxToPemConverter'))
const SslChecker = React.lazy(() => import('./SslChecker'))

const SubnetCalculator = React.lazy(() => import('./SubnetCalculator'))
const IpInfo = React.lazy(() => import('./IpInfo'))
const DnsResolver = React.lazy(() => import('./DnsResolver'))
const WhoisLookup = React.lazy(() => import('./WhoisLookup'))

const JsonFormatter = React.lazy(() => import('./JsonFormatter'))
const FormatConverter = React.lazy(() => import('./FormatConverter'))
const JsonToGo = React.lazy(() => import('./JsonToGo'))
const SqlToGo = React.lazy(() => import('./SqlToGo'))
const SqlToEnt = React.lazy(() => import('./SqlToEnt'))

const ImageConverter = React.lazy(() => import('./ImageConverter'))
const ImagePreview = React.lazy(() => import('./ImagePreview'))
const VideoConverter = React.lazy(() => import('./VideoConverter'))
const ImageCompressor = React.lazy(() => import('./ImageCompressor'))

const RegexTester = React.lazy(() =>
  import('./RegexTester').then((m) => ({ default: m.RegexTester })),
)
const TimestampConverter = React.lazy(() => import('./TimestampConverter'))

const Settings = React.lazy(() => import('./Settings'))


const encodingTools: ToolDefinition[] = [
  {
    id: 'base64converter',
    name: 'Base64 编解码',
    description: 'Base64 编码和解码工具，支持文本和文件的 Base64 转换',
    keywords: ['base64', '编码', '解码', 'encode', 'decode', '转换', '文本'],
    category: 'encoding',
    icon: IconEncoding,
    component: Base64Converter,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'urlencoderdecoder',
    name: 'URL 编解码',
    description: 'URL 编码和解码工具，对 URL 中的特殊字符进行百分号编码转换',
    keywords: ['url', 'uri', '编码', '解码', 'encode', 'decode', 'percent', '百分号'],
    category: 'encoding',
    icon: IconEncoding,
    component: UrlEncoderDecoder,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'aescrypto',
    name: 'AES 加密/解密',
    description: 'AES 对称加密和解密工具，支持多种模式和填充方式',
    keywords: ['aes', '加密', '解密', 'encrypt', 'decrypt', '对称加密', 'symmetric'],
    category: 'encoding',
    icon: IconEncoding,
    component: AesCrypto,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'md5crypto',
    name: 'MD5 加密',
    description: 'MD5 哈希生成工具，计算文本或文件的 MD5 摘要值',
    keywords: ['md5', '哈希', 'hash', '摘要', 'digest', '散列', '加密'],
    category: 'encoding',
    icon: IconEncoding,
    component: Md5Crypto,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'shacrypto',
    name: 'SHA 哈希加密',
    description: 'SHA 系列哈希生成工具，支持 SHA-1/SHA-256/SHA-512 等算法',
    keywords: ['sha', 'sha1', 'sha256', 'sha512', '哈希', 'hash', '加密', '散列'],
    category: 'encoding',
    icon: IconEncoding,
    component: ShaCrypto,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'jwtencode',
    name: 'JWT 生成',
    description: 'JWT Token 生成工具，创建带有自定义 Header 和 Payload 的 JSON Web Token',
    keywords: ['jwt', 'token', '生成', 'generate', 'encode', '令牌', 'json web token'],
    category: 'encoding',
    icon: IconEncoding,
    component: JwtEncode,
    windowSize: { width: 1000, height: 750 },
  },
  {
    id: 'jwtdecode',
    name: 'JWT 解码',
    description: 'JWT Token 解码工具，解析和验证 JSON Web Token 的内容',
    keywords: ['jwt', 'token', '解码', 'decode', '解析', 'verify', '令牌'],
    category: 'encoding',
    icon: IconEncoding,
    component: JwtDecode,
    windowSize: { width: 1000, height: 750 },
  },
  {
    id: 'passwordgenerator',
    name: '密码生成器',
    description: '安全密码生成工具，可自定义长度、字符类型等选项生成强密码',
    keywords: ['密码', 'password', '生成', 'generate', '随机', 'random', '强密码'],
    category: 'encoding',
    icon: IconEncoding,
    component: PasswordGenerator,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'passwordhasher',
    name: '密码加密验证',
    description: '密码哈希和验证工具，支持 bcrypt 等算法对密码进行安全加密和比对',
    keywords: ['密码', 'password', '哈希', 'hash', '加密', 'verify', '验证', 'bcrypt'],
    category: 'encoding',
    icon: IconEncoding,
    component: PasswordHasher,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'rsakeygenerator',
    name: 'RSA 密钥对生成',
    description:
      'RSA 公私钥对生成工具，支持 PKCS#8 / PKCS#1 私钥与 SPKI / PKCS#1 / OpenSSH 公钥格式',
    keywords: [
      'rsa',
      'ssh',
      '密钥',
      '公钥',
      '私钥',
      'key pair',
      'generate',
      'pkcs8',
      'pkcs1',
      'pem',
      'authorized_keys',
    ],
    category: 'encoding',
    icon: IconEncoding,
    component: RsaKeyGenerator,
    windowSize: { width: 900, height: 700 },
  },
]

const certificateTools: ToolDefinition[] = [
  {
    id: 'certificate',
    name: '证书查看器',
    description: 'PEM 证书解析工具，查看证书的详细信息、有效期和公钥等',
    keywords: ['证书', 'certificate', 'pem', 'x509', '查看', 'viewer', 'ssl', 'tls'],
    category: 'certificate',
    icon: IconCertificate,
    component: CertificateViewer,
    windowSize: { width: 1100, height: 800 },
  },
  {
    id: 'csrviewer',
    name: 'CSR 查看',
    description:
      'PKCS#10 证书签名请求（CSR）解析工具，查看主题、公钥、签名算法与请求的扩展信息',
    keywords: [
      'csr',
      'pkcs10',
      'certificate request',
      '证书请求',
      '签名请求',
      'csr查看',
      'csr 查看',
      'subject',
      '公钥',
    ],
    category: 'certificate',
    icon: IconCertificate,
    component: CsrViewer,
    windowSize: { width: 950, height: 750 },
  },
  {
    id: 'csrgenerator',
    name: 'CSR 生成',
    description:
      '生成 PKCS#10 证书签名请求（CSR）与配套私钥，支持 RSA/ECC 密钥和 SAN 扩展',
    keywords: [
      'csr',
      'csr生成',
      'csr 生成',
      'generate',
      'generate csr',
      '生成证书请求',
      '证书请求',
      '签名请求',
      'pkcs10',
      'san',
      '私钥',
    ],
    category: 'certificate',
    icon: IconCertificate,
    component: CsrGenerator,
    windowSize: { width: 1000, height: 800 },
  },
  {
    id: 'pemtopfx',
    name: 'PEM 转 PFX',
    description: '将 PEM 格式证书转换为 PFX/PKCS#12 格式，方便在 Windows 等环境中使用',
    keywords: ['pem', 'pfx', 'pkcs12', '转换', 'convert', '证书', 'certificate', '导入'],
    category: 'certificate',
    icon: IconCertificate,
    component: PemToPfxConverter,
    windowSize: { width: 950, height: 750 },
  },
  {
    id: 'pfxtopem',
    name: 'PFX 转 PEM',
    description: '将 PFX/PKCS#12 格式证书转换为 PEM 格式，提取证书和私钥',
    keywords: ['pfx', 'pem', 'pkcs12', '转换', 'convert', '证书', 'certificate', '提取'],
    category: 'certificate',
    icon: IconCertificate,
    component: PfxToPemConverter,
    windowSize: { width: 950, height: 750 },
  },
  {
    id: 'sslchecker',
    name: '在线 SSL 检测',
    description: '在线 SSL 证书检测工具，检查网站的 SSL 证书信息和到期时间',
    keywords: ['ssl', 'tls', '检测', 'checker', '证书', 'certificate', 'https', '在线'],
    category: 'certificate',
    icon: IconCertificate,
    component: SslChecker,
    windowSize: { width: 950, height: 750 },
  },
]

const networkTools: ToolDefinition[] = [
  {
    id: 'subnetcalculator',
    name: '子网掩码计算器',
    description: '子网掩码计算工具，支持 IP 地址与子网掩码的网络规划计算',
    keywords: ['子网', 'subnet', '掩码', 'mask', '计算器', 'calculator', 'ip', 'cidr', '网络'],
    category: 'network',
    icon: IconNetwork,
    component: SubnetCalculator,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'ipinfo',
    name: 'IP 地址信息查询',
    description: 'IP 地址信息查询工具，获取 IP 地址的地理位置、运营商等详细信息',
    keywords: ['ip', '地址', 'address', '查询', 'lookup', '地理位置', 'geoip', '信息'],
    category: 'network',
    icon: IconNetwork,
    component: IpInfo,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'dnsresolver',
    name: 'DNS 解析工具',
    description: 'DNS 记录查询工具，解析域名对应的 A/AAAA/CNAME/MX/TXT 等记录',
    keywords: ['dns', '解析', 'resolve', '域名', 'domain', '记录', 'record', 'a记录', 'cname'],
    category: 'network',
    icon: IconNetwork,
    component: DnsResolver,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'whois',
    name: '域名 Whois 查询',
    description: '域名 Whois 信息查询工具，查看域名的注册信息、到期时间和注册商',
    keywords: ['whois', '域名', 'domain', '注册', '查询', 'lookup', '注册商', 'registrar'],
    category: 'network',
    icon: IconNetwork,
    component: WhoisLookup,
    windowSize: { width: 900, height: 700 },
  },
]

const dataFormatTools: ToolDefinition[] = [
  {
    id: 'jsonformatter',
    name: 'JSON 格式化',
    description: 'JSON 格式化工具，支持美化、压缩、验证 JSON 数据并高亮显示',
    keywords: ['json', '格式化', 'format', '美化', 'pretty', '验证', 'validate', '压缩'],
    category: 'dataformat',
    icon: IconDataFormat,
    component: JsonFormatter,
    windowSize: { width: 1100, height: 800 },
  },
  {
    id: 'formatconverter',
    name: '格式转换器',
    description: '多格式转换工具，支持 JSON、YAML、TOML 等数据格式之间的互相转换',
    keywords: ['格式', 'format', '转换', 'convert', 'json', 'yaml', 'toml', '互转'],
    category: 'dataformat',
    icon: IconDataFormat,
    component: FormatConverter,
    windowSize: { width: 1100, height: 800 },
  },
  {
    id: 'jsontogo',
    name: 'JSON 转 Go 结构体',
    description: '将 JSON 数据自动生成 Go 语言结构体定义，支持嵌套类型',
    keywords: ['json', 'go', 'golang', '结构体', 'struct', '转换', 'convert', '生成'],
    category: 'dataformat',
    icon: IconDataFormat,
    component: JsonToGo,
    windowSize: { width: 1100, height: 800 },
  },
  {
    id: 'sqltogo',
    name: 'SQL 转 Go 结构体',
    description: '将 SQL CREATE TABLE 语句转换为 Go 语言结构体定义，支持多表解析',
    keywords: ['sql', 'go', 'golang', '结构体', 'struct', '转换', 'convert', '建表', 'create table'],
    category: 'dataformat',
    icon: IconDataFormat,
    component: SqlToGo,
    windowSize: { width: 1100, height: 800 },
  },
  {
    id: 'sqltoent',
    name: 'SQL 转 Go Ent ORM',
    description: '将 SQL DDL 语句转换为 Go Ent ORM Schema 代码，支持多表和复杂类型映射',
    keywords: ['sql', 'go', 'ent', 'orm', 'schema', '转换', 'convert', '数据库', 'database'],
    category: 'dataformat',
    icon: IconDataFormat,
    component: SqlToEnt,
    windowSize: { width: 1100, height: 800 },
  },
]

const mediaTools: ToolDefinition[] = [
  {
    id: 'imageconverter',
    name: '图片格式转换',
    description: '图片格式转换工具，支持 PNG、JPEG、WebP、BMP 等常见图片格式互转',
    keywords: ['图片', 'image', '格式', 'format', '转换', 'convert', 'png', 'jpeg', 'webp', 'bmp'],
    category: 'media',
    icon: IconMedia,
    component: ImageConverter,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'imagepreview',
    name: '图片预览器',
    description: '图片预览和信息查看工具，支持拖拽上传、缩放浏览和图片元数据查看',
    keywords: ['图片', 'image', '预览', 'preview', '查看', 'viewer', '元数据', 'metadata', 'exif'],
    category: 'media',
    icon: IconMedia,
    component: ImagePreview,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'videoconverter',
    name: '视频格式转换',
    description: '视频格式转换工具，支持 MP4、WebM、AVI 等常见视频格式互转',
    keywords: ['视频', 'video', '格式', 'format', '转换', 'convert', 'mp4', 'webm', 'avi', 'ffmpeg'],
    category: 'media',
    icon: IconMedia,
    component: VideoConverter,
    windowSize: { width: 900, height: 700 },
  },
  {
    id: 'imagecompressor',
    name: '图片压缩工具',
    description: 'PNG/JPG 无损优化与质量压缩，支持等比缩放、批量处理、压缩前后滑块对比预览',
    keywords: ['图片', '压缩', 'image', 'compress', '无损', 'lossless', '缩放', 'resize', '优化', 'optimize', 'png', 'jpg', 'jpeg', 'webp', '瘦身'],
    category: 'media',
    icon: IconMedia,
    component: ImageCompressor,
    windowSize: { width: 1100, height: 820 },
  },
]

const developerTools: ToolDefinition[] = [
  {
    id: 'regextester',
    name: '正则表达式测试器',
    description: '正则表达式在线测试工具，实时匹配高亮并显示捕获组结果',
    keywords: ['正则', 'regex', 'regexp', '表达式', '测试', 'test', '匹配', 'match', 'pattern'],
    category: 'developer',
    icon: IconDeveloper,
    component: RegexTester,
    windowSize: { width: 1000, height: 750 },
  },
]

const timeTools: ToolDefinition[] = [
  {
    id: 'timestamp',
    name: '时间戳转换',
    description: 'Unix 时间戳转换工具，支持时间戳与可读日期的双向转换，支持秒级和毫秒级',
    keywords: ['时间戳', 'timestamp', 'unix', '转换', 'convert', '日期', 'date', '时间', 'time', 'epoch'],
    category: 'time',
    icon: IconTime,
    component: TimestampConverter,
    windowSize: { width: 900, height: 700 },
  },
]

const settingsTools: ToolDefinition[] = [
  {
    id: 'settings',
    name: '设置',
    description: '应用配置和偏好设置，全局快捷键、主题等',
    keywords: ['设置', 'settings', '配置', 'config', '偏好', 'preference', '快捷键', 'shortcut'],
    category: 'settings',
    icon: IconSettings,
    component: Settings,
    windowSize: { width: 900, height: 700 },
  },
]


export const toolCategories: ToolCategory[] = [
  { id: 'encoding', name: '编码/解码', icon: IconEncoding, tools: encodingTools },
  { id: 'certificate', name: '证书工具', icon: IconCertificate, tools: certificateTools },
  { id: 'network', name: '网络工具', icon: IconNetwork, tools: networkTools },
  { id: 'dataformat', name: '数据格式转换', icon: IconDataFormat, tools: dataFormatTools },
  { id: 'media', name: '媒体格式转换', icon: IconMedia, tools: mediaTools },
  { id: 'developer', name: '开发工具', icon: IconDeveloper, tools: developerTools },
  { id: 'time', name: '时间工具', icon: IconTime, tools: timeTools },
  { id: 'settings', name: '设置', icon: IconSettings, tools: settingsTools },
]

export const allTools: ToolDefinition[] = toolCategories.flatMap((c) => c.tools)

export function getToolById(id: string): ToolDefinition | undefined {
  return allTools.find((t) => t.id === id)
}

export function searchTools(query: string): ToolDefinition[] {
  if (!query.trim()) return allTools

  const q = query.toLowerCase().trim()

  return allTools.filter((tool) => {
    const categoryName = toolCategories.find((c) => c.id === tool.category)?.name ?? ''
    const haystack = [
      tool.name,
      tool.description,
      categoryName,
      ...tool.keywords,
    ]
      .join(' ')
      .toLowerCase()

    return haystack.includes(q)
  })
}
