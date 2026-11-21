//! 密码加密与验证工具。
//!
//! Tab「密码加密 / 密码验证」；算法 bcrypt / PBKDF2 / SHA-256 / SHA-512 / MD5。
//! bcrypt 与 PBKDF2 计算较慢（bcrypt rounds=10 约「秒级」），经 core_rt 后台
//! 线程执行、按钮进入 loading 态，完成后 spawn_in 回填结果；MD5/SHA 系列
//! 为同步 hex 计算。PBKDF2 迭代次数两页共享同一 state。
//! 契约见 docs/plans/tool-specs/batch1.md。

use futures::StreamExt as _;
use gpui::{
    div, prelude::FluentBuilder as _, px, App, AppContext as _, Context, Div, Entity, FontWeight,
    InteractiveElement as _, IntoElement, ParentElement as _, Render,
    StatefulInteractiveElement as _, Styled as _, Window,
};
use gpui_component::input::{Input, InputState, NumberInput, TextareaState};
use gpui_component::{ActiveTheme as _, Disableable as _};

use crate::components::ui;

/// 可选算法（展示顺序为历史契约顺序）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum HashAlgorithm {
    Bcrypt,
    Pbkdf2,
    Sha256,
    Sha512,
    Md5,
}

const ALGORITHMS: [HashAlgorithm; 5] = [
    HashAlgorithm::Bcrypt,
    HashAlgorithm::Pbkdf2,
    HashAlgorithm::Sha256,
    HashAlgorithm::Sha512,
    HashAlgorithm::Md5,
];

/// bcrypt 盐值轮数合法范围。
const ROUNDS_RANGE: std::ops::RangeInclusive<u32> = 4..=20;
/// PBKDF2 迭代次数合法范围。
const ITERATIONS_RANGE: std::ops::RangeInclusive<u32> = 1000..=1_000_000;

impl HashAlgorithm {
    /// 展示名。
    fn display(self) -> &'static str {
        match self {
            HashAlgorithm::Bcrypt => "bcrypt",
            HashAlgorithm::Pbkdf2 => "PBKDF2",
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha512 => "SHA-512",
            HashAlgorithm::Md5 => "MD5",
        }
    }

    /// 算法说明（「当前算法」说明卡）。
    fn description(self) -> &'static str {
        match self {
            HashAlgorithm::Bcrypt => "业界标准的密码哈希算法，内置盐值和可调节的计算成本",
            HashAlgorithm::Pbkdf2 => "基于密码的密钥派生函数，使用 HMAC-SHA256",
            HashAlgorithm::Sha256 => "256位安全哈希算法，单向哈希函数",
            HashAlgorithm::Sha512 => "512位安全哈希算法，比SHA-256更长的输出",
            HashAlgorithm::Md5 => "已被破解的哈希算法，仅供兼容性使用",
        }
    }

    /// 使用场景（算法卡描述 + 说明卡）。
    fn use_case(self) -> &'static str {
        match self {
            HashAlgorithm::Bcrypt => "推荐用于密码存储",
            HashAlgorithm::Pbkdf2 => "推荐用于密码存储和密钥派生",
            HashAlgorithm::Sha256 => "仅用于数据完整性校验，不推荐用于密码存储",
            HashAlgorithm::Sha512 => "仅用于数据完整性校验，不推荐用于密码存储",
            HashAlgorithm::Md5 => "不安全，仅用于非安全场景的数据校验",
        }
    }

    /// 是否为推荐的密码哈希算法（「安全/不安全」徽标）。
    fn secure(self) -> bool {
        matches!(self, HashAlgorithm::Bcrypt | HashAlgorithm::Pbkdf2)
    }

    /// 用于拼接元素 id。
    fn element_key(self) -> &'static str {
        match self {
            HashAlgorithm::Bcrypt => "bcrypt",
            HashAlgorithm::Pbkdf2 => "pbkdf2",
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Sha512 => "sha512",
            HashAlgorithm::Md5 => "md5",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Hash,
    Verify,
}

/// 验证页结果：`is_valid` Some(true)=绿 / Some(false)=红 / None=提示（蓝）。
struct VerifyOutcome {
    is_valid: Option<bool>,
    message: String,
}

pub struct PasswordHasher {
    active_tab: Tab,
    // —— 加密页 ——
    password: Entity<InputState>,
    salt_rounds: Entity<InputState>,
    algorithm: HashAlgorithm,
    /// 加密结果或校验错误文案（复用同一区域展示，故用 Option<String>）。
    hash_result: Option<String>,
    is_hashing: bool,
    // —— 验证页 ——
    verify_password: Entity<InputState>,
    verify_hash: Entity<TextareaState>,
    verify_algorithm: HashAlgorithm,
    verify_result: Option<VerifyOutcome>,
    is_verifying: bool,
    // —— PBKDF2 迭代次数（两页共享同一 state）——
    iterations: Entity<InputState>,
}

impl PasswordHasher {
    pub fn create_view(window: &mut Window, cx: &mut App) -> gpui::AnyView {
        cx.new(|cx| Self::new(window, cx)).into()
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("请输入要加密的密码...")
                .masked(true)
        });
        let salt_rounds = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入盐值轮数 (4-20)")
                .default_value("10")
                .min(4.)
                .max(20.)
                .step(1.)
        });
        let iterations = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("输入迭代次数 (1000-1000000)")
                .default_value("10000")
                .min(1000.)
                .max(1_000_000.)
                .step(1000.)
        });
        let verify_password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("请输入要验证的密码...")
                .masked(true)
        });
        let verify_hash =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("请输入要验证的哈希值..."));

        Self {
            active_tab: Tab::Hash,
            password,
            salt_rounds,
            algorithm: HashAlgorithm::Bcrypt,
            hash_result: None,
            is_hashing: false,
            verify_password,
            verify_hash,
            verify_algorithm: HashAlgorithm::Bcrypt,
            verify_result: None,
            is_verifying: false,
            iterations,
        }
    }

    // ------------------------------------------------------------------
    // 参数读取
    // ------------------------------------------------------------------

    /// 解析盐值轮数输入（无法解析视为越界，走统一错误文案）。
    fn salt_rounds_value(&self, cx: &Context<Self>) -> Option<u32> {
        self.salt_rounds.read(cx).value().trim().parse::<u32>().ok()
    }

    /// 解析共享的 PBKDF2 迭代次数输入。
    fn iterations_value(&self, cx: &Context<Self>) -> Option<u32> {
        self.iterations.read(cx).value().trim().parse::<u32>().ok()
    }

    // ------------------------------------------------------------------
    // 加密
    // ------------------------------------------------------------------

    fn hash_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let password = self.password.read(cx).value().to_string();
        if password.is_empty() {
            self.hash_result = Some("请输入密码".to_string());
            cx.notify();
            return;
        }

        match self.algorithm {
            HashAlgorithm::Bcrypt => {
                // bcrypt 计算慢（rounds 10 约秒级）→ core_rt 后台执行
                let rounds = match self.salt_rounds_value(cx) {
                    Some(r) if ROUNDS_RANGE.contains(&r) => r,
                    _ => {
                        self.hash_result = Some("盐值轮数必须在 4-20 之间".to_string());
                        cx.notify();
                        return;
                    }
                };
                self.is_hashing = true;
                cx.notify();
                let rx = crate::core_rt::submit(cx, async move {
                    devtools_core::crypto::bcrypt_hash(&password, rounds)
                });
                self.spawn_hash_result(rx, window, cx);
            }
            HashAlgorithm::Pbkdf2 => {
                let iterations = match self.iterations_value(cx) {
                    Some(n) if ITERATIONS_RANGE.contains(&n) => n,
                    _ => {
                        self.hash_result = Some("迭代次数必须在 1000-1000000 之间".to_string());
                        cx.notify();
                        return;
                    }
                };
                self.is_hashing = true;
                cx.notify();
                // 盐固定 'salt'、dkLen 32（历史契约值）
                let rx = crate::core_rt::submit(cx, async move {
                    Ok::<String, String>(devtools_core::crypto::pbkdf2_hex(
                        &password, "salt", iterations, 32,
                    ))
                });
                self.spawn_hash_result(rx, window, cx);
            }
            // MD5 / SHA 系列纯哈希：直接同步计算
            HashAlgorithm::Sha256 => {
                self.hash_result = Some(devtools_core::crypto::sha256_hex(&password));
                cx.notify();
            }
            HashAlgorithm::Sha512 => {
                self.hash_result = Some(devtools_core::crypto::sha512_hex(&password));
                cx.notify();
            }
            HashAlgorithm::Md5 => {
                self.hash_result = Some(devtools_core::crypto::md5_hex(&password));
                cx.notify();
            }
        }
    }

    /// 后台任务回流：加密完成（成功或 core 报错）回填结果区。
    fn spawn_hash_result(
        &self,
        mut rx: futures::channel::mpsc::UnboundedReceiver<Result<String, String>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            if let Some(result) = rx.next().await {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.on_hash_result(result, window, cx)
                });
            }
        })
        .detach();
    }

    fn on_hash_result(
        &mut self,
        result: Result<String, String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_hashing = false;
        self.hash_result = Some(match result {
            Ok(hash) => hash,
            Err(msg) => format!("加密过程中发生错误: {msg}"),
        });
        cx.notify();
    }

    /// 加密区重置（密码/结果清空、算法与两个参数复位）。
    fn reset_hash(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.password
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.salt_rounds
            .update(cx, |state, cx| state.set_value("10", window, cx));
        self.iterations
            .update(cx, |state, cx| state.set_value("10000", window, cx));
        self.algorithm = HashAlgorithm::Bcrypt;
        self.hash_result = None;
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 验证
    // ------------------------------------------------------------------

    fn verify_password(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let password = self.verify_password.read(cx).value().to_string();
        let hash = self.verify_hash.read(cx).value().to_string();
        if password.is_empty() {
            self.verify_result = Some(VerifyOutcome {
                is_valid: None,
                message: "请输入要验证的密码".to_string(),
            });
            cx.notify();
            return;
        }
        if hash.is_empty() {
            self.verify_result = Some(VerifyOutcome {
                is_valid: None,
                message: "请输入哈希值".to_string(),
            });
            cx.notify();
            return;
        }

        match self.verify_algorithm {
            HashAlgorithm::Bcrypt => {
                self.is_verifying = true;
                cx.notify();
                let rx = crate::core_rt::submit(cx, async move {
                    devtools_core::crypto::bcrypt_verify(&password, &hash)
                });
                self.spawn_verify_result(rx, window, cx);
            }
            HashAlgorithm::Pbkdf2 => {
                // 与加密页共享同一迭代次数输入；越界走提示横幅
                let iterations = match self.iterations_value(cx) {
                    Some(n) if ITERATIONS_RANGE.contains(&n) => n,
                    _ => {
                        self.verify_result = Some(VerifyOutcome {
                            is_valid: None,
                            message: "迭代次数必须在 1000-1000000 之间".to_string(),
                        });
                        cx.notify();
                        return;
                    }
                };
                self.is_verifying = true;
                cx.notify();
                // 同盐重算比对（盐固定 'salt'、dkLen 32）
                let rx = crate::core_rt::submit(cx, async move {
                    devtools_core::crypto::pbkdf2_hex(&password, "salt", iterations, 32) == hash
                });
                self.spawn_verify_result(rx, window, cx);
            }
            // MD5 / SHA 系列：重算严格相等
            HashAlgorithm::Sha256 => {
                self.finish_verify(devtools_core::crypto::sha256_hex(&password) == hash, cx);
            }
            HashAlgorithm::Sha512 => {
                self.finish_verify(devtools_core::crypto::sha512_hex(&password) == hash, cx);
            }
            HashAlgorithm::Md5 => {
                self.finish_verify(devtools_core::crypto::md5_hex(&password) == hash, cx);
            }
        }
    }

    /// 后台任务回流：bcrypt / PBKDF2 校验完成。
    fn spawn_verify_result(
        &self,
        mut rx: futures::channel::mpsc::UnboundedReceiver<bool>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn_in(window, async move |this, cx| {
            if let Some(is_valid) = rx.next().await {
                let _ = this.update_in(cx, |this, window, cx| {
                    this.on_verify_result(is_valid, window, cx)
                });
            }
        })
        .detach();
    }

    fn on_verify_result(&mut self, is_valid: bool, _window: &mut Window, cx: &mut Context<Self>) {
        self.is_verifying = false;
        self.finish_verify(is_valid, cx);
    }

    fn finish_verify(&mut self, is_valid: bool, cx: &mut Context<Self>) {
        let message = if is_valid {
            "✅ 密码验证成功！"
        } else {
            "❌ 密码验证失败！"
        };
        self.verify_result = Some(VerifyOutcome {
            is_valid: Some(is_valid),
            message: message.to_string(),
        });
        cx.notify();
    }

    /// 验证区重置（密码/哈希/结果清空、验证算法复位；不动共享迭代次数）。
    fn reset_verify(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.verify_password
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.verify_hash
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.verify_algorithm = HashAlgorithm::Bcrypt;
        self.verify_result = None;
        cx.notify();
    }

    // ------------------------------------------------------------------
    // 渲染
    // ------------------------------------------------------------------

    fn set_algorithm(&mut self, verify: bool, alg: HashAlgorithm, cx: &mut Context<Self>) {
        if verify {
            if self.verify_algorithm != alg {
                self.verify_algorithm = alg;
                cx.notify();
            }
        } else if self.algorithm != alg {
            self.algorithm = alg;
            cx.notify();
        }
    }

    /// Tab 导航（自制按钮组，选中项 primary 下划线高亮）。
    fn render_tabs(&self, cx: &mut Context<Self>) -> gpui::Stateful<gpui::Div> {
        let theme = cx.theme().clone();
        let active_color = theme.colors.primary;
        let inactive_color = theme.colors.muted_foreground;
        let hover_color = theme.colors.foreground;
        let active = self.active_tab;
        div()
            .id("passwordhasher-tabs")
            .flex()
            .gap_2()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(Self::tab_button(
                active,
                "tab-hash",
                "密码加密",
                Tab::Hash,
                active_color,
                inactive_color,
                hover_color,
                cx,
            ))
            .child(Self::tab_button(
                active,
                "tab-verify",
                "密码验证",
                Tab::Verify,
                active_color,
                inactive_color,
                hover_color,
                cx,
            ))
    }

    #[allow(clippy::too_many_arguments)]
    fn tab_button(
        active: Tab,
        id: &'static str,
        label: &'static str,
        tab: Tab,
        active_color: gpui::Hsla,
        inactive_color: gpui::Hsla,
        hover_color: gpui::Hsla,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let is_active = active == tab;
        let (fg, border) = if is_active {
            (active_color, active_color)
        } else {
            (inactive_color, gpui::transparent_black())
        };
        div()
            .id(id)
            .px_3()
            .py_2()
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .border_b_2()
            .text_color(fg)
            .border_color(border)
            .hover(move |s| s.text_color(if is_active { active_color } else { hover_color }))
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.active_tab != tab {
                    this.active_tab = tab;
                    cx.notify();
                }
            }))
    }

    /// 算法选择卡片区（5 张可选卡片，选中高亮；`page` 用于区分两页的元素 id）。
    fn render_algorithm_section(
        &self,
        page: &'static str,
        selected: HashAlgorithm,
        verify: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let mut grid = div().grid().grid_cols(3).gap_2();
        for alg in ALGORITHMS {
            let id = format!("{page}-alg-{}", alg.element_key());
            grid = grid.child(self.algorithm_card(&id, alg, selected, verify, cx));
        }
        ui::card(cx)
            .child(ui::card_header("选择加密算法", cx))
            .child(grid)
    }

    fn algorithm_card(
        &self,
        id: &str,
        alg: HashAlgorithm,
        selected: HashAlgorithm,
        verify: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let theme = cx.theme().clone();
        let is_selected = alg == selected;
        let (border, bg) = if is_selected {
            (theme.colors.primary, theme.colors.primary.opacity(0.08))
        } else {
            (theme.colors.border, gpui::transparent_black())
        };
        let hover_border = theme.colors.muted_foreground.opacity(0.6);
        let fg = theme.colors.foreground;
        let muted_fg = theme.colors.muted_foreground;
        let badge_color = if alg.secure() {
            theme.colors.success
        } else {
            theme.colors.warning
        };
        let badge_text = if alg.secure() { "安全" } else { "不安全" };

        div()
            .id(id.to_string())
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded(px(6.))
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_left()
            .hover(move |s| {
                if is_selected {
                    s
                } else {
                    s.border_color(hover_border)
                }
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_algorithm(verify, alg, cx);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(fg)
                            .child(alg.display()),
                    )
                    .child(ui::badge(badge_text, badge_color)),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(muted_fg)
                    .child(alg.use_case()),
            )
    }

    /// 加密页：算法卡 + 参数卡 + 结果卡 + 当前算法说明卡。
    fn render_hash_page(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme().clone();
        let alg = self.algorithm;

        let mut params_card = ui::card(cx)
            .child(ui::card_header("加密参数", cx))
            .child(ui::field("输入密码", Input::new(&self.password), cx));
        if alg == HashAlgorithm::Bcrypt {
            params_card = params_card
                .child(ui::field(
                    "盐值轮数 (Salt Rounds)",
                    NumberInput::new(&self.salt_rounds).w_full(),
                    cx,
                ))
                .child(ui::hint_text(
                    "盐值轮数越高，安全性越好，但加密时间越长。推荐值为 10-12。范围：4 - 20",
                    cx,
                ));
        } else if alg == HashAlgorithm::Pbkdf2 {
            params_card = params_card
                .child(ui::field(
                    "迭代次数 (Iterations)",
                    NumberInput::new(&self.iterations).w_full(),
                    cx,
                ))
                .child(ui::hint_text(
                    "迭代次数越多，安全性越好，但加密时间越长。推荐值为 10,000 以上。范围：1,000 - 1,000,000",
                    cx,
                ));
        }
        params_card = params_card.child(
            div()
                .flex()
                .gap_3()
                .child(
                    ui::primary_button(
                        "hash-go",
                        if self.is_hashing {
                            "加密中..."
                        } else {
                            "加密密码"
                        },
                        cx.listener(|this, _, window, cx| this.hash_password(window, cx)),
                    )
                    .flex_1()
                    .loading(self.is_hashing)
                    .disabled(self.is_hashing),
                )
                .child(
                    ui::secondary_button(
                        "hash-reset",
                        "重置",
                        cx.listener(|this, _, window, cx| this.reset_hash(window, cx)),
                    )
                    .flex_1(),
                ),
        );

        let mut page = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.render_algorithm_section("hash", alg, false, cx))
            .child(params_card);

        // 加密结果卡（错误文案也写进同一结果区，此时隐藏复制按钮）
        if let Some(result) = self.hash_result.clone() {
            let copyable = hash_copyable(&result);
            page = page.child(
                ui::card(cx)
                    .child(ui::card_header("加密结果", cx).when(copyable, |h| {
                        h.child(ui::copy_button("hash-copy", result.clone()))
                    }))
                    .child(ui::mono_block("mono-password-result", result.clone(), cx))
                    .child(ui::hint_text(
                        &format!(
                            "算法: {} | 哈希长度: {} 字符",
                            alg.display(),
                            result.chars().count()
                        ),
                        cx,
                    )),
            );
        }

        // 当前算法说明卡
        page = page.child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .p_3()
                .rounded(px(6.))
                .border_1()
                .border_color(theme.colors.primary.opacity(0.3))
                .bg(theme.colors.primary.opacity(0.08))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.primary)
                        .child(format!("当前算法: {}", alg.display())),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.colors.foreground)
                        .child(alg.description()),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(theme.colors.muted_foreground)
                        .child(format!("使用场景: {}", alg.use_case())),
                ),
        );
        page
    }

    /// 验证页：算法卡 + 参数卡 + 结果横幅 + 安全提示。
    fn render_verify_page(&self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme().clone();
        let alg = self.verify_algorithm;

        let mut params_card = ui::card(cx)
            .child(ui::card_header("验证参数", cx))
            .child(ui::field(
                "输入要验证的密码",
                Input::new(&self.verify_password),
                cx,
            ))
            .child(ui::field(
                "输入哈希值",
                ui::edit_area(&self.verify_hash, 72.),
                cx,
            ));
        if alg == HashAlgorithm::Bcrypt {
            // 固定说明框：bcrypt 轮数包含在哈希值中
            params_card = params_card.child(
                div()
                    .p_3()
                    .rounded(px(6.))
                    .border_1()
                    .border_color(theme.colors.border)
                    .bg(theme.colors.muted.opacity(0.4))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(theme.colors.muted_foreground)
                            .child("bcrypt 的盐值轮数已包含在哈希值中，验证时无需手动设置"),
                    ),
            );
        } else if alg == HashAlgorithm::Pbkdf2 {
            params_card = params_card
                .child(ui::field(
                    "迭代次数 (Iterations)",
                    NumberInput::new(&self.iterations).w_full(),
                    cx,
                ))
                .child(ui::hint_text(
                    "验证时需要使用与加密时相同的迭代次数。范围：1,000 - 1,000,000",
                    cx,
                ));
        }
        params_card = params_card.child(
            div()
                .flex()
                .gap_3()
                .child(
                    ui::primary_button(
                        "verify-go",
                        if self.is_verifying {
                            "验证中..."
                        } else {
                            "验证密码"
                        },
                        cx.listener(|this, _, window, cx| this.verify_password(window, cx)),
                    )
                    .flex_1()
                    .loading(self.is_verifying)
                    .disabled(self.is_verifying),
                )
                .child(
                    ui::secondary_button(
                        "verify-reset",
                        "重置",
                        cx.listener(|this, _, window, cx| this.reset_verify(window, cx)),
                    )
                    .flex_1(),
                ),
        );

        let mut page = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.render_algorithm_section("verify", alg, true, cx))
            .child(params_card);

        // 验证结果横幅（绿/红/提示）
        if let Some(outcome) = &self.verify_result {
            let banner = match outcome.is_valid {
                Some(true) => ui::success_banner(&outcome.message, cx),
                Some(false) => ui::error_banner(&outcome.message, cx),
                None => ui::info_banner(&outcome.message, cx),
            };
            page = page.child(
                ui::card(cx)
                    .child(ui::card_header("验证结果", cx))
                    .child(banner),
            );
        }

        // 安全提示（4 条）
        let bullet_color = theme.colors.warning;
        let mut tips = div().flex().flex_col().gap_1();
        for tip in SAFETY_TIPS {
            tips = tips.child(
                div()
                    .text_size(px(11.))
                    .text_color(bullet_color)
                    .child(format!("• {tip}")),
            );
        }
        page = page.child(
            div()
                .flex()
                .flex_col()
                .gap_2()
                .p_3()
                .rounded(px(6.))
                .border_1()
                .border_color(theme.colors.warning.opacity(0.3))
                .bg(theme.colors.warning.opacity(0.08))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_size(px(12.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.colors.warning)
                        .child("⚠️ 安全提示"),
                )
                .child(tips),
        );
        page
    }
}

/// 安全提示文案（与历史版本一致）。
const SAFETY_TIPS: [&str; 4] = [
    "不同的哈希值需要使用对应的算法进行验证",
    "bcrypt 每次加密会产生不同的哈希值（内置随机盐值）",
    "MD5 和 SHA 系列算法不推荐用于密码存储，仅用于数据完整性校验",
    "推荐使用 bcrypt 或 PBKDF2 算法进行密码存储",
];

/// 结果是否为真实哈希（按错误文案前缀禁用复制按钮）。
fn hash_copyable(result: &str) -> bool {
    !(result.starts_with("请输入密码")
        || result.starts_with("盐值轮数必须在")
        || result.starts_with("迭代次数必须在")
        || result.starts_with("加密过程中发生错误"))
}

impl Render for PasswordHasher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        ui::tool_page("passwordhasher")
            .child(ui::themed_header(
                "密码加密与验证工具",
                "支持多种加密算法的密码哈希与验证",
                cx,
            ))
            .child(self.render_tabs(cx))
            .child(match self.active_tab {
                Tab::Hash => self.render_hash_page(cx),
                Tab::Verify => self.render_verify_page(cx),
            })
    }
}
