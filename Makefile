# Makefile for Devtools (Tauri + React + TypeScript)
# 管理开发、构建、清理、检查等常用工作流
#
# 用法:
#   make help           # 显示所有可用命令
#   make install        # 安装前端依赖
#   make dev            # 启动桌面应用开发模式 (Tauri)
#   make build          # 构建前端到 dist/
#   make tauri-build    # 打包桌面应用
#   make clean          # 清理构建产物

# --- 可执行程序 -----------------------------------------------------------
PNPM    := pnpm
CARGO   := cargo
NPM     := npm

# --- 路径 -----------------------------------------------------------------
ROOT_DIR     := $(CURDIR)
SRC_TAURI    := $(ROOT_DIR)/src-tauri
DIST_DIR     := $(ROOT_DIR)/dist
TARGET_DIR   := $(SRC_TAURI)/target

# --- 颜色输出 -------------------------------------------------------------
COLOR_RESET  := \033[0m
COLOR_BOLD   := \033[1m
COLOR_GREEN  := \033[32m
COLOR_CYAN   := \033[36m
COLOR_YELLOW := \033[33m

.DEFAULT_GOAL := help

# Phony targets - 不对应实际文件
.PHONY: help install install-ci dev preview build tauri \
        tauri-build tauri-clean check check-tsc check-styles \
        cargo-check cargo-fmt cargo-clippy clean clean-all release \
        fmt info

# =========================================================================
# 帮助
# =========================================================================
help: ## 显示此帮助信息
	@printf "$(COLOR_BOLD)Devtools Makefile$(COLOR_RESET)\n"
	@printf "用法: $(COLOR_CYAN)make <target>$(COLOR_RESET)\n\n"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / { printf "  $(COLOR_GREEN)%-15s$(COLOR_RESET) %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
	@printf "\n详见 $(COLOR_YELLOW)AGENTS.md$(COLOR_RESET) 了解开发约定。\n"

# =========================================================================
# 依赖安装
# =========================================================================
install: ## 安装前端依赖 (pnpm install)
	@printf "$(COLOR_CYAN)▶ 安装前端依赖...$(COLOR_RESET)\n"
	$(PNPM) install

install-ci: ## 以 CI 模式安装依赖 (frozen lockfile)
	$(PNPM) install --frozen-lockfile

# =========================================================================
# 开发
# =========================================================================
dev: ## 启动桌面应用开发模式 (Tauri + Vite)
	@printf "$(COLOR_CYAN)▶ 启动桌面应用开发模式...$(COLOR_RESET)\n"
	$(PNPM) tauri dev

preview: ## 预览构建后的 Web 产物
	$(PNPM) preview

# =========================================================================
# 构建
# =========================================================================
build: check-tsc ## 构建前端 (tsc + vite build) 到 dist/
	@printf "$(COLOR_CYAN)▶ 构建前端...$(COLOR_RESET)\n"
	$(PNPM) build

tauri-build: ## 打包桌面应用 (前端 + Rust)
	@printf "$(COLOR_CYAN)▶ 打包桌面应用...$(COLOR_RESET)\n"
	$(PNPM) tauri build

tauri: ## 直接转发到 tauri CLI (make tauri args="dev" / "build" / ...)
	$(PNPM) tauri $(args)

release: tauri-build ## 以 release 模式打包桌面应用 (与 tauri-build 等价)

# =========================================================================
# 检查 / Lint
# =========================================================================
check: check-tsc check-styles cargo-check ## 运行前端 + Rust 全部检查

check-tsc: ## TypeScript 类型检查
	@printf "$(COLOR_CYAN)▶ TypeScript 类型检查...$(COLOR_RESET)\n"
	$(PNPM) exec tsc --noEmit

check-styles: ## 检查 Tailwind/CSS 样式一致性
	@printf "$(COLOR_CYAN)▶ 检查样式...$(COLOR_RESET)\n"
	$(NPM) run check:styles

fmt: cargo-fmt ## 格式化代码 (当前仅 Rust)

# --- Rust 侧 -------------------------------------------------------------
cargo-check: ## cargo check (src-tauri)
	@printf "$(COLOR_CYAN)▶ cargo check...$(COLOR_RESET)\n"
	cd $(SRC_TAURI) && $(CARGO) check

cargo-fmt: ## cargo fmt (src-tauri)
	@printf "$(COLOR_CYAN)▶ cargo fmt...$(COLOR_RESET)\n"
	cd $(SRC_TAURI) && $(CARGO) fmt

cargo-clippy: ## cargo clippy (src-tauri)
	@printf "$(COLOR_CYAN)▶ cargo clippy...$(COLOR_RESET)\n"
	cd $(SRC_TAURI) && $(CARGO) clippy --all-targets --all-features

# =========================================================================
# 清理
# =========================================================================
clean: ## 清理前端构建产物 (dist/)
	@printf "$(COLOR_YELLOW)▶ 清理 dist/...$(COLOR_RESET)\n"
	rm -rf $(DIST_DIR)

tauri-clean: ## 清理 Rust 构建产物 (src-tauri/target)
	@printf "$(COLOR_YELLOW)▶ 清理 src-tauri/target...$(COLOR_RESET)\n"
	cd $(SRC_TAURI) && $(CARGO) clean

clean-all: clean tauri-clean ## 清理全部构建产物 (前端 + Rust)

# =========================================================================
# 信息
# =========================================================================
info: ## 显示项目环境信息
	@printf "$(COLOR_BOLD)--- 环境信息 ---$(COLOR_RESET)\n"
	@printf "Node:    %s\n" "$$(node --version)"
	@printf "pnpm:    %s\n" "$$(pnpm --version)"
	@printf "Rust:    %s\n" "$$(rustc --version)"
	@printf "Cargo:   %s\n" "$$(cargo --version)"
	@printf "Tauri:   %s\n" "$$(pnpm tauri --version 2>/dev/null || echo 'n/a')"
	@printf "Root:    %s\n" "$(ROOT_DIR)"
	@printf "Dist:    %s\n" "$(DIST_DIR)"
