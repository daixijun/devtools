# Makefile for DevTools 开发者工具箱（纯 Rust 桌面应用）
# 管理开发、构建、清理、检查等常用工作流
#
# 用法:
#   make help           # 显示所有可用命令
#   make run            # 运行桌面应用 (debug; watch 模式: 运行中保存文件自动重启, 退出应用即结束)
#   make run-direct     # 运行桌面应用 (debug, 不启用 watch)
#   make check          # 全量检查 (check + clippy + test)
#   make app            # 打包应用 (macOS → DevTools.app, 其他平台 → 裸二进制)
#   make dmg            # 打包 DevTools.dmg 安装镜像 (release, 仅 macOS)
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

.PHONY: help run run-direct run-release check clippy test fmt app app-release \
        dmg cargo-check clean clean-all info

# =========================================================================
# 帮助
# =========================================================================
help: ## 显示此帮助信息
	@printf "$(COLOR_BOLD)DevTools Makefile$(COLOR_RESET)\n"
	@printf "用法: $(COLOR_CYAN)make <target>$(COLOR_RESET)\n\n"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / { printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@printf "\n详见 $(COLOR_YELLOW)AGENTS.md$(COLOR_RESET) 了解开发约定。\n"

# =========================================================================
# 开发
# =========================================================================
run: ## 运行桌面应用 (debug; watch 模式: 运行中保存文件自动重启, 退出应用即结束 watch)
	@printf "$(COLOR_CYAN)▶ 运行 devtools...$(COLOR_RESET)\n"
	@scripts/dev-watch.sh

run-direct: ## 运行桌面应用 (debug, 不启用 watch)
	@printf "$(COLOR_CYAN)▶ 运行 devtools...$(COLOR_RESET)\n"
	$(CARGO) run

run-release: ## 运行桌面应用 (release)
	$(CARGO) run --release

# =========================================================================
# 检查 / Lint / 测试
# =========================================================================
check: fmt-check cargo-check clippy test ## 全量检查 (fmt + check + clippy + test)

fmt-check: ## cargo fmt --check（CI Format & Clippy job 同标准）
	@printf "$(COLOR_CYAN)▶ cargo fmt --check...$(COLOR_RESET)\n"
	$(CARGO) fmt --all -- --check

cargo-check: ## cargo check
	@printf "$(COLOR_CYAN)▶ cargo check...$(COLOR_RESET)\n"
	$(CARGO) check

clippy: ## cargo clippy (零警告标准，与 CI 一致)
	@printf "$(COLOR_CYAN)▶ cargo clippy...$(COLOR_RESET)\n"
	$(CARGO) clippy --all-targets -- -D warnings

test: ## cargo test (单元测试 + 黄金向量)
	@printf "$(COLOR_CYAN)▶ cargo test...$(COLOR_RESET)\n"
	$(CARGO) test

fmt: ## cargo fmt (workspace)
	$(CARGO) fmt

# =========================================================================
# 构建 / 打包
# =========================================================================
app: ## 打包应用（macOS → DevTools.app，其他平台 → 裸二进制）
	@printf "$(COLOR_CYAN)▶ 打包应用...$(COLOR_RESET)\n"
	scripts/build-app.sh

app-release: ## 打包应用 (release)
	@printf "$(COLOR_CYAN)▶ 打包应用 (release)...$(COLOR_RESET)\n"
	scripts/build-app.sh release

dmg: ## 打包 DevTools.dmg 安装镜像 (release, 仅 macOS, scripts/build-dmg.sh)
	@printf "$(COLOR_CYAN)▶ 打包 DevTools.dmg (release)...$(COLOR_RESET)\n"
	scripts/build-dmg.sh

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
