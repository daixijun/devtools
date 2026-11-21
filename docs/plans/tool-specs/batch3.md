# 批次 III：9 个复杂工具 UI 契约

> Rust 逻辑在 crates/core/src/tools/*。UI 直接调用 core 函数（core_rt 桥）。
> 返回字段名 = serde 字段名。registry id：sslchecker/imageconverter/imagecompressor/imagepreview/videoconverter/sqltogo/sqltoent/dnsresolver/settings

## 通用
- 文件对话框 rfd（后台线程）：pick_file/pick_files/pick_folder/save_file。
- 文件表格：用 gpui_component Table 或手工 div 表格均可（列头 + 行，行 hover）。缩略图用 gpui img（`gpui::img(ImageSource::Image(Arc<gpui::Image>))`，从 image crate 解码 RGBA 得到）或简单状态徽章替代（v1 允许）。
- 进度条：gpui_component progress 或自制 div（宽度百分比）。
- 对比滑块等复杂交互可降级为并排双图 + 说明（记录在代码注释）。

## SslChecker（950×750）
- 输入页：「域名:」单行输入 placeholder=`example.com` 前缀装饰 `https://`（灰只读块），Enter 触发；格式提示 + 示例。
- 最近记录：会话内存（最多 10 个小写域名 chip，点击直接检测，× 删除）。
- 按钮：`开始检测`（loading「检测中...」）；结果页 header `← 返回` 回输入页；`清除历史`。
- 输出：成功后整页切换为结果视图，6 个 Tab（gpui_component tab）：
  1. 概览：domain/server_ip（可复制）/server_info/security_score（≥90 绿「优秀」≥70 黄「良好」≥50 橙「一般」否则红「较差」，x/100）/ssl_labs_rating.grade（A+~F 分色）+score/100；「安全发现： N 项待处理」徽章（CRITICAL 红/HIGH 橙/其他黄）；ssl_versions 徽章（TLS1.0/1.1 标 ⚠️）；http2/http3/spdy 支持徽章；alpn 紫/证书徽章。
  2. 证书信息：行列表 subject/issuer/valid_from/valid_to/serial_number/fingerprint/signature_algorithm/public_key_algorithm/key_size(位)/san_domains 徽章，前 5 项可复制。
  3. 证书链：状态卡（chain_validation_status: valid/valid-with-warnings/其他 三色）；chain_length、is_complete、root_ca_info；chain_errors 红列表；链关系图（垂直节点+箭头：叶子/中间=黄/根=绿，徽章「叶子证书 (服务器)/中间CA证书/根证书/根证书（交叉签名形式）/自签名」，每节点主题/颁发者/有效期至/签名算法/密钥 N 位/验证错误，主题与指纹可复制）；总结（root_in_chain）。无 chain 时「未获取到证书链信息」。
  4. 加密套件：server_cipher_order「服务器优先/客户端优先」；按 protocol_support 分组徽章（TLS1.2/1.3 推荐、1.1 过时等）；每 CipherSuite 行 name/strength（高/中/弱分色）/server_order，可复制。
  5. CVE 漏洞：severity 过滤按钮组（全部/严重/高危/中危/低危+计数）；每条 severity 徽章、category、grade_cap「等级封顶 X」、name、affected_components、evidence「🔍 观测证据」、remediation、references、cve_ids；底部计数。无漏洞绿色「未发现可确认的安全问题」。
  6. 安全评估：ssl_labs_rating 四类分项（证书/协议/密钥交换/套件强度）+加权说明、applied_caps（finding→cap+reason）、vulnerabilities、recommendations。
- 调用：core `check_ssl_info(domain)`（异步 core_rt）。成功 toast「SSL 检测完成」；错误红条「检测失败: <msg>」；域名空「请输入域名」格式错「请输入有效的域名格式」。

## ImageConverter（900×700）
- 转换设置卡：源格式 select（auto 自动检测+jpg/jpeg/png/gif/bmp/tiff/webp/ico/heic）/目标格式 select（9 项不含 auto 默认 jpg）/质量 N% range 1-100 默认 85（仅 jpg/jpeg/webp 显示）/尺寸调整 radio（none 保持原尺寸/width/height/both，选后显示宽或高数字输入，默认 1920/1080，1-10000）/移除 EXIF checkbox（红字「保护隐私」）/转换成功后删除源文件 checkbox（红字「谨慎操作」）/指定输出目录 checkbox（勾选显示只读框+`选择目录`+`清除`）。
- 文件添加：「添加文件」pick_files（filter 按源格式）、「添加目录」pick_folder 递归过滤；逐个调 core `get_image_info_command(input_path)` 填尺寸/EXIF；按 path 去重。
- 表格列：路径信息（源 path 点击复制 + outputPath「未转换」）/大小/尺寸/EXIF（「包含」橙 |「无」）/状态（等待中/转换中/已完成/错误）/错误（截断 30 字）/操作（删除）。
- 按钮：`开始转换`（逐个调 `convert_image(request)`，request 字段：input_path/output_path/target_format/quality（仅 jpg/jpeg/webp）/width/height/remove_exif/delete_source_file）disabled=converting||空||(自定义目录未选)；`清空列表`。设置项 disabled=converting。
- 进度：进度条「总体进度: i+1 / n」；完成绿「✅ 批量转换完成：成功 N 个文件」或红「…M 个失败」。

## ImageCompressor（1100×820）
- 压缩设置：模式 radio lossless（无损优化）/quality（质量压缩，默认）；输出格式 select keep「保持原格式」/png/jpeg/webp；质量 N% 默认 80（mode=quality 显示）；PNG 无损级别 select 1-6 默认 3（输出含 png 显示）；等比缩放 最大宽度/最大高度 数字（空=原尺寸）；移除 EXIF checkbox **默认 true**；输出位置 radio 自动（源同级 `compressed/` 子目录）vs 自定义目录。
- 文件添加同 ImageConverter（白名单 jpg/jpeg/png/gif/bmp/tiff/tif/webp）。
- 表格列：预览（48×48 缩略图，懒加载 `read_image_data_url(path, maxDimension=128)` 返回 `{data_url,width,height}`，失败 N/A；GPUI 可解码 data_url 显示或降级为格式徽章）/文件名（点击复制 path）/原始大小/压缩后（compressed_size+节省百分比）/尺寸（原 → output_width x output_height）/状态/操作（删除）。
- 按钮：`开始压缩`（逐个 `compress_image(request)`：input_path/output_path/mode/quality/max_width/max_height/output_format/remove_exif/png_optimize_level）；`清空列表`。
- 完成统计卡 4 格：已压缩/原始合计/压缩后/节省空间（+pct% 绿）；对比弹窗可降级为并排展示（原 size/新 size/-N%）。

## ImagePreview（900×700）纯本地
- 输入：URL 输入框（placeholder「粘贴图片 URL 或 data: URL...」）+`加载`按钮；「上传文件」pick_file 读 bytes；支持 Cmd+V 粘贴图片（gpui on_clipboard? 可降级为仅文件加载，注释说明）。
- 预览区：img 显示；滚轮缩放 0.5-5、拖拽平移、双击 1×/2×；浮动按钮 `重置`/`复制 URL`；缩放指示器「− N% +」。全屏遮罩可简化（记录注释）。
- 信息面板：尺寸 W × H/格式（大写）/大小（估算）；保存设置：格式 select png/jpeg/gif/webp/质量（jpeg/webp 默认 85）/文件名 input 默认 `image`/`保存图片到指定目录`（save_file + 写文件；格式转换用 image crate 重编码）。
- 空态：「粘贴图片或输入 URL 开始预览」。

## VideoConverter（900×700）
- 前置：挂载即调 core `check_ffmpeg_available()`；false→顶部红横幅「❌ FFmpeg 未安装，无法使用视频转换功能」+安装方法（macOS `brew install ffmpeg` 等）+所有控件 disabled。
- 输入：源格式 select（auto+mp4/mov/avi/mkv/wmv/flv/webm/m4v/3gp/mpeg）/目标格式 select（默认 mp4）/指定输出目录 checkbox+选择/删除源文件 checkbox（红字警告「⚠️ 转换成功后自动删除源文件，此操作不可恢复」）。
- 文件添加：「添加文件」多选、「添加目录」递归；逐个 `get_video_info(input_path)`→`{name,size,format,duration,resolution,path}` 填时长/分辨率。
- 表格列：路径信息（源/转换后点击复制）/大小/时长/分辨率/状态/错误（30 字）/操作（删除 confirm→直接删，注释说明）。
- 按钮：`开始转换`（逐个 `convert_video(request{input_path,output_path,delete_source_file})`）；`清空列表`。进度「总体进度: i+1 / n」。完成文案同图片转换。

## SqlToGo（1100×800）不用 ToolLayout
- 顶部配置条 + 左右分栏：复数表名 checkbox=true；导出字段=true；Go1.24+=true；JSON 处理 select（none「无」/omitempty 默认/omitzero 仅 Go1.24+ 出现，未勾选且当前=omitzero 回退 omitempty）；标签 checkbox 组 json:true gorm:true db:false sql:false yaml:false toml:false env:false ini:false。
- 输入编辑器 sql；输出只读 go。「示例」（users+posts 两表 DDL 含外键）「清空」。
- 联动：500ms 防抖自动调 core `convert_sql_to_go(sql, options: SqlToGoOptions)`→`GoStructOutput{outputs: HashMap<String,String>, table_names}`；**仅显示第一个表的代码**。错误仅清输出（红条可选）。

## SqlToEnt（1100×800）
- 配置区 + 分栏：复数表名=true；生成边缘关系=true；生成 Mixin=true；生成 Hooks=false；生成 Policy=false；UUID 主键=false；软删除=false；包名 text 默认 `schema`。
- 「示例」「清空」。500ms 防抖自动调 core `convert_sql_to_ent(sql, options)`→`EntSchemaOutput{outputs, table_names}`；输出侧按 tableNames 生成 **Tab 页（标题 `<表名>.go`）**，切换显示 outputs[tableName]，只读 go 编辑器 tabSize 2。出错红条显示 err 或「转换失败」。

## DnsResolver（900×700）
- Tab：「正向解析 / 反向解析」。
- 正向（左配置右结果）：DNS 服务器多选 checkbox——挂载调 core `get_dns_servers()`（`{name,server}`），**默认全选**，label `name (server)`；「添加自定义 DNS 服务器」输入（placeholder「例如: 9.9.9.9」Enter 添加）+`添加`按钮；记录类型 select：ALL 全部记录(默认)/A/AAAA/MX/TXT/CNAME/NS/SOA（中文 label：A 记录 (IPv4) 等）；域名输入（placeholder「例如: www.example.com」）。按钮：`查询`（loading「查询中...」）/`示例`（www.example.com/ALL）/`清空`。
  - 结果：域名；每 results 分组卡：服务器 label+`latencyMs`「N ms」徽章；组错误红字；records 表格列 类型/值/TTL（null 显示 -，值 mono）；无记录「未找到记录」；顶部 error 红字；ipInfo 卡「IP 地理位置信息」2 列（IP/国家/城市/运营商/地区/时区缺失 -）+「数据来源: source」。
- 反向：单 IP（输入 placeholder「例如: 8.8.8.8」+`查询` `reverse_dns_lookup(ip)`；结果 IP+domains 列表或「未找到对应的域名记录」）+批量（textarea 6 行 placeholder「输入多个 IP 地址，每行一个」+`批量查询` `batch_reverse_dns_lookup(ips)`；结果每 IP 卡+域名列表）。示例（8.8.8.8/114.114.114.114/1.1.1.1）+清空。错误红条「查询失败： msg」等。
- 返回字段全 camelCase（serde rename_all）：`DnsLookupResponse{domain, results[], ipInfo?, error?}`、`DnsServerLookupResult{dnsServer, records[], latencyMs?, error?}`、`DnsRecord{recordType, name, value, ttl?}`、`DnsServerEntry{name, server}`、`IpGeolocationInfo{ip,country,city,org,region,timezone,source}`、`ReverseDnsResponse{ip,domains[],error?}`、`BatchReverseDnsResponse{results[]}`。

## Settings（900×700）
- 状态加载：挂载并行调 core？不——托盘/快捷键/自启是壳层能力：
  - 托盘开关：壳层 tray 模块（crates/app/src/tray.rs）——托盘开关需要**运行时重建/移除托盘**。tray.rs 当前 init 一次性；Settings 通过 `crate::tray::set_enabled(enabled, cx)` 立即生效并返回实际 bool。
  - 自启：`crate::autostart::set(enabled)` / `status()`（已实现，去掉 dead_code allow）。
  - 启动时最小化/关闭最小化到托盘：写 core::config::AppConfig（set_start_minimized/set_close_to_tray）。
  - 快捷键设置：显示当前组合（⌥/Alt/⌃/Ctrl/⌘ + key，Space→「空格」，缺省 ⌥+空格）；`修改`按钮（录制中「按下新组合键...」橙色，10 秒超时）；录制中捕获 keydown：无修饰键或单独修饰键忽略；空格存 `Space`；确认后 `crate::hotkey::re_register(config, cx)`（需新增：Global 保存 manager 句柄，unregister+register 新键；失败提示「快捷键注册失败: <err>. 请检查快捷键是否已被其他应用占用。」但仍保存配置）。
  - 主题：三按钮组 ☀️ 浅色/🌙 深色/🖥️ 跟随系统；点击写 config.theme + `Theme::change`/`sync_system_appearance` 全窗口即时生效（GPUI 天然全局，无需事件广播）。
- 系统设置 4 开关（Switch，disabled=loading）：显示系统托盘图标→toggle_tray；开机自动启动→set_autostart；启动时最小化到托盘→set_start_minimized（托盘关时仅存配置；disabled=loading||!showTray）；关闭时最小化到托盘→set_close_to_tray（同）。
- 关于应用卡：名称「开发者工具箱」/版本/技术栈「GPUI + Rust」。
- 每项失败回滚 UI 状态 + toast_error。
