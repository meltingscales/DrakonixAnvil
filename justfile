# List available recipes
help:
    @just --list

# Build the project in release mode
build:
    cargo build --release

# Run all tests
test:
    cargo test

# Run the GUI application (dev mode)
run:
    cargo run

# Run the GUI application in release mode (faster)
run-release:
    cargo run --release

# Format Rust code
fmt:
    cargo fmt

# Run clippy linter
lint:
    cargo clippy -- -D warnings

# Run clippy with all warnings
lint-all:
    cargo clippy -- -W clippy::all -W clippy::pedantic

# Show version info that will be embedded in the build
version-info:
    @echo "Git Commit:  $(git rev-parse --short HEAD)"
    @echo "Git Branch:  $(git rev-parse --abbrev-ref HEAD)"
    @echo "Build Date:  $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Clean up generated files and build artifacts
clean:
    cargo clean

# Development helpers
# ===================

# Watch for changes and rebuild (requires cargo-watch)
watch:
    cargo watch -x run

# Check code without building
check:
    cargo check

# Build documentation
doc:
    cargo doc --open

# Docker helpers
# ==============

# Check if Docker is running
docker-check:
    @docker info > /dev/null 2>&1 && echo "Docker is running" || echo "Docker is NOT running"

# List all Drakonix-managed containers
docker-list:
    docker ps -a --filter "label=drakonix.managed=true" --format "table {{{{.Names}}}}\t{{{{.Status}}}}\t{{{{.Ports}}}}"

# Stop all Drakonix-managed containers
docker-stop-all:
    docker ps -q --filter "label=drakonix.managed=true" | xargs -r docker stop

# Remove all stopped Drakonix-managed containers
docker-clean:
    docker ps -aq --filter "label=drakonix.managed=true" --filter "status=exited" | xargs -r docker rm

# Show Docker resource usage for Minecraft servers
docker-stats:
    docker stats --no-stream --filter "label=drakonix.managed=true"

# Release workflow
# ================

# Run full CI check (format, lint, test, build)
ci: fmt lint test build
    @echo "CI checks passed!"

# Create a release build for distribution
release:
    cargo build --release
    @echo "Release binary at: target/release/drakonix-anvil"
    @ls -lh target/release/drakonix-anvil
