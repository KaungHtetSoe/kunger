# Testing Kunger CLI TUI in Docker

This guide explains how to test the Kunger CLI TUI inside a Docker container.

## Why Docker?

- **Isolated Environment**: Clean, reproducible testing environment
- **Terminal Support**: Proper TTY for TUI interaction
- **System Dependencies**: All required build tools included
- **Consistent Results**: Same environment on any machine

## Quick Start

### 1. Build and Run Docker Container

Using `docker-compose` (recommended):

```bash
docker-compose -f docker-compose.dev.yml up -d
docker-compose -f docker-compose.dev.yml exec kunger-dev bash
```

Or with plain Docker:

```bash
docker build -f Dockerfile.dev -t kunger-dev .
docker run -it --rm -v $(pwd):/kunger kunger-dev bash
```

### 2. Run Tests Inside Container

```bash
# Run all unit tests
cargo test --lib tui::app

# Run integration tests
cargo test --test tui_integration_test

# Run all tests with output
cargo test --lib tui::app -- --nocapture

# Run specific test
cargo test test_cli_workflow_1_basic_loading_and_navigation -- --nocapture
```

### 3. Build the CLI Binary

```bash
cargo build --bin kunger-cli
```

### 4. Start the CLI

The CLI starts with an empty inventory when no cache exists. Press `F5` to run
a scan; press `Esc` to cancel an active scan and `q` to quit.

```bash
./target/debug/kunger-cli
```

### 5. Optional: Create Test Database

To test navigation and filtering against known data, preload the cache:

```bash
# Option A: Use SQLite directly to insert test data
sqlite3 ~/.local/share/kunger/kunger.db < test_data.sql

# Option B: Run the desktop app first to generate real inventory
# (requires X11 forwarding or running on your host machine)
```

### 6. Run the CLI Interactive

```bash
# Run the CLI with your user data directory
./target/debug/kunger-cli

# Or with custom data directory (for testing)
export XDG_DATA_HOME=/tmp/kunger-test
./target/debug/kunger-cli
```

## Testing Scenarios

### Unit Tests Only

```bash
cd /kunger/src-tauri
cargo test --lib tui::app -- --test-threads=1
```

**What it tests:**

- App state management
- Filtering logic
- Search functionality
- Sorting
- Detail view state
- Scan state transitions

**Expected:** 42 tests passing

### Integration Tests

```bash
cd /kunger/src-tauri
cargo test --test tui_integration_test -- --test-threads=1
```

**What it tests:**

- Real user workflows
- Search + filter combinations
- Navigation across pages
- Sorting with filtering
- Complex scenarios

**Expected:** 15 tests passing

### Full Test Suite

```bash
cd /kunger/src-tauri
cargo test --lib tui::app --test tui_integration_test
```

**Expected:** 57 tests passing

## Testing with Real Data

### Option 1: Copy Host Database

If you have the Kunger desktop app and want to test with real data:

```bash
# On your host machine
docker cp ~/.local/share/kunger/kunger.db kunger-dev:/home/kunger/.local/share/kunger/

# Inside container
./target/debug/kunger-cli
```

### Option 2: Create Test Database

Create a test database with sample data:

```bash
# Inside container
sqlite3 ~/.local/share/kunger/kunger.db < /kunger/tests/test_data.sql
./target/debug/kunger-cli
```

### Option 3: Mock Data Script

Use a helper script to generate test data:

```bash
cd /kunger/src-tauri
python3 ../scripts/generate_test_inventory.py
./target/debug/kunger-cli
```

## Troubleshooting

### Container won't start

```bash
# Check build output
docker-compose -f docker-compose.dev.yml build --no-cache

# Check logs
docker-compose -f docker-compose.dev.yml logs
```

### Cargo build fails

```bash
# Clear cargo cache and rebuild
docker-compose -f docker-compose.dev.yml exec kunger-dev cargo clean
docker-compose -f docker-compose.dev.yml exec kunger-dev cargo build --lib

# Or rebuild the container
docker-compose -f docker-compose.dev.yml down
docker-compose -f docker-compose.dev.yml up -d --build
```

### Tests fail

```bash
# Run with backtrace
RUST_BACKTRACE=1 cargo test --lib tui::app -- --nocapture

# Run single test with verbose output
cargo test test_search_filtering -- --nocapture --exact
```

### CLI starts with no items

```bash
# Press F5 to create and populate the cache from installed software.
# If it remains empty, inspect the provider commands available in the container.
```

## Advanced: Persistent Container

Keep the container running and attach/detach as needed:

```bash
# Start container in background
docker-compose -f docker-compose.dev.yml up -d

# Execute commands
docker-compose -f docker-compose.dev.yml exec kunger-dev cargo test --lib tui::app
docker-compose -f docker-compose.dev.yml exec kunger-dev cargo build --bin kunger-cli

# Interactive shell
docker-compose -f docker-compose.dev.yml exec kunger-dev bash

# View logs
docker-compose -f docker-compose.dev.yml logs -f kunger-dev

# Stop container
docker-compose -f docker-compose.dev.yml down
```

## Testing Checklist

- [ ] Unit tests pass (37 tests)
- [ ] Integration tests pass (15 tests)
- [ ] Binary builds successfully
- [ ] CLI starts without errors
- [ ] Can navigate with arrow keys
- [ ] Search filters items correctly
- [ ] Detail view opens/closes with Enter/Esc
- [ ] Filters work independently
- [ ] Sort order toggles correctly
- [ ] Pagination works with 20 items per page

## Performance Testing

Monitor resource usage inside container:

```bash
# In another terminal
docker stats kunger-dev

# Inside container - run tests and check
time cargo test --lib tui::app --release
```

## Next Steps

Once tests pass in Docker:

1. **Manual Testing**: Run `./target/debug/kunger-cli` with real data
2. **User Acceptance**: Have actual users test on their systems
3. **Performance**: Benchmark with large inventories (10,000+ items)
4. **Edge Cases**: Test with unusual software configurations
5. **CI/CD Integration**: Add Docker test stage to GitHub Actions

## References

- [Dockerfile.dev](../Dockerfile.dev) — Development Docker image
- [docker-compose.dev.yml](../docker-compose.dev.yml) — Docker Compose configuration
- [Testing Guide](../src-tauri/tests/README.md) — Detailed test documentation
