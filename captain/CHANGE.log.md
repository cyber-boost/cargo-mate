# Cargo‑Mate Change Log

> **Generated**: 2025‑08‑31 01:03:44 (America/Port‑Au‑Prince)  
> **Scope**: All changes that actually affect the public API, build process or user experience.  

---

## 2025‑09‑25
| Area | What Changed |
|------|--------------|
| **Probe Commands** | • **Complete Implementation**: Added all 10 probe commands under `cm probe` namespace (`flake`, `impact`, `coverage`, `profile`, `tag`, `ci-gen`, `env`, `replay`, `order`, `doc`). <br>• **Flaky Detection**: Parallel probe execution with statistical analysis (`cm probe flake -i 30 -j 8 --threshold 95`). <br>• **Impact Analysis**: Git-based change detection to run only affected probes (`cm probe impact --base main`). <br>• **Coverage Integration**: Automatic backend detection (llvm-cov/tarpaulin) with HTML reports (`cm probe coverage --open`). <br>• **Performance Profiling**: Per-probe timing with optional flamegraphs (`cm probe profile --top 10 --flamegraph perf.svg`). <br>• **Tag System**: Custom probe categorization with `#[tag(...)]` attribute (`cm probe tag slow --exclude network`). <br>• **CI Generation**: One-click pipeline creation for GitHub/GitLab/Azure (`cm probe ci-gen --platform github --coverage`). <br>• **Docker Environment**: Container management for integration probes (`cm probe env up && cm probe env run`). <br>• **Failure Reproduction**: Deterministic replay from snapshots (`cm probe replay <run-id>`). <br>• **Order Randomization**: Detect ordering dependencies (`cm probe order --random --repeat 3`). <br>• **Documentation**: Auto-generated markdown probe inventories (`cm probe doc -o PROBES.md`). |
| **Testing Suite** | • **Comprehensive Tests**: 58 test cases covering all commands, subcommands, parameters, and edge cases. <br>• **Integration Testing**: End-to-end workflow validation with multiple command sequences. <br>• **Error Handling**: Boundary condition testing and graceful failure scenarios. <br>• **Help System**: Complete CLI documentation verification for all commands. |
| **Documentation** | • **Complete Reference**: `probe.md` with detailed usage, parameters, examples, and implementation notes for all 10 commands. <br>• **Advanced Patterns**: CI/CD integration, development workflows, debugging scenarios. <br>• **Configuration Guide**: Global and project-level configuration options. |
| **Code Quality** | • **Compilation Fixes**: Resolved all Rust compilation errors in probe.rs and test suite. <br>• **Type Safety**: Proper clap derive usage with correct enum variants and parameter handling. <br>• **Error Handling**: Comprehensive error propagation and user-friendly messages. |

---

## 2025‑08‑30
| Area | What Changed |
|------|--------------|
| **Codebase** | • Added `DatabaseWithLogging` in all classes that perform DB work.  <br>• Updated all PDO usage to log queries (query time, statement, parameters). |
| **Schema** | • Inserted `email` column in `users` (fixes silent user‑creation failures). |
| **Affiliate** | • Detect `cargo_affiliate` cookie during checkout.  <br>• Create referral records in `affiliate_referrals`. |
| **Build** | • Removed all `sccache` verification (prevents false build errors).  <br>• Produced ARM64‑Linux (musl) and macOS‑Intel binaries; macOS‑Intel build skipped because `osxcross` was missing. |
| **CLI** | • `cm version` now supports Patch, Minor, Major, Custom policy.  <br>• Added `cm tool`, `cm check`, `cm view`, `cm optimize`.  <br>• `cm install` auto‑detects platform, downloads the right tarball, installs binaries. |
| **Security & Deployment** | • SSL certificates are installed/valid on all domains.  <br>• Fixed Nginx redirect rule (`$server_name → $host`).  <br>• Added musl OpenSSL fallback chain.  <br>• Updated Nginx to route download requests through a PHP handler that logs each request. |

---

## 2025‑08‑29
| Area | What Changed |
|------|--------------|
| **Affinity Tables** | • Created `affiliate_payouts` – tracks quarterly payouts.  <br>• Added proper indices for fast lookup. |
| **Build Stability** | • Completely disabled `sccache`; all builds run cleanly. |
| **Platforms** | • Built ARM64‑Linux (musl), Windows (x86_64, i686). |
| **Versioning** | • `Cargo.toml` auto‑synchronises on each Cargo command (increment `1.0.0 → 1.0.1` by default). |
| **NGINX** | • Removed old `mate.grim.so` site‑configuration. |
| **Enhancements** | • `cm optimize` added per‑profile optimization switches (Aggressive, Balanced, Conservative, Custom). |
| **Installer** | • All installer scripts live in a single `sh/` folder; embedded via `include_str!`. |

---

## 2025‑08‑28
| Area | What Changed |
|------|--------------|
| **API** | • `/api/download‑stats.php` & `/api/webhook.php` now log all queries.  <br>• `/api/download.php` logs affiliate information & file names. |
| **CLI** | • `cm exec` added: runs arbitrary Cargo commands with pre/post hooks & auto‑increment.  <br>• `cm install` now supports PowerShell (`install.ps1`) on Windows. |
| **Environment** | • All DB credentials moved to `DB_*` env vars; no hard‑coded secrets. |
| **Configuration** | • Added `config.rs` for persistent, per‑project configuration and CLI `cm config` command. |

---

## 2025‑08‑27
| Area | What Changed |
|------|--------------|
| **Affiliate System** | • Full creation of `Affiliates` & `AffiliateReferrals` tables.  <br>• Cookie capture (`cargo_mate_afl`) now marks users as `referred = TRUE`. |
| **Installer / Protection** | • XOR‑encrypted, per‑platform `captain` binaries.  <br>• Fallback key chain (`CAPTAIN_SOBER / CAPTAIN_DRUNK`) for offline usage. |
| **Upgrades** | • Added `cm upgrade` command to update binaries & embedded installer scripts. |
| **Maintenance** | • Automatic cleanup of old logs & database backups. |

---

## 2025‑08‑26
| Area | What Changed |
|------|--------------|
| **Distribution** | • All installer scripts are embedded into the single `cm` binary (`include_str!`), removing any external script dependencies. |
| **Command Surface** | • `cm list`, `cm find`, `cm strip`, `cm map`, `cm wtf`, `cm help`. |
| **Logging** | • Structured logs with timestamp, context and error description.  <br>• `cm view errors` reads from `~/.shipwreck/errors/latest.txt`. |

---

## 2025‑08‑25
| Area | What Changed |
|------|--------------|
| **Checkout Flow** | • Affiliate cookie capture now fully functional; `set_test_affiliate_cookie.php` correctly flags referrals. |
| **Stripe** | • Single “Card Information” element used; `stripe.createPaymentMethod()` receives it correctly. |
| **Cert & Redirects** | • Nginx uses `$host`; all certs are now valid. |
| **Email** | • Professional HTML templates for license delivery, purchase confirmation, renewal reminders. |
| **Codebase** | • Removed obsolete ML/AI modules (`ml_engine.rs`, `analytics_engine.rs`, etc.); stubs left only for backward compatibility. |

---

## 2025‑08‑24
| Area | What Changed |
|------|--------------|
| **Command Naming** | • Every command now starts with `cm`; old `captain` sub‑command naming removed. |
| **Wrappers** | • All wrapper scripts (`wrapper-linux.sh`, `wrapper-macos.sh`, `wrapper-windows.ps1`) consolidated into a single `wrapper/` directory. |

---

## 2025‑08‑23 – 2025‑08‑21
| Area | What Changed |
|------|--------------|
| **Compatibility** | • GLIBC requirement lowered 2.39 → 2.31; musl fallback added for Alpine & legacy systems. |
| **CLI** | • `cm exec` now accepts any Cargo command with automatic version bump. |
| | • Standardised the key used for the affiliate cookie (`cargo_mate_afl`). |
| | • Updated `tide`, `treasure_map`, and `optimize` modules to expose clean delegates (`captain::*`). |
| | • Permission checks added to commands; error codes returned to the shell. |

---

### Quick‑Start Cheat Sheet

| Command | Purpose |
|---------|---------|
| `cm install` | Installs `cm` and the protected `captain` binary for your platform. |
| `cm version` | Show current version; `cm version bump [patch|minor|major]` to auto‑increment. |
| `cm check` | Checks Cargo and toolchain for missing dependencies. |
| `cm view errors` | Opens the latest error log. |
| `cm optimize [profile]` | Applies the chosen optimization profile (`aggressive`, `balanced`, `conservative`). |
| `cm strip [options]` | Obfuscation tool to rename identifiers, scramble strings, pack files. |
| `cm map` | Generate a visual dependency map. |
| `cm wtf [sub‑cmd]` | AI‑powered troubleshooting assistant (e.g., `wtf analyze`, `wtf fix`). |
| `cm upgrade` | Pulls the next release and updates the binary + installer scripts. |
