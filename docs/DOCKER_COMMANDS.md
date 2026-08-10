# Quick Docker Testing Commands

## Setup

```bash
# Build Docker image
docker-compose -f docker-compose/docker-compose.dev.yml build

# Start container
docker-compose -f docker-compose/docker-compose.dev.yml up -d

# Stop container
docker-compose -f docker-compose/docker-compose.dev.yml down
```

## Running Tests

```bash
# Enter container shell
docker-compose -f docker-compose/docker-compose.dev.yml exec kunger-dev bash

# Then inside container:
cd src-tauri

# Run unit tests
cargo test --lib tui::app

# Run integration tests
cargo test --test tui_integration_test

# Run all tests
cargo test --lib tui::app --test tui_integration_test

# Run specific test
cargo test test_search_filtering -- --nocapture
```

## One-Liners

```bash
# Run all tests without entering container
docker-compose -f docker-compose/docker-compose.dev.yml exec kunger-dev \
  bash -c "cd src-tauri && cargo test --lib tui::app"

# Build CLI binary
docker-compose -f docker-compose/docker-compose.dev.yml exec kunger-dev \
  bash -c "cd src-tauri && cargo build --bin kunger-cli"

# Run CLI (requires test data)
docker-compose -f docker-compose/docker-compose.dev.yml exec -e TERM=xterm-256color kunger-dev \
  bash -c "cd src-tauri && ./target/debug/kunger-cli"
```

## Useful Cargo Commands

Inside container:

```bash
# Build only (no tests)
cargo build --lib --bin kunger-cli

# Build in release mode (optimized)
cargo build --bin kunger-cli --release

# Run tests with output
cargo test --lib tui::app -- --nocapture

# Run tests single-threaded
cargo test --lib tui::app -- --test-threads=1

# Generate documentation
cargo doc --no-deps --open

# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy
```

## Debugging

```bash
# Get detailed error messages
RUST_BACKTRACE=1 cargo test --lib tui::app

# Full backtrace
RUST_BACKTRACE=full cargo test --lib tui::app

# Check test names
cargo test --lib tui::app -- --list

# Run test and print all output
cargo test test_navigation_next -- --nocapture
```

## Volume & Data Management

```bash
# Copy file INTO container
docker cp ./file.txt kunger-dev:/kunger/file.txt

# Copy file FROM container
docker cp kunger-dev:/kunger/file.txt ./file.txt

# Mount additional volume
docker-compose -f docker-compose/docker-compose.dev.yml exec kunger-dev \
  -v /path/on/host:/path/in/container bash
```

## Clean Up

```bash
# Stop and remove container
docker-compose -f docker-compose/docker-compose.dev.yml down

# Remove all containers and volumes
docker-compose -f docker-compose/docker-compose.dev.yml down -v

# Clean cargo cache in container
docker-compose -f docker-compose/docker-compose.dev.yml exec kunger-dev \
  bash -c "cd src-tauri && cargo clean"

# Rebuild from scratch
docker-compose -f docker-compose/docker-compose.dev.yml down -v
docker-compose -f docker-compose/docker-compose.dev.yml up -d --build
```

## Monitoring

```bash
# View container stats
docker stats kunger-dev

# View container logs
docker-compose -f docker-compose/docker-compose.dev.yml logs -f kunger-dev

# List running containers
docker ps

# Inspect container
docker inspect kunger-dev
```

## Common Workflow

```bash
# 1. Start development container
docker-compose -f docker-compose/docker-compose.dev.yml up -d

# 2. Enter container
docker-compose -f docker-compose/docker-compose.dev.yml exec kunger-dev bash

# 3. Inside container:
cd src-tauri

# 4. Run tests
cargo test --lib tui::app

# 5. Make changes locally (on host), they sync to container

# 6. Re-run tests inside container
cargo test --lib tui::app

# 7. Build release binary
cargo build --bin kunger-cli --release

# 8. Exit and clean up
exit
docker-compose -f docker-compose/docker-compose.dev.yml down
```

## Troubleshooting

```bash
# Check if image exists
docker images | grep kunger

# Check if container is running
docker ps | grep kunger-dev

# Rebuild image if broken
docker-compose -f docker-compose/docker-compose.dev.yml build --no-cache --force-rm

# Remove dangling images
docker image prune -a

# Rebuild everything fresh
docker-compose -f docker-compose/docker-compose.dev.yml down -v
docker-compose -f docker-compose/docker-compose.dev.yml up -d --build
```
