# Cargo-Mate Probe Commands Overview

The `cm probe` namespace provides a comprehensive suite of tools for managing, analyzing, and optimizing your Rust probe suite. These commands help you detect flaky probes, measure performance, generate CI configurations, and maintain reliable testing infrastructure.

## Table of Contents

- [Quick Start](#quick-start)
- [Command Reference](#command-reference)
  - [1. `cm probe flake` - Flaky Probe Detector](#1-cm-probe-flake--flaky-probe-detector)
  - [2. `cm probe impact` - Impact Analysis](#2-cm-probe-impact--impact-analysis)
  - [3. `cm probe coverage` - Coverage Analysis](#3-cm-probe-coverage--coverage-analysis)
  - [4. `cm probe profile` - Performance Profiling](#4-cm-probe-profile--performance-profiling)
  - [5. `cm probe tag` - Tag-Based Filtering](#5-cm-probe-tag--tag-based-filtering)
  - [6. `cm probe ci-gen` - CI Configuration Generator](#6-cm-probe-ci-gen--ci-configuration-generator)
  - [7. `cm probe env` - Docker Environment Manager](#7-cm-probe-env--docker-environment-manager)
  - [8. `cm probe replay` - Failure Reproduction](#8-cm-probe-replay--failure-reproduction)
  - [9. `cm probe order` - Randomized Execution Order](#9-cm-probe-order--randomized-execution-order)
  - [10. `cm probe doc` - Documentation Generator](#10-cm-probe-doc--documentation-generator)
- [Advanced Usage Patterns](#advanced-usage-patterns)
- [Configuration](#configuration)
- [Implementation Details](#implementation-details)

## Quick Start

```bash
# Detect flaky probes
cm probe flake -i 20 --threshold 95

# Run only probes affected by recent changes
cm probe impact --base main --head HEAD

# Generate coverage report and open it
cm probe coverage --open

# Profile the slowest probes
cm probe profile --top 10

# Use custom tags for selective execution
cm probe tag slow              # Run only slow probes
cm probe tag --exclude network # Skip network-related probes

# Generate CI configuration
cm probe ci-gen --platform github --coverage --flake-detect > .github/workflows/ci.yml

# Spin up Docker services for integration probes
cm probe env up && cm probe env run && cm probe env down

# Replay a failed probe run deterministically
cm probe replay <run-id>

# Randomize probe order to detect ordering dependencies
cm probe order --random

# Generate markdown documentation of all probes
cm probe doc -o PROBES.md
```

## Command Reference

### 1. `cm probe flake` – Flaky Probe Detector
**Purpose:** Identifies probes that pass/fail inconsistently across multiple runs.

**Common Usage:**
```bash
cm probe flake -i 30 -j 8 --threshold 95
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `-i, --iterations <N>` | Number of times to run each probe | 20 |
| `-j, --jobs <N>` | Number of parallel workers | 4 |
| `-t, --probe <PATTERN>` | Run only probes matching pattern | All probes |
| `--threshold <PCT>` | Fail if pass-rate below percentage | 90 |
| `-n, --dry-run` | Show plan without executing | false |

**Output:**
```
NAME                     PASS  FAIL  PASS%
------------------------------------------------
db::connect               28     2   93.3%
api::slow_response        15    13   53.6%
```

**Implementation:** Uses Rayon for parallel execution, aggregates pass/fail statistics, generates JSON reports in `target/cmt-reports/flake.json`.

---

### 2. `cm probe impact` – Impact Analysis
**Purpose:** Runs only probes affected by recent Git changes for faster feedback.

**Common Usage:**
```bash
cm probe impact --base origin/main --head HEAD
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `-b, --base <REF>` | Base Git reference | origin/main |
| `-h, --head <REF>` | Head Git reference | HEAD |
| `-c, --cache <DIR>` | Cache directory for source→probe mapping | ~/.cache/cmt-impact |
| `-v, --verbose` | Show selected probe list | false |

**Output:**
```
Running 7/42 probes (16%)
```

**Implementation:** Uses Git diff to identify changed files, maintains a cached mapping of source files to probes, executes only affected probes.

---

### 3. `cm probe coverage` – Coverage Analysis
**Purpose:** Generates coverage reports with automatic backend detection.

**Common Usage:**
```bash
cm probe coverage --open --threshold 85.0
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `--open` | Open HTML report in browser | false |
| `-o, --output <FILE>` | JSON summary output path | target/coverage.json |
| `--compare <FILE>` | Compare against previous run | - |
| `--threshold <PCT>` | Fail if coverage below percentage | - |

**Backend Priority:**
1. `llvm-cov` (fast, precise) - requires `rustup component add llvm-tools-preview`
2. `tarpaulin` (fallback, auto-downloaded)

**Output:**
```json
{
  "lines": 84.3,
  "functions": 91.2,
  "branches": 78.5
}
```

---

### 4. `cm probe profile` – Performance Profiling
**Purpose:** Measures execution time of individual probes with optional flamegraphs.

**Common Usage:**
```bash
cm probe profile --top 15 --flamegraph slow.svg
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `--top <N>` | Show N slowest probes | 10 |
| `--probe <PATTERN>` | Focus on specific probe | All probes |
| `--flamegraph <FILE>` | Generate flamegraph SVG | - |
| `-n, --dry-run` | Show plan without executing | false |

**Output:** Markdown table + JSON report (`target/cmt-reports/profile.json`) + optional flamegraph SVG.

---

### 5. `cm probe tag` – Tag-Based Filtering
**Purpose:** Custom probe categorization and selective execution using the `#[tag(...)]` attribute.

**Common Usage:**
```bash
cm probe tag slow network        # Run probes tagged with both "slow" AND "network"
cm probe tag --exclude flaky     # Run all probes except those tagged "flaky"
cm probe tag --list              # List all available tags
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `<TAG>...` | Run probes with these tags (AND logic) | - |
| `--exclude <TAG>` | Exclude probes with these tags | - |
| `--list` | Display all available tags | false |
| `--dry-run` | Show selection without running | false |

**Usage:**
```rust
#[tag(slow, integration, database)]
#[probe]
fn heavy_database_query() {
    // This probe has three tags
}
```

---

### 6. `cm probe ci-gen` – CI Configuration Generator
**Purpose:** One-click generation of CI pipelines that integrate Cargo-Mate probe commands.

**Common Usage:**
```bash
cm probe ci-gen --platform github --coverage --flake-detect --profile > .github/workflows/ci.yml
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `--platform <NAME>` | CI platform: `github`, `gitlab`, `azure` | - |
| `--coverage` | Include coverage step | false |
| `--flake-detect` | Include flaky probe detection | false |
| `--profile` | Include profiling step | false |
| `-o, --output <FILE>` | Output file path | stdout |

**Supported Platforms:**
- **GitHub Actions** (.github/workflows/ci.yml)
- **GitLab CI** (.gitlab-ci.yml)
- **Azure DevOps** (azure-pipelines.yml)

---

### 7. `cm probe env` – Docker Environment Manager
**Purpose:** Manages Docker containers for probe dependencies (databases, services, etc.).

**Common Usage:**
```bash
cm probe env up && cm probe env run && cm probe env down
```

**Subcommands:**
| Subcommand | Description |
|------------|-------------|
| `up` | Start all defined services |
| `run` | Execute probes with services running |
| `down` | Stop and remove all containers |

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `-c, --config <FILE>` | TOML configuration file | cargo-mate-env.toml |
| `--verbose` | Show container startup logs | false |

**Configuration Example:**
```toml
[[service]]
name = "postgres"
image = "postgres:15"
ports = ["5432:5432"]
env = { POSTGRES_PASSWORD = "test", POSTGRES_DB = "probes" }

[[service]]
name = "redis"
image = "redis:7"
ports = ["6379:6379"]
```

---

### 8. `cm probe replay` – Failure Reproduction
**Purpose:** Deterministically replay failed probe runs from saved snapshots.

**Common Usage:**
```bash
cm probe replay 2024-09-25T14:30:15Z
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `<RUN_ID>` | UUID/timestamp of failed run | - |
| `--output <DIR>` | Directory to extract snapshot | Temporary |
| `--no-cleanup` | Preserve extracted snapshot | false |

**Snapshot Contents:**
- Compiled probe binaries
- `Cargo.lock` and `Cargo.toml`
- Environment variables
- Rust/Cargo version info
- Original command arguments

---

### 9. `cm probe order` – Randomized Execution Order
**Purpose:** Detects probe ordering dependencies by randomizing execution sequence.

**Common Usage:**
```bash
cm probe order --random --repeat 3
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `--random` | Use random seed each run | true |
| `--seed <HEX>` | Fixed seed for reproducibility | Random |
| `-n, --dry-run` | Show order without executing | false |
| `--repeat <N>` | Run suite N times | 1 |

**Output:**
```
SEED=0x9f2a1c7b5e4d3f2a
1  api::authentication
2  db::connection_pool
3  utils::json_parsing
...
```

---

### 10. `cm probe doc` – Documentation Generator
**Purpose:** Generates comprehensive markdown documentation of all probes.

**Common Usage:**
```bash
cm probe doc -o PROBES.md --include-private
```

**Parameters:**
| Flag | Description | Default |
|------|-------------|---------|
| `-o, --output <FILE>` | Output markdown file | probes.md |
| `--include-private` | Include probes in `#[cfg(probe)]` modules | false |
| `--skip-ignored` | Exclude `#[ignore]` probes | false |

**Output Format:**
```markdown
# Probe Inventory

| Probe | Description | Tags | File |
|-------|-------------|------|------|
| db::connect | Database connectivity test | slow, db | probes/db.rs:12 |
| api::auth | Authentication flow validation | security | src/api/auth.rs:45 |
```

## Advanced Usage Patterns

### CI/CD Integration
```bash
# Generate comprehensive CI pipeline
cm probe ci-gen --platform github --coverage --flake-detect --profile > .github/workflows/ci.yml

# Run impact analysis in CI (fast feedback)
cm probe impact --base ${{ github.event.pull_request.base.sha }} --head ${{ github.sha }}

# Fail PR if coverage drops
cm probe coverage --compare baseline.json --threshold 90.0
```

### Development Workflow
```bash
# Quick feedback on changed code
cm probe impact --verbose

# Profile performance regressions
cm probe profile --top 5 --flamegraph perf.svg

# Tag and filter probes
cm probe tag unit                # Fast unit probes only
cm probe tag integration         # Integration probes only
cm probe tag --exclude slow      # Skip slow probes during development
```

### Debugging and Reliability
```bash
# Detect flaky probes
cm probe flake -i 50 --threshold 98

# Reproduce exact failure conditions
cm probe replay <run-id> --no-cleanup

# Find ordering dependencies
cm probe order --seed 0x123456789abcdef0 --repeat 5
```

## Configuration

### Global Configuration (~/.shipwreck/config.toml)
```toml
[probe]
flaky_iterations = 30
coverage_threshold = 85.0
impact_cache_dir = "~/.cache/cmt-impact"
env_config_file = "cargo-mate-env.toml"
```

### Project Configuration (.cargo-mate.toml)
```toml
[probe]
default_platform = "github"
enable_coverage = true
enable_flake_detection = true
tags = ["unit", "integration", "e2e"]
```

## Implementation Details

**Key Dependencies:**
- `clap` - Command-line argument parsing
- `serde` - JSON serialization/deserialization
- `tokio` - Async runtime for Docker operations
- `git2` - Git integration for impact analysis
- `bollard` - Docker API client
- `cargo_metadata` - Cargo project metadata
- `rayon` - Parallel probe execution

**Report Storage:**
All commands store results in `target/cmt-reports/`:
- `flake.json` - Flaky probe detection results
- `impact.json` - Impact analysis results
- `coverage.json` - Coverage summaries
- `profile.json` - Performance profiling data
- `tag_index.json` - Tag metadata

**Integration Points:**
- `cm view <report>` - View JSON reports in terminal UI
- `cm config` - Configuration management
- `cm tool` - Plugin system for extensibility

This comprehensive probe suite transforms Rust testing from basic correctness verification into a sophisticated quality assurance and performance monitoring system.
