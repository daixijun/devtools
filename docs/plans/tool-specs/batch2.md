# 批次 II：10 个证书/网络/正则工具 UI 契约

> Rust 逻辑在 crates/core/src/tools/*。UI 直接调用 core 函数（无 IPC）。
> 返回字段名 = serde 字段名（核对 crates/core/src/tools 对应文件）。
> registry id：certificate/csrviewer/csrgenerator/pemtopfx/pfxtopem/rsakeygenerator/regextester/ipinfo/whois/jsontogo

## 通用（对齐批次 I 共享约定）
- FileUpload 双模式（文件选择区 / 粘贴文本区）：组件层没有现成实现——用「按钮选文件（rfd，后台线程）+ 可切换粘贴 textarea」实现；binary 模式读字节（Vec<u8>），text 模式读 UTF-8。
- 错误：内联红横条 `ui::error_banner`；loading：`ui::loading_row` 或按钮 `.loading(true)`。
- 复制：`ui::copy_button`；下载保存：rfd `save_file` + `std::fs::write`（后台线程），成功 toast 2 秒（notify::toast_success）。

## CertificateViewer（1100×800，id=certificate）
- 输入：FileUpload(binary) accept `.pem,.crt,.cer,.pfx,.p12`；上传后显示「已上传文件： {文件名}」+类型徽标（PEM/PFX 按扩展名/`-----BEGIN CERTIFICATE-----` 检测）；PFX 追加密码输入「PFX文件密码（可选）」（Enter 触发解析）；文本模式有「Base64解码」复选框（勾选先 base64 解码再解析）。
- 按钮：`解析证书`（通栏 primary，disabled=loading 或输入与文件均空）。
- 输出：解析成功自动进入全屏模式（也可手动切换）。顶部链状态卡：`is_full_chain`→绿点「完整证书链」/红「证书链不完整 - 缺少CA证书」+ `chain_status`；顺序检测卡（`order_check`）：`is_correct` 为假显示黄条 + `message` +「当前顺序」`actual_order` 与「正确顺序」`expected_order`（每项 `{role, subject_cn, serial_number}`），`corrected_pem` 有值给「复制正确顺序的证书」；Tab「证书链概览」+「证书 N - {终端证书|中间CA证书|根CA证书}」（chain_level 0/1/2）。概览页含颁发关系图（`missing_certificates` 红卡：subject_name/issuer_name/description，节点间「颁发给 ↓」+图例）+链级信息（`ca_download_urls`、`missing_ca_info`）。每证书详情：主题/颁发者（`subject`/`issuer` 中文 key map 如「通用名称 (CN)」）、有效期（`validity.not_before`/`not_after`/状态：`days_until_expiry` 负→红「已过期 N 天」、<30→黄「还有 N 天到期」否则绿）、详细信息（终端证书：`certificate_type`/`brand`；`serial_number`/`signature_algorithm`/`sha1_fingerprint`/`sha256_fingerprint`）、公钥（`public_key_info.key_type`/`key_size` bits/`algorithm`）、SAN（终端证书逐条 `sans`）。无复制（除 corrected_pem）。
- 调用：PEM→core `parse_pem_certificate(pem: String)`；PFX→`parse_pfx_certificate(pfx_data: Vec<u8>, password: Option<String>)`。返回 `CertificateChainInfo` 全 snake_case（见 core/src/tools/certificate_viewer.rs）。
- 特殊：文本输入清空已上传文件状态；清空输入重置全部；全屏组件「退出全屏」返回。

## CsrViewer（950×750）
- 输入：FileUpload(binary) accept `.csr,.pem,.txt` placeholder「粘贴 PEM 格式的 CSR 文件…」上传后「已选择文件： {文件名}」；可选私钥 textarea h-24 label「私钥（可选）— 提供 PKCS#8 / PKCS#1 / SEC1 私钥，校验其与 CSR 公钥是否匹配」。
- 按钮：`解析 CSR`（loading「解析中…」）；`清空`。
- 输出（Section 卡 + InfoRow）：基本信息卡 actions=徽标 `signature_valid`→「✓ 签名验证通过」绿/「✗ 签名验证失败」红；行：CSR 版本（version==0 显示「v1（版本号 0）」）、签名算法=`signature_algorithm`、DER 大小=`der_size`（<1024 「N 字节」否则 KB）；主题（Subject）卡：`subject[]` 每项 name 左 value+「OID: {oid}」右；公钥信息卡：`key_type`+「 · N 位」(key_size)+「 · 曲线 {curve_name}」，公钥指纹=`fingerprint_sha256`+复制按钮，公钥 PEM=`spki_pem` 只读 h-32+复制；私钥校验卡（仅提供私钥时）：徽标 `matches_csr`→「✓ 私钥与 CSR 匹配」/「✗ 私钥与 CSR 不匹配」，行 `format`/密钥类型/指纹；请求扩展卡：`requested_extensions[]`（name+critical 红徽标+value）；原始 CSR 卡：复制+「下载 .csr」（save 对话框默认 certificate_request.csr），textarea h-40 显示 `pem`。
- 调用：core `parse_csr(csr_content: Option<String>, csr_der: Option<Vec<u8>>, private_key: Option<String>)`。返回 camelCase：version/subject{oid,name,value}/publicKey{keyType,keySize,curveName,spkiPem,fingerprintSha256}/privateKey{...}/signatureAlgorithm/requestedExtensions{name,value,critical}/signatureValid/derSize/pem（**TS 侧是 camelCase**——Rust serde rename_all=camelCase；直接用 Rust 结构体字段 snake_case，无需转换）。
- 空态：「上传或粘贴 CSR 文件（PEM / Base64 / DER 格式）后点击「解析 CSR」」。文件含 `-----BEGIN` 按文本否则按 DER 字节。

## CsrGenerator（1000×800）
- 输入：通用名称（CN，必填）placeholder「如 example.com」（Enter 触发）；密钥类型 select RSA/ECC；RSA→密钥长度 select（2048 推荐/3072/4096），ECC→曲线 select（P-256 prime256v1/P-384 secp384r1/P-521 secp521r1）；国家（C）w-32「两位字母，如 CN」、省份、城市、组织、部门、邮箱；SAN textarea h-20 label「SAN（可选，每行一个域名或 IP，IP 自动识别）」。loading 时全部禁用。
- 按钮：`生成 CSR`；CN 空→错误「请填写通用名称（CN），通常为要申请证书的域名」。
- 输出：结果卡（主题=`subject` 整串；密钥=`key_type`+位+曲线；SAN=`sans[]` 徽章）；三张输出卡：CSR=`csr_pem`（下载 .csr {CN 安全化}）；私钥 PKCS#8=`private_key_pem`（.key {CN}_private_key）；公钥 SPKI=`public_key_pem`（.pem {CN}_public_key）。文件名安全化：非 `[a-zA-Z0-9._-]`→`_`，空则 fallback。
- 调用：core `generate_csr(CsrGenerateRequest)`——字段 commonName/country/.../keyType/keySize/curve/sans（camelCase serde；直接构造 Rust struct）。异步（openssl 生成慢，走 core_rt）。

## PemToPfxConverter（950×750）
- 输入：PEM证书 FileUpload(text) accept `.pem,.crt,.cer`；私钥区**条件渲染**（仅当证书文本不含私钥块）FileUpload(text) accept `.key,.pem`；密码区：私钥密码（仅检测到加密私钥显示，password+显隐切换，label「私钥密码 *」）与「PFX密码 *」（password+切换，注释「用于保护生成的PFX文件」）。格式校验横条：证书含 `-----BEGIN CERTIFICATE-----` 对→绿「证书格式有效」；含私钥块→蓝「证书已包含私钥，无需额外输入」；私钥块→「私钥格式有效」；加密检测=`ENCRYPTED PRIVATE KEY` 头或 legacy 头含 ENCRYPTED。
- 头部 actions：`查看/隐藏OpenSSL命令`（展开 4 条参考命令代码块）。
- 按钮：`转换为PFX` 通栏 lg；disabled=processing||证书空||密码空||(证书无私钥且私钥空)。
- 输出：成功卡「转换成功！文件大小：{KB} KB」+「下载PFX文件」（save 默认 certificate.pfx，二进制写）。
- 调用：core `convert_pem_to_pfx(cert_pem, private_key_pem: Option<String>, password: String, private_key_password: Option<String>)`→`PfxConversionResult{pfx_data: Vec<u8>, success, error}` snake_case。私钥优先手输否则证书内提取。错误按关键词分派字段级（「证书」→证书区红字，「私钥」→私钥区红字，其余→表单级）；成功消息 3 秒自动消失。

## PfxToPemConverter（950×750）
- 输入：FileUpload(binary) accept `.pfx,.p12`（非该扩展名报「请选择PFX或P12格式的文件」）；已选显示「已选择： {文件名}」+红「清除」；PFX 密码 (可选) password+显隐切换。
- 头部 actions：`查看/隐藏OpenSSL命令`（4 条+💡说明）。
- 按钮：`开始转换` lg 通栏 disabled=loading||无文件。
- 输出：转换结果只读 textarea h-64 `combined_pem`；`复制结果`；`下载PEM`（save 默认原文件名换 .pem）。
- 调用：core `convert_pfx_to_pem(pfx_data: Vec<u8>, password: Option<String>)`→`{certificates[], private_keys[], combined_pem, success, error}` snake_case。success=false 报「转换失败」；密码错误→「密码错误或文件损坏…」；格式→「文件格式错误…」；未找到→三条排查提示。顶部「💡 使用提示」三条固定文案。

## RsaKeyGenerator（900×700）
- 输入：密钥长度 select（2048 推荐/3072/4096 默认 2048）；OpenSSH 注释（可选）placeholder「如 user@host，留空使用默认值」（Enter 触发）。
- 按钮：`生成密钥对`（loading「生成中…」）。
- 输出：两张卡（只读 textarea+格式 Tab 切换+复制+下载）：私钥卡 h-64（描述「泄露私钥将危及所有依赖它的系统…」）Tab PKCS#8=`private_pkcs8_pem`、PKCS#1=`private_pkcs1_pem`（.pem 默认名 rsa_private_key_{格式}.pem）；公钥卡 h-40（描述「OpenSSH 格式可直接粘贴到服务器的 authorized_keys 中」）Tab PEM=`public_spki_pem`、PKCS#1=`public_pkcs1_pem`（.pem）、OpenSSH=`public_openssh`（.pub 默认名 rsa_public_key_{格式}.{ext}）。Tab 切换按钮文案「下载 .{ext}」随之变化。
- 调用：core `generate_rsa_keypair(RsaGenerateRequest{key_size, comment: Option<String>})`→camelCase `{privatePkcs8Pem, privatePkcs1Pem, publicSpkiPem, publicPkcs1Pem, publicOpenssh}`。异步走 core_rt。
- 空态：「选择密钥长度后点击「生成密钥对」，生成的私钥请妥善保管」。

## RegexTester（1000×750）
- 左栏「配置与控制」：正则引擎 radio 5 选（Rust 默认/RE2/PCRE/Golang/JavaScript，值 rust/re2/pcre/golang/javascript）；操作模式 radio（测试匹配 test 默认/替换文本 replace）；正则表达式单行输入（font mono，语法错误红边框），label 右侧实时状态 ✓/✗——**300ms 防抖** invoke `validate_regex(pattern, engine)`→bool；test 模式有效时显示「找到 {match_count} 个匹配」；replace 模式追加「替换文本」输入+「替换所有匹配」checkbox 默认 true；标志 checkbox 2 列（snake_case）：忽略大小写 case_insensitive(i)/多行 multiline(m)/. 匹配换行 dot_matches_new_line(s)/非贪婪 swap_greed(U)/Unicode unicode(默认 true 其余 false)。
- 按钮：`测试匹配`/`执行替换`（随模式）；disabled=!pattern||!text||loading。
- 测试文本：大 textarea（占剩余），test 有效时叠加黄色高亮层（mark 匹配段，title=「匹配 N: start-end」）——GPUI 实现为结果列表替代高亮层可接受，但保留 mark 语义。
- 右栏输出：test：无效→红卡「正则表达式无效」+`error_message`；无匹配→灰卡「没有找到匹配项」；否则「匹配详情：」列表每项：匹配 N=`full_match`、位置 start-end、`groups[]`（「组 i:」值或「(未匹配)」）、`named_groups`（「{name}:」值或「(未匹配)」）。replace：绿条「完成 {replacement_count} 个替换」+「替换结果:」等宽块=`result`。
- 调用：core `test_regex(pattern, text, flags, engine)`→`{is_valid, error_message?, matches[], match_count}`；`replace_regex(pattern, text, replacement, flags, engine, replace_all)`→`{is_valid, error_message?, result?, replacement_count}` snake_case。手动按钮触发。
- 布局：左右双栏（h_resizable）。

## IpInfo（900×700）
- 输入：IP 输入框 placeholder「请输入IP地址，留空则查询当前IP」（Enter 查询）；IPv4/IPv6 正则校验失败报「请输入有效的IP地址」。
- 按钮：`查询`（primary，loading「查询中...」）；`显示原始 JSON/隐藏原始` 切换。
- 输出：loading 且无数据→骨架卡；结果多卡（每来源一张，标题「IP 信息 ({source})」）：行 label：IP 地址（带复制小图标）/国家/城市/地区/运营商/时区，空值 N/A。原始 JSON 区：格式化响应 maxHeight 300 +「复制全部」。
- 调用：core `query_ip_info(ip: Option<String>)`→camelCase `{infos: [{source, ip?, country?, city?, org?, region?, timezone?}], error?}`。**挂载即自动查询一次**（空 IP→查本机出口）。

## WhoisLookup（900×700，标题「域名 Whois 查询（多源）」）
- 输入：批量域名 textarea h-40 label「批量域名（每行一个或用逗号/空格分隔）」；按 `\s,;\n` 切分+校验+去重+小写化；查询源 select：自动 auto（默认）/whois_verisign/grs-whois.cndns.com(`whois_cndns`)/grs-whois.hichina.com(`whois_hichina`)。
- 按钮：`开始查询 ({N})`（running「查询中...」，disabled=running||无有效域名）；`导出 CSV`（绿，无结果禁用）；`导出 JSON`（无结果禁用）；`清空历史`。
- 输出：统计卡（总数/已完成/成功（成功率%）/平均响应 ms）；结果表格列：选择复选框、域名、来源、注册商、注册人、注册日期、过期日期、状态（`status[]` join '|'）、NS 数、NS 列表（前 3+…）、错误（红）；空值 —。勾选≥2 出现「结果对比」表（行 source/registrar/registrant/created/expires/updated/status/nameServers，列=所选域名）。「查询历史」区最近 20 条卡片。
- 调用：每域名 core `query_whois_unified(domain, source: Option<String>)`；auto 传 Some("auto")；**限流 700ms/源**；并发 6（tokio 任务，经 core_rt）；返回 camelCase `{domain, source, registrar?, registrant?, created?, expires?, updated?, status?, nameServers?, rawText?}`。缓存/历史：内存会话态（不持久化），历史保留 200 条内存、表格展示 20。CSV 导出含 BOM 列 domain,source,registrar,registrant,created,expires,updated,status,nameServers 文件名 whois_results_{ts}.csv。
- 底部固定提示：「频率限制：WHOIS 每 700ms（按服务器区分）。缓存有效期 24 小时。失败不记录历史。」→ GPUI 版本缓存为内存会话，文案改为「频率限制：WHOIS 每 700ms（按服务器区分）。查询历史保存在本次会话中。失败不记录历史。」

## JsonToGo（1100×800）
- 顶部配置行：结构体名称 text w-64（默认 `AutoGenerated`）；checkbox：导出字段=true、Go1.18+ any=true、Go1.24+ omitzero=false；标签 checkbox 组：json(true)/yaml/gorm/db/sql/toml/env/ini。
- 按钮：左编辑器头部「示例」（内置嵌套 JSON）与「清空」；无显式转换按钮。
- 输出：左右分栏（h_resizable）：左「JSON 输入」编辑器 json；右「Go 结构体输出」编辑器 go readOnly tabSize 4。
- 联动：**自动**——input 或配置变化后 500ms 防抖（input 空清输出）；core `convert_json_to_go(json_str: &str, options: JsonToGoOptions)`→String。错误红条「转换出错: 」+message。
