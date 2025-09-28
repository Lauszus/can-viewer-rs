.PHONY: cargo build-debug build-release check check-clippy check-fmt clean fix fix-clippy fix-fmt lock lock-upgrade run test

# This is the first target, so it is run if "make" is called without arguments.
run: $(CARGO)
	$(CARGO) run -- --channel vcan0

# Path to cargo.
CARGO ?= $(shell which cargo 2>/dev/null || echo "$(HOME)/.cargo/bin/cargo")

# Target for installing uv.
$(CARGO):
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	if [ -f $(HOME)/.cargo/env ]; then . $(HOME)/.cargo/env; fi

# Install cargo.
cargo: $(CARGO)

build-debug: $(CARGO)
	$(CARGO) build

build-release: $(CARGO)
	$(CARGO) build --release

check: check-fmt check-clippy

check-clippy: $(CARGO)
	$(CARGO) clippy -- -W clippy::pedantic -D warnings -A clippy::missing-errors-doc

check-fmt: $(CARGO)
	$(CARGO) fmt -- --check

clean: $(CARGO)
	$(CARGO) clean

fix: fix-clippy fix-fmt

fix-clippy: $(CARGO)
	$(CARGO) clippy --fix --allow-dirty --allow-staged -- -W clippy::pedantic -D warnings -A clippy::missing-errors-doc

fix-fmt: $(CARGO)
	$(CARGO) fmt

# Update the lock file if Cargo.toml changes.
# Cargo does not have a way of simply updating the lock file without upgrading,
# so check the code, which will update the lock file if needed.
Cargo.lock: Cargo.toml
	$(CARGO) check
	touch $(@)
lock: Cargo.lock

# Upgrade the Cargo lock file.
lock-upgrade: Cargo.lock
	$(CARGO) generate-lockfile

test: $(CARGO)
	$(CARGO) test
