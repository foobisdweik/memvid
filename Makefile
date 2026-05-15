.PHONY: help test lint install clean

DESTDIR ?=
PREFIX  ?= /usr/local
CONFIG_DIR ?= /etc/memvid
STATE_DIR ?= /var/lib/memvid

help: ## Show this help
	@echo "Memvid Makefile targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-12s %s\n", $$1, $$2}'

test: ## Run shell test suite
	@bash tests/test_shard_lifecycle.sh
	@bash tests/test_wrappers.sh
	@bash tests/test_installer.sh

lint: ## Shellcheck deploy/bin and packaging
	@command -v shellcheck >/dev/null || { echo "shellcheck not installed" >&2; exit 1; }
	@shellcheck -x deploy/bin/memvid-write deploy/bin/memvid-context \
	             deploy/bin/claude-memvid deploy/bin/codex-memvid deploy/bin/gemini-memvid \
	             packaging/install.sh

install: ## Install scripts and create state dirs (use DESTDIR / PREFIX)
	@DESTDIR="$(DESTDIR)" PREFIX="$(PREFIX)" CONFIG_DIR="$(CONFIG_DIR)" STATE_DIR="$(STATE_DIR)" \
	  bash packaging/install.sh

clean: ## Remove build artifacts (no-op; nothing to clean)
	@true
