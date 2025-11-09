# DDR (Docker Dock Rust) - Parallel Docker-based Rust Build Orchestration

**Source**: `cargo-mate/captain/src/cmd/ddr.rs` and `cargo-mate/captain/src/cmd/smune.rs:DockDockRustCommands`

## Overview

DDR (Docker Dock Rust) is a sophisticated build orchestration system that leverages Docker containers to provide parallel, cross-platform Rust compilation with maximum efficiency and reproducibility. It supports multiple architectures, build targets, and optimization strategies.

**Main Handler**: `ddr.rs:handle_ddr()` (verified in `ddr.rs:503-521`)

## Quick Start

```bash
# Basic build
captain ddr build

# Cross-platform build with specific targets
captain ddr build --target x86_64-unknown-linux-gnu --target aarch64-unknown-linux-gnu

# Build with custom Docker image
captain ddr build --image rust:1.70

# Maximum parallel jobs
captain ddr build --jobs 32
```

## Architecture

### Core Components

1. **CLI Interface** (`src/cmd/ddr.rs`)
   - Command-line argument parsing
   - Build configuration management
   - Progress reporting and monitoring

2. **Build Orchestrator** (`src/ddr/ddr-orchestrator.sh`)
   - Docker container management
   - Parallel build coordination
   - Artifact collection and optimization

3. **Docker Images** (`src/ddr/docker/`)
   - Cross-compilation environments
   - Optimized build contexts
   - Multi-stage build configurations

## Commands

**Implementation**: All commands are handled by `ddr.rs:handle_ddr()` which matches on `DdrAction` enum.

### Build Command

**Source**: `ddr.rs:handle_build()` (verified in `ddr.rs:523-560`)

**Implementation**:
- If `ddr.toml` exists and `use_config` is true, loads config directly
- Otherwise prompts user if config exists
- Generates default config if none exists
- Creates `BuildOrchestrator` instance and runs build
- Prints build report

```bash
cm ddr build [OPTIONS]
```

#### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--image` | `-i` | Docker image to use | `rust:latest` |
| `--target` | `-t` | Build targets (repeatable) | Current platform |
| `--jobs` | `-j` | Maximum parallel jobs | `16` |
| `--config` | `-c` | Configuration file | `ddr.toml` |
| `--use-config` | | Use existing config without prompting | `false` |
| `--verbose` | `-v` | Verbose output | `false` |
| `--dry-run` | | Show what would be built | `false` |
| `--force` | `-f` | Force rebuild | `false` |
| `--no-cache` | | Disable Docker layer caching | `false` |
| `--output-dir` | | Custom output directory | `target/ddr` |
| `--clean` | | Clean build artifacts | `false` |
| `--watch` | | Watch mode (rebuild on changes) | `false` |
| `--profile` | | Build profile | `release` |

### Configuration File (ddr.toml)

```toml
# DDR Configuration File
[build]
# Docker image
image = "rust:latest"

# Build targets
targets = [
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "armv7-unknown-linux-gnueabihf"
]

# Parallel jobs
max_jobs = 16

# Build profiles
profiles = ["debug", "release"]

# Cache settings
cache_dir = "~/.cache/cargo-mate-ddr"

# Output configuration
output_dir = "target/ddr"
artifacts = ["binary", "debug", "docs"]

# Docker settings
docker = { pull = true, buildkit = true }

# Cross-compilation
[cross]
enabled = true
toolchains = ["musl", "gnu", "mingw"]
```

## Supported Platforms

### Native Builds
- **Linux GNU**: Debian-based with full GNU toolchain
- **Linux MUSL**: Alpine Linux with static linking
- **Windows**: Cross-compilation for x64 and x86 targets
- **macOS**: Cross-compilation for Intel and Apple Silicon

### Cross-Compilation
- **ARM**: ARMv7 and AArch64 support
- **Android**: Android NDK integration
- **FreeBSD**: FreeBSD cross-compilation
- **WebAssembly**: WASM and WASI targets

### Advanced Builds
- **BuildKit**: Modern Docker build system with parallel stages
- **Multi-stage**: Optimized layered builds with caching
- **Parallel**: Concurrent target building

## Docker Images

### Available Images

| Image | Purpose | Size | Cross-compilation |
|-------|---------|------|-------------------|
| `alpine-musl.dockerfile` | Minimal static builds | ~50MB | Linux MUSL |
| `debian-gnu.dockerfile` | Full GNU compatibility | ~150MB | Linux GNU |
| `windows-cross.dockerfile` | Windows executables | ~200MB | Win64/Win32 |
| `arm-cross.dockerfile` | ARM architectures | ~180MB | ARMv7/AArch64 |
| `wasm.dockerfile` | WebAssembly builds | ~120MB | WASM/WASI |
| `macos-cross.dockerfile` | macOS applications | ~250MB | macOS x64/ARM64 |
| `android.dockerfile` | Android applications | ~300MB | Android ABIs |
| `freebsd.dockerfile` | FreeBSD compatibility | ~160MB | FreeBSD |
| `buildkit-parallel.dockerfile` | Parallel builds | ~100MB | Multi-platform |
| `multi-stage-cached.dockerfile` | Cached builds | ~80MB | Optimized |

### Custom Images

Create custom Docker images by extending the base configurations:

```dockerfile
FROM rust:latest AS builder

# Add your custom dependencies
RUN apt-get update && apt-get install -y \
    custom-package \
    && rm -rf /var/lib/apt/lists/*

# Your build logic here
```

## Build Optimization

### Caching Strategies

1. **Dependency Caching**: Cache Rust dependencies separately
2. **Layer Optimization**: Minimize layer changes for better caching
3. **Artifact Reuse**: Reuse build artifacts across builds

### Performance Tuning

```bash
# Maximum parallelism
captain ddr build --jobs 32

# Use BuildKit for parallel builds
export DOCKER_BUILDKIT=1

# Enable SCCache
export RUSTC_WRAPPER=sccache

# Custom cache directory
export SCCACHE_DIR=~/.cache/sccache
```

## Examples

### Basic Linux Build

```bash
captain ddr build
```

### Cross-Platform Release

```bash
captain ddr build \
    --target x86_64-unknown-linux-gnu \
    --target x86_64-pc-windows-gnu \
    --target x86_64-apple-darwin \
    --profile release
```

### Android Application

```bash
captain ddr build \
    --target aarch64-linux-android \
    --target armv7-linux-androideabi \
    --image android-ndk:latest
```

### WebAssembly

```bash
captain ddr build \
    --target wasm32-unknown-unknown \
    --target wasm32-wasi
```

### Multi-Architecture

```bash
captain ddr build \
    --target x86_64-unknown-linux-gnu \
    --target aarch64-unknown-linux-gnu \
    --target armv7-unknown-linux-gnueabihf \
    --jobs 8
```

## Monitoring and Debugging

### Progress Reporting

DDR provides real-time progress reporting:

```
[DDR] Building 12 targets with 8 parallel jobs
[DDR] [1/12] x86_64-unknown-linux-gnu: Building...
[DDR] [1/12] x86_64-unknown-linux-gnu: ✅ Completed (2.3s)
[DDR] [2/12] aarch64-unknown-linux-gnu: Building...
```

### Verbose Mode

```bash
captain ddr build --verbose
```

Shows detailed Docker commands, build logs, and timing information.

### Debug Information

```bash
captain ddr build --dry-run
```

Shows what would be executed without actually building.

## Troubleshooting

### Common Issues

#### Docker Not Found
```bash
# Install Docker
curl -fsSL https://get.docker.com | sh

# Start Docker service
sudo systemctl start docker
```

#### Permission Issues
```bash
# Add user to docker group
sudo usermod -aG docker $USER

# Restart session or run:
newgrp docker
```

#### Build Failures
```bash
# Check Docker image
docker run --rm rust:latest rustc --version

# Clear caches
rm -rf ~/.cache/cargo-mate-ddr
docker system prune -f
```

#### Memory Issues
```bash
# Reduce parallel jobs
captain ddr build --jobs 4

# Increase Docker memory limit
docker system info | grep "Total Memory"
```

### Logs and Debugging

```bash
# Enable verbose logging
export RUST_LOG=debug
captain ddr build --verbose

# View Docker logs
docker logs <container_id>

# Inspect build artifacts
ls -la target/ddr/
```

## Advanced Configuration

### Custom Docker Registry

```bash
export DDR_DOCKER_REGISTRY=myregistry.com
captain ddr build --image myregistry.com/rust:latest
```

### CI/CD Integration

```yaml
# GitHub Actions
- name: Build with DDR
  run: |
    captain ddr build \
      --target x86_64-unknown-linux-gnu \
      --target x86_64-pc-windows-gnu \
      --jobs 4
```

### Custom Build Scripts

Create `ddr-orchestrator.sh` hooks:

```bash
# Pre-build hook
function pre_build_hook() {
    echo "Setting up build environment..."
    # Your custom setup
}

# Post-build hook
function post_build_hook() {
    echo "Build completed, running tests..."
    # Your custom post-processing
}
```

## Security Considerations

### Sandboxed Builds
- All builds run in isolated Docker containers
- No access to host filesystem (except explicitly mounted volumes)
- Network access can be restricted

### Dependency Scanning
DDR integrates with security scanning tools:
- Cargo audit for Rust dependencies
- Container image vulnerability scanning
- Binary analysis for compiled artifacts

## Performance Benchmarks

### Build Times (example project)

| Configuration | Targets | Time | Improvement |
|---------------|---------|------|-------------|
| Native build | 1 | 45s | Baseline |
| DDR parallel | 3 | 52s | +15% overhead |
| DDR cached | 3 | 18s | 60% faster |
| DDR BuildKit | 5 | 28s | 38% faster |

### Resource Usage

| Configuration | CPU | Memory | Disk |
|---------------|-----|--------|------|
| Single target | 100% | 512MB | 2GB |
| 8 parallel jobs | 800% | 4GB | 16GB |
| 16 parallel jobs | 1600% | 8GB | 32GB |

## Contributing

### Development Setup

```bash
# Clone repository
git clone https://github.com/your-org/cargo-mate.git
cd cargo-mate

# Build development version
cargo build

# Run tests
cargo test --package cargo-mate
```

### Adding New Targets

1. Create Docker image in `src/ddr/docker/`
2. Add target to `DdrAction::Build` enum
3. Update documentation
4. Test cross-compilation

### Code Structure

```
src/
├── cmd/
│   └── ddr.rs              # CLI interface
├── ddr/
│   ├── docker/            # Docker images
│   │   └── *.dockerfile
│   └── ddr-orchestrator.sh # Build script
└── main.rs                # Command routing
```

## License

DDR is part of the cargo-mate project and follows the same licensing terms.

## See Also

- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Docker Documentation](https://docs.docker.com/)
- [Cross-compilation Guide](https://rust-lang.github.io/rustup/cross-compilation.html)
- [BuildKit](https://docs.docker.com/develop/dev-best-practices/)
