# 批次 I：13 个纯文本工具 UI 契约

> 来源：历史版本实现的通读提取（算法行为与中文文案契约的权威记录）。
> 实现时以本规格为准；「组件层」指 crates/app/src/components（见 crates/app/src/tools/README.md）。

## 共享契约（所有工具通用）

- **布局**：ToolLayout（标题+副标题+描述，内容区滚动）→ 组件层 `tool_page` + `themed_header`。
- **useCopyToClipboard**：`copy(text)` → 成功后按钮文案「复制 ⇄ 已复制 ✓」2 秒复位 → 组件层 `copy_button`（Clipboard 组件自带）。
- **错误展示约定**：13 个工具均不用全局 toast 报错，全部**内联红色横条** → `ui::error_banner(text, cx)`；无错误时不渲染。
- registry id：base64converter/urlencoderdecoder/aescrypto/md5crypto/shacrypto/jwtencode/jwtdecode/passwordgenerator/passwordhasher/jsonformatter/timestamp/subnetcalculator/formatconverter。
- 窗口尺寸已由 registry 驱动（shell 自动），无需处理。

---

## Base64Converter（900×700）
- 模式分段按钮「编码/解码」（默认编码）｜主输入 textarea｜placeholder 编码=`请输入要编码的文本...`、解码=`请输入要解码的Base64文本...`；顶部「输入长度: N」「输出长度: N」。
- 按钮：`示例`（编码模式填入 `Hello, World! 你好，世界！`；解码模式填入其 btoa）｜`交换输入/输出`（input=output 且模式翻转；disabled `!output||error`）｜`清空`（disabled `!input`）｜`复制结果`。
- 输出：只读 textarea，placeholder 编码=`编码结果将在这里显示...`/解码=`解码结果将在这里显示...`，**出错时 placeholder 显示错误文案**、output 清空。
- 联动：输入经 **200ms 防抖**自动转换；mode 变化重算。标题随模式：`Base64 编码|解码`，副标题 `将文本编码为Base64格式|将Base64格式解码为普通文本`。
- 算法（core::text_tools::base64）：编码 `btoa(unescape(encodeURIComponent(s)))`（即 UTF-8 字节→标准 base64）；解码先校验（失败 `输入的不是有效的Base64格式`），再 UTF-8 解码。错误前缀 `Base64编码失败`/`Base64解码失败`。

## UrlEncoderDecoder（900×700）
- 模式按钮「编码/解码」（默认编码）+「交换输入/输出」（无 disabled，交换并翻转模式）；textarea h-32，placeholder 编码=`输入要编码的文本`、解码=`输入要解码的 URL`。
- 居中主按钮，文案「编码/解码」，**手动触发、无防抖**。
- 输出：只读 textarea h-32。**无复制按钮**。下方固定「说明:」3 条列表（编码/解码用途、encodeURIComponent/decodeURIComponent）。
- 算法：`encodeURIComponent`/`decodeURIComponent`（core::text_tools::url）；异常把 `错误: <msg>` 写入输出框。

## AesCrypto（900×700）
- 顶部 Tab「加密/解密」（默认加密），两页状态独立（text/key/iv/result/error 各一套），**AES 配置共享**：模式 select（CBC 默认/ECB/CFB/OFB/CTR）、密钥长度 select（128/192/256 位默认 128）、填充 select（Pkcs7 默认、Iso97971、AnsiX923、Iso10126、ZeroPadding、NoPadding）。
- 输入：文本 textarea rows4｜结果只读 textarea rows4（placeholder `加密(解密)结果将显示在这里`，右上浮动「复制」小按钮仅结果非空显示）｜密钥单行输入 placeholder `请输入${keySize}位密钥 (${keySize/8} 字符)` + 提示 `密钥长度必须为 X 字符 (Y 位)`｜IV 输入（**mode≠ECB 时显示**）placeholder `请输入16字符的IV (128位)` +「随机生成」按钮（32 位小写 hex）+ 提示 `IV长度必须为16字符 (128位)`。
- 按钮：加密/解密（disabled `!text || !key`）+ `加载示例`（text=`这是一段需要加密的示例文本`，key=`my-secret-key-16`，iv=`initialization-ve`）。
- 前置校验（内联红框）：空文本→`请输入要加密(解密)的文本`；空密钥→`请输入密钥`；非 ECB 空 IV→`当前模式需要初始化向量(IV)`。
- 算法：core::crypto::aes_encrypt(text,key,iv,mode,key_size,padding)->Base64 / aes_decrypt_to_bytes→UTF-8 解码，空串→`解密失败，请检查密钥、IV和密文是否正确`。异常文案 `加密过程中发生错误: <msg>` / `解密过程中发生错误: <msg>`。

## Md5Crypto（900×700）三 Tab
- 文本页：textarea placeholder `请输入要加密的文本...`；显示输入长度；`示例`（`Hello, World! 你好，世界！`）、`清空`；**输入即算（无防抖）** md5_hex；输出浅底卡片文本「MD5 哈希值」空态 `MD5 哈希值将在这里显示...`；`复制结果`；错误红框 `加密过程中发生错误: <msg>`。
- 文件页：只读单行输入（placeholder `点击右边按钮选择文件...`）+ `选择文件` → rfd pick_file；路径变化自动读文件+md5_hex；处理中显示 `正在处理文件...`；`清空`；错误：`请先选择一个文件`、`选择文件时发生错误: <msg>`、`处理文件时发生错误: <msg>`。
- 批量页：`选择文件`（rfd pick_files 顺序逐个处理）、`清空`；表格列 `文件名/状态/哈希值`，状态徽章 `待处理/处理中/已完成/错误`（error 行 hash=`-`）；`复制结果`（所有已完成行 `名: 哈希` 换行拼接）；`导出结果` → rfd save_file（txt）+ std::fs::write；空态 `点击"选择文件"按钮选择多个文件进行批量MD5加密`。单文件失败仅置 error 不弹 UI。

## ShaCrypto（900×700）= Md5Crypto 结构 + 全局算法 select
- 顶部 `选择哈希算法:`：sha1/sha224/sha256(默认)/sha384/sha512/sha3；算法切换立即重算文本页、文件页并影响批量。
- 输出标题动态 `${算法名} 哈希值`；空态 `点击"选择文件"按钮选择多个文件进行批量${算法名}加密`。
- sha3 = **Keccak-512**（core::crypto::sha3_hex）。

## JwtEncode（1000×750）
- 头部 actions：`示例`、`清空`。
- Header JSON 编辑器（h-200，默认 `{alg:'HS256',typ:'JWT'}` pretty 2）解析失败红字 `Header 格式错误: 请输入有效的 JSON`；Payload 同构默认 `{sub:'1234567890', iss:'', iat: 当前秒}` 错误 `Payload 格式错误: 请输入有效的 JSON`；`有效期 (秒)` number 默认 3600。
- 密钥区按 header.alg 联动：HS* → Secret Key 单行（默认 `mySecretKey`）；RS* → RSA 私钥区（rfd 选 .pem/.key 文件 or 粘贴 textarea），校验：须含 `-----BEGIN (RSA )?PRIVATE KEY-----`/END、正文 base64 字符，否则 `RSA私钥格式无效，请检查或重新选择文件`。
- 按钮 `生成JWT`（primary lg 全宽；disabled：JSON 错误||(RS 且私钥无效)）。
- 行为：expiresIn 非正整数→输出框写 `生成出错: 有效期必须是正整数`；否则 payload.exp=now+expiresIn（iat 保持）；调 core::crypto::sign_jwt(alg, header_json, payload_json, key)；异常前缀 `生成JWT失败` 写入输出框。
- 输出：「生成的JWT」只读 textarea h-24；`生成出错` 开头则红色无复制；否则 `复制JWT`。
- 示例：header HS256/JWT；payload sub=`1234567890`, iss=`example.com`, iat=now, exp=now+3600；secret=`your-256-bit-secret`；expiresIn=3600。清空：payload `{sub:'',iss:'',iat:now}`、密钥清空、expiresIn=3600。

## JwtDecode（1000×750）
- 输入：JWT Token 多行输入（h-120，placeholder `请输入JWT令牌...`）+ 长度显示；`清空输入`（同时清 key/结果/验证区）。
- **300ms 防抖自动解码**。空→全清；`split('.')` 必须恰 3 段否则 `无效的JWT格式：JWT应该包含3个部分，用点号分隔`；header/payload 各 base64url 解码 + JSON pretty(2)；algorithm=header.alg；signature=第三段原文；失败清空三区红框 `错误: JWT解码失败: <msg>`。
- 验证面板（黄色卡片，解码成功显示）：标题 `⚠️ JWT签名验证` + `算法: X`；固定安全提醒「JWT签名验证通常在服务端进行……」；HS* → Secret Key 输入（提示 `💡 输入生成JWT时使用的密钥`）；RS* → RSA 公钥区（校验 PUBLIC KEY 头，错误 `RSA公钥格式无效，请检查或重新选择文件`，提示 `💡 输入与签名私钥对应的RSA公钥`）。按钮 `验证JWT签名`。
- 验证分支：空输入→warning `请先输入JWT令牌`；空 key→warning `请输入Secret Key|请输入RSA公钥`；HS→verify_jwt（`JWT签名验证成功`/`JWT签名验证失败：签名不匹配`）；RS→校验失败 `RSA公钥格式无效，请检查公钥内容`，调用抛错→`RSA公钥解析失败，请检查公钥格式`；其他算法→warning `不支持的签名算法：${alg}`。结果绿/黄/红横幅（✓/⚠/✕）。
- 输出：Header/Payload 两个只读 JSON 编辑器（并排，各带复制按钮，复制 pretty 文本）；Signature 只读编辑器 h-80 + 复制（base64url 原文）。

## PasswordGenerator（900×700）
- `密码长度: N` slider（4–64，默认 16，两端标注 4/64）→ gpui_component Slider。
- 字符类型 4 个 checkbox 全勾选：`大写字母 (A-Z)`/`小写字母 (a-z)`/`数字 (0-9)`/`特殊符号`；勾选特殊符号时出现自定义区：快捷 chips——常用=`!@#$%^&*()_+-=`、括号=`[]{}()<>`、标点=`.,:;?!`、数学=`+-*/=<>`、货币=`$€£¥¢`、其他=`|~`'"`；自定义输入默认 `'!@#$%^&*()_+-=[]{}|;:,.<>?'`，下方 `当前字符数: N`。
- 按钮：`生成新密码`、`重置选项`（恢复默认）。
- 输出：卡片居中 mono 大字密码，空态 `点击生成按钮创建密码`；强度徽章 `强度: X`（score=(长度≥12?2:≥8?1:0)+命中类型数；≥5 非常强绿、≥4 强 primary、≥3 中等黄、否则弱红）；`复制密码`。
- 联动：**挂载及任意 option 变化自动生成**。字符集=大写26+小写26+数字10+自定义符号依次拼接；随机取模；全不选显示 `请至少选择一种字符类型`。底部「密码强度说明」4 条。

## PasswordHasher（900×700）Tab `密码加密`/`密码验证`
- 算法选择卡片按钮（两页各一份）：bcrypt（安全，业界标准，推荐密码存储）/PBKDF2（安全，HMAC-SHA256）/SHA-256（不安全，仅完整性）/SHA-512（不安全）/MD5（不安全，兼容性）。
- 加密页：密码输入（type=password）placeholder `请输入要加密的密码...`；bcrypt→`盐值轮数 (Salt Rounds)` number（4-20 默认 10，提示「推荐值为 10-12。范围：4 - 20」）；pbkdf2→`迭代次数 (Iterations)` number（1000-1000000 默认 10000）；按钮 `加密密码`（处理中 `加密中...`）+ `重置`。
- 行为：空密码→hash 区写 `请输入密码`；bcrypt 轮数越界→`盐值轮数必须在 4-20 之间`；pbkdf2 越界→`迭代次数必须在 1000-1000000 之间`；bcrypt=core::crypto::bcrypt_hash(pw,rounds)；md5/sha256/sha512=对应 hex；pbkdf2=`pbkdf2_hex(password,"salt",iterations,32)`（盐固定 'salt'，dkLen 32）。异常→`加密过程中发生错误: <msg>`。结果卡 `加密结果` + `复制` + 底部 `算法: X | 哈希长度: N 字符` + 当前算法说明卡。
- 验证页：算法卡（默认 bcrypt）；密码输入；哈希值 textarea rows3 placeholder `请输入要验证的哈希值...`；bcrypt 显示固定说明「bcrypt 的盐值轮数已包含在哈希值中，验证时无需手动设置」；pbkdf2 显示迭代次数输入（**与加密页共享 state**）。按钮 `验证密码`（`验证中...`）+ `重置`。
- 行为：空密码→`请输入要验证的密码`；空 hash→`请输入哈希值`；bcrypt=bcrypt_verify；md5/sha=重算严格相等；pbkdf2=同盐重算比对；成功 `✅ 密码验证成功！`绿 / 失败 `❌ 密码验证失败！`红；异常 `验证过程中发生错误: <msg>`；未知算法 `不支持的算法`。底部「安全提示」4 条。

## JsonFormatter（1100×800）单编辑器
- 代码编辑器（json 高亮），格式化结果**写回同一编辑器**。
- 工具条：`格式化`、`压缩`、分隔、`示例`、`去除转义`、`清空`（示例外 disabled `!input`）；右侧 `长度: N` + `复制`（编辑器全文）。
- 行为（core::text_tools::format）：格式化=JSON 校验（失败 `输入的不是有效的JSON格式`）→pretty 2，错误前缀 `JSON格式化失败`；压缩→压缩串，前缀 `JSON压缩失败`；去除转义=parse 后若是**字符串**再 parse 输出压缩 JSON，否则 pretty；parse 链失败回退纯文本替换 `\\"→"`、`\\\\→\` 写回，再失败前缀 `去除转义失败`；清空清输入+错误。
- 示例：`{"name":"张三","age":30,"city":"北京","hobbies":["读书","电影","旅游"],"address":{"street":"朝阳区某某路","zipcode":"100000"},"active":true}`。

## TimestampConverter（900×700）
- 「当前时间」卡每 **1s** 刷新：`时间戳（秒）`/`时间戳（毫秒）`/`当前时间:`（本地化中文）。
- 输入：文本框（placeholder `输入时间戳或日期时间`）+ `当前时间`按钮（ts→dt 按单位填秒/毫秒；dt→ts 填 ISO8601；填后立即转换）｜转换类型 select：`时间戳 → 日期时间`(默认)/`日期时间 → 时间戳`｜时区 select（全量 tz 数据库，label 值中 `_`→空格，默认 Asia/Shanghai）｜毫秒 checkbox（仅 ts→dt 显示，默认开）。
- 按钮 `转换`（手动）。
- 输出：只读单行（placeholder `转换结果`）。**无复制**。
- 算法（core::text_tools::timestamp）：空→`请输入值`；ts→dt：parseInt NaN→`请输入有效数字`；非毫秒×1000；范围 [0,9999999999999] 外→`时间戳超出合理范围`；输出 `zh-CN 本地化格式`（YYYY年M月D日 HH:mm:ss 或对齐 toLocaleString('zh-CN')）。dt→ts：parse 失败→`请输入有效的日期时间格式`；输出毫秒或 floor(/1000) 字符串。

## SubnetCalculator（900×700）
- IP 版本 radio 三选：auto(默认)/IPv4/IPv6；主输入 + `计算` 按钮；placeholder 随版本（ipv4=`输入IPv4和CIDR或子网掩码 (例如: 192.168.1.0/24)`、ipv6=`输入IPv6和前缀 (例如: 2001:db8::/64)`、auto=`输入IP和CIDR (例如: 192.168.1.0/24 或 2001:db8::/64)`）。
- **500ms 防抖自动计算**（有值才算）+ 按钮手动。
- 示例卡：IPv4 `192.168.1.0/24 或 10.0.0.0/255.0.0.0`；IPv6 `2001:db8::/64 或 fe80::/10`；简写 `::1/128 或 2001:db8::1/64`。
- 错误（红框 + 结果清空）：`请输入IP地址和CIDR或子网掩码`/`输入格式应为 IP/CIDR 或 IP/子网掩码`/`无法识别IP地址版本`/`IP地址版本不匹配：检测到${..}，但选择了${..}`/`无效的IPv4地址`/`无效的子网掩码`/`无效的CIDR值 (IPv4: 0-32)`/`无效的IPv6地址`/`IPv6只支持CIDR格式 (例如: /64)`/`无效的前缀长度 (IPv6: 0-128)`。版本检测：含 `.`→v4，含 `:`→v6。
- IPv4（core::text_tools::subnet）：`/`后含 `.`→掩码（过 0-255 校验）→maskToCidr（二进制 1 计数）；否则 CIDR 0–32→cidrToMask。network=ip&mask 按段；broadcast=network|(255-mask)；total=2^(32-cidr) 千分位；usable=cidr==32?1:max(0,total-2)；first/last=network+1/broadcast-1（cidr≥31 显示 `N/A`）。
- IPv6：仅 CIDR 0–128；`::` 展开（左右组补 0000，每组 pad4）；network=前 prefix 位保留；输出完整 8 组小写 hex 冒号分隔；总地址=主机位≤53 时 2^n 千分位否则 `2^${n}`；networkPortion=前 ceil(prefix/16) 组，hostPortion=其余（空显 `N/A`）；firstIp=network；lastIp 固定文案 `计算复杂`；mask/broadcast 空。
- 输出卡「计算结果 (IPv4|IPv6)」label:value 行：版本/CIDR；v4 追加 子网掩码/广播地址；通用 网络地址；v4 `第一个可用IP:` / v6 `网络前缀:`；v4 `最后一个可用IP:`；总IP数/总地址数；可用IP数/可用地址数；v6 追加 网络部分/主机部分。

## FormatConverter（1100×800）
- `输入格式` select（JSON/YAML/TOML 默认 JSON）、`输出格式` select 同。
- 按钮：`示例`、`清空`（disabled `!input`）、`复制结果`（disabled `!output||error`）。
- 联动：input/格式任一变化**立即自动转换（无防抖）**；空输入清输出；格式相同→错误 `输入和输出格式相同，无需转换`。
- 算法（core::text_tools::format）：json=JSON；yaml=serde_yaml 语义等价（indent 2）；toml=解析用 toml crate，**序列化用移植的手写序列化器**：基本类型数组内联 `[a, b]`；对象数组/混合数组多行 `[\n  <递归>,\n]`；对象先简单键（嵌套时 `[table.path]` 表头）+空行，再递归子表 `[parent.child]`；键含 `[^a-zA-Z0-9_-]` 双引号包裹转义 `"`；字符串值双引号包裹（先 `"`→`\"` 再 `\`→`\\`，**保持此转义顺序与旧行为一致**）；null 输出 `null`。异常包裹 `转换失败: ${msg}` 红框。
- 输出：左右两个编辑器（grid 2 列），输入 placeholder `粘贴 JSON、YAML 或 TOML 内容...`，输出 readOnly placeholder `转换结果将显示在这里...`。示例：`{"name":"开发者工具箱","version":"1.0.0","description":"一个功能强大的开发者工具集合","features":[...4项],"config":{"theme":"dark","language":"zh-CN","autoSave":true}}`。
