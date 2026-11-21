# Makefile for Devtools (GPUI 纯 Rust 桌面应用)
# 管理开发、构建、清理、检查等常用工作流
#
# 用法:
#   make help           # 显示所有可用命令
#   make run            # 运行桌面应用 (debug; 装有 cargo-watch 时文件变化自动重编译重启)
#   make run-direct     # 运行桌面应用 (debug, 不启用 watch)
#   make check          # 全量检查 (check + clippy + test)
#   make app            # 打包 DevTools.app
#   make clean          # 清理构建产物

# --- 可执行程序 -----------------------------------------------------------
CARGO := cargo

# --- 路径 -----------------------------------------------------------------
ROOT_DIR := $(CURDIR)
TARGET_DIR := $(ROOT_DIR)/target

# --- 颜色输出 -------------------------------------------------------------
COLOR_RESET  := \033[0m
COLOR_BOLD   := \033[1m
COLOR_GREEN  := \033[32m
COLOR_CYAN   := \033[36m
COLOR_YELLOW := \033[33m

.DEFAULT_GOAL := help

.PHONY: help run run-direct run-release check clippy test fmt app \
        cargo-check clean clean-all info

# =========================================================================
# 帮助
# =========================================================================
help: ## 显示此帮助信息
	@printf "$(COLOR_BOLD)Devtools Makefile$(COLOR_RESET)\n"
	@printf "用法: $(COLOR_CYAN)make <target>$(COLOR_RESET)\n\n"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / { printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@printf "\n详见 $(COLOR_YELLOW)AGENTS.md$(COLOR_RESET) 了解开发约定。\n"

# =========================================================================
# 开发
# =========================================================================
run: ## 运行桌面应用 (debug; 装有 cargo-watch 时 watch 模式, 文件变化自动重启)
	@printf "$(COLOR_CYAN)▶ 运行 devtools-app...$(COLOR_RESET)\n"
	@if command -v cargo-watch >/dev/null 2>&1; then \
		printf "$(COLOR_CYAN)▶ watch 模式: crates/、Cargo.toml 或 Cargo.lock 变化自动重编译并重启 (Ctrl+C 退出)$(COLOR_RESET)\n"; \
		$(CARGO) watch -w crates -w Cargo.toml -w Cargo.lock -x "run -p devtools-app"; \
	else \
		printf "$(COLOR_YELLOW)提示: 未安装 cargo-watch, 普通模式运行; cargo install cargo-watch 可启用文件变化自动重启$(COLOR_RESET)\n"; \
		$(CARGO) run -p devtools-app; \
	fi

run-direct: ## 运行桌面应用 (debug, 不启用 watch)
	@printf "$(COLOR_CYAN)▶ 运行 devtools-app...$(COLOR_RESET)\n"
	$(CARGO) run -p devtools-app

run-release: ## 运行桌面应用 (release)
	$(CARGO) run --release -p devtools-app

# =========================================================================
# 检查 / Lint / 测试
# =========================================================================
check: cargo-check clippy test ## 全量检查 (check + clippy + test)

cargo-check: ## cargo check (workspace)
	@printf "$(COLOR_CYAN)▶ cargo check...$(COLOR_RESET)\n"
	$(CARGO) check -p devtools-core -p devtools-app

clippy: ## cargo clippy (workspace, 零警告标准)
	@printf "$(COLOR_CYAN)▶ cargo clippy...$(COLOR_RESET)\n"
	$(CARGO) clippy -p devtools-core --all-targets
	$(CARGO) clippy -p devtools-app --all-targets

test: ## cargo test (core 单元测试 + 黄金向量)
	@printf "$(COLOR_CYAN)▶ cargo test...$(COLOR_RESET)\n"
	$(CARGO) test -p devtools-core

fmt: ## cargo fmt (workspace)
	$(CARGO) fmt

# =========================================================================
# 构建 / 打包
# =========================================================================
app: ## 打包 DevTools.app (debug, scripts/build-app.sh)
	@printf "$(COLOR_CYAN)▶ 打包 DevTools.app...$(COLOR_RESET)\n"
	scripts/build-app.sh

app-release: ## 打包 DevTools.app (release)
	@printf "$(COLOR_CYAN)▶ 打包 DevTools.app (release)...$(COLOR_RESET)\n"
	scripts/build-app.sh release

# 兼容别名
gpui-run: run
gpui-check: check
gpui-clean: clean-all

# =========================================================================
# 清理
# =========================================================================
clean: ## 清理构建产物 (target/)
	@printf "$(COLOR_YELLOW)▶ 清理 target/...$(COLOR_RESET)\n"
	$(CARGO) clean

clean-all: clean ## 清理全部构建产物

# =========================================================================
# 信息
# =========================================================================
info: ## 显示项目环境信息
	@printf "$(COLOR_BOLD)--- 环境信息 ---$(COLOR_RESET)\n"
	@printf "Rust:    %s\n" "$$(rustc --version)"
	@printf "Cargo:   %s\n" "$$(cargo --version)"
	@printf "Root:    %s\n" "$(ROOT_DIR)"
	@printf "Target:  %s\n" "$(TARGET_DIR)"
