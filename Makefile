.PHONY: run cargo target-x86_64-unknown-linux-musl target-armv7-unknown-linux-musleabihf build-debug build-release build-release-armv7 publish publish-dry-run check check-clippy check-fmt clean fix fix-clippy fix-fmt lock lock-upgrade test sdist wheel

# This is the first target, so it is run if "make" is called without arguments.
run: $(CARGO)
	$(CARGO) run -- --channel vcan0

# Path to cargo.
CARGO ?= $(shell which cargo 2>/dev/null || echo "$(HOME)/.cargo/bin/cargo")

# Path to rustup.
RUSTUP ?= $(shell which rustup 2>/dev/null || echo "$(HOME)/.cargo/bin/rustup")

# Target for installing uv.
$(CARGO):
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
	if [ -f $(HOME)/.cargo/env ]; then . $(HOME)/.cargo/env; fi

# Install cargo.
cargo: $(CARGO)

# Install the x86_64-unknown-linux-musl target if not already installed.
target-x86_64-unknown-linux-musl: $(CARGO)
	if ! $(RUSTUP) target list | grep -q 'x86_64-unknown-linux-musl (installed)'; then \
	  $(RUSTUP) target add x86_64-unknown-linux-musl; \
	fi

target-armv7-unknown-linux-musleabihf: $(CARGO)
	if ! $(RUSTUP) target list | grep -q 'armv7-unknown-linux-musleabihf (installed)'; then \
	  $(RUSTUP) target add armv7-unknown-linux-musleabihf; \
	fi

# Build the project for the x86_64-unknown-linux-musl target.
build-debug: $(CARGO) target-x86_64-unknown-linux-musl
	$(CARGO) build --target x86_64-unknown-linux-musl

build-release: $(CARGO) target-x86_64-unknown-linux-musl
	$(CARGO) build --target x86_64-unknown-linux-musl --locked --release

build-release-armv7: $(CARGO) target-armv7-unknown-linux-musleabihf
	$(CARGO) build --target armv7-unknown-linux-musleabihf --locked --release

# Publish the crate to crates.io.
publish: $(CARGO)
	$(CARGO) publish --locked

publish-dry-run: $(CARGO)
	$(CARGO) publish --locked --dry-run

# Check formatting and clippy issues.
check: check-fmt check-clippy

check-clippy: $(CARGO)
	$(CARGO) clippy -- -W clippy::pedantic -D warnings -A clippy::missing-errors-doc

check-fmt: $(CARGO)
	$(CARGO) fmt -- --check

clean: $(CARGO)
	$(CARGO) clean

# Fix formatting and clippy issues.
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

# Run tests.
test: $(CARGO)
	$(CARGO) test

# Create source distribution and wheel.
sdist:
	docker run --rm -v $$(pwd):/io ghcr.io/pyo3/maturin sdist

wheel:
	docker run --rm -v $$(pwd):/io ghcr.io/pyo3/maturin build
