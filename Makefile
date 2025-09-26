.PHONY: cargo build-debug build-release check clean fix lock lock-upgrade run test

# This is the first target, so it is run if "make" is called without arguments.
run: $(CARGO)
	$(CARGO) run

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

check: $(CARGO)
	$(CARGO) fmt -- --check
	$(CARGO) clippy -- -W clippy::pedantic -D warnings

clean: $(CARGO)
	$(CARGO) clean

fix: $(CARGO)
	$(CARGO) fmt
	$(CARGO) fix --allow-dirty --allow-staged

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
