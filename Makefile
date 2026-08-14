# Makefile for building and testing clap_schema.
.DEFAULT_GOAL := help

# Cargo profile for builds.
PROFILE ?= dev

##@ Help

.PHONY: help
help: ## Display this help.
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Build

.PHONY: build
build: ## Build the workspace into the `target` directory.
	cargo build \
		--workspace \
		--all-features \
		--profile "$(PROFILE)" \
		--locked

##@ Test

.PHONY: test-unit
test-unit: ## Run unit and integration tests.
	cargo nextest run \
		--workspace \
		--all-features \
		--no-fail-fast \
		--locked

.PHONY: test-doc
test-doc: ## Run doc tests.
	cargo test \
		--doc \
		--workspace \
		--all-features \
		--locked

.PHONY: test-examples
test-examples: ## Build and run all runnable examples.
	cargo build \
		--package clap_schema \
		--examples \
		--all-features \
		--locked
	@set -eu; \
	examples='$(sort $(patsubst crates/clap_schema/examples/%.rs,%,$(wildcard crates/clap_schema/examples/*.rs)))'; \
	for example in $$examples; do \
		printf "\n==> Running example: %s\n" "$$example"; \
		cargo run \
			--quiet \
			--package clap_schema \
			--example "$$example" \
			--all-features \
			--locked >/dev/null; \
	done

.PHONY: test
test: ## Run unit, integration, example, and documentation tests.
	$(MAKE) test-unit && \
	$(MAKE) test-examples && \
	$(MAKE) test-doc

.PHONY: test-coverage
test-coverage: ## Run tests with coverage and generate an LCOV report.
	cargo +nightly llvm-cov nextest \
		--workspace \
		--all-features \
		--lcov \
		--output-path lcov.info \
		--locked

.PHONY: test-coverage-html
test-coverage-html: ## Run tests with coverage and generate and open an HTML report.
	cargo +nightly llvm-cov nextest \
		--workspace \
		--all-features \
		--html \
		--open \
		--locked

##@ Linting

.PHONY: fmt
fmt: ## Run all formatters.
	cargo +nightly fmt --all

.PHONY: lint-clippy
lint-clippy: ## Run Clippy on the codebase.
	cargo +nightly clippy \
		--workspace \
		--all-targets \
		--all-features \
		--locked \
		-- -D warnings

.PHONY: lint-clippy-fix
lint-clippy-fix: ## Run Clippy on the codebase and fix warnings.
	cargo +nightly clippy \
		--workspace \
		--all-targets \
		--all-features \
		--fix \
		--allow-dirty \
		--allow-staged \
		--locked \
		-- -D warnings

.PHONY: lint-typos
lint-typos: ## Run typos on the codebase.
	@command -v typos >/dev/null || { \
		echo "typos not found. Please install it by running the command 'cargo install typos-cli' or refer to the following link for more information: https://github.com/crate-ci/typos"; \
		exit 1; \
	}
	typos

.PHONY: lint
lint: ## Run all linters.
	$(MAKE) fmt && \
	$(MAKE) lint-clippy && \
	$(MAKE) lint-typos

##@ Documentation

.PHONY: doc
doc: ## Build the documentation.
	RUSTDOCFLAGS="--cfg docsrs -D warnings -Zunstable-options --show-type-layout --generate-link-to-definition" \
		cargo +nightly doc \
			--workspace \
			--all-features \
			--document-private-items \
			--no-deps \
			--locked

##@ Other

.PHONY: lock
lock: ## Update the Cargo.lock file with the current dependencies.
	cargo fetch

.PHONY: clean
clean: ## Clean the project.
	cargo clean

.PHONY: deny
deny: ## Perform a `cargo deny` check.
	cargo deny --locked --all-features check all

.PHONY: about
about: ## Generate the `THIRD_PARTY_NOTICES.md` file.
	cargo about generate -c .github/about.toml -o THIRD_PARTY_NOTICES.md .github/about.hbs --frozen

.PHONY: check
check: ## Check all crates and targets.
	cargo hack check --locked --feature-powerset --depth 1

.PHONY: pr
pr: ## Run all checks and tests.
	$(MAKE) deny && \
	$(MAKE) check && \
	$(MAKE) lint && \
	$(MAKE) test && \
	$(MAKE) doc && \
	$(MAKE) about
