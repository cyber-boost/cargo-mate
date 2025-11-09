## Config File Examples

### Global Config (`~/.shipwreck/config.toml`)
```toml
[project]
theme = "nautical"
auto_checklist = true
track_performance = true

[shortcuts]
b = "build --release"
t = "test --all"
c = "check"

[auto_fix]
format_on_save = true
clippy_on_build = false

[build]
default_profile = "dev"
incremental = true
```

### Project Config (`.cg`)
```toml
[project]
name = "my-awesome-project"
default_journey = "dev-cycle"

[shortcuts]
dev = "run --bin dev-server"
prod = "build --release --target x86_64-unknown-linux-musl"

[hooks]
pre_build = ["cargo fmt --check"]
post_build = ["cargo test --quiet"]
on_error = ["cm checklist"]

[version]
auto_increment = true
increment_policy = "patch"
```

---