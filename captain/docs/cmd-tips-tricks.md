## Tips & Tricks

### 1. Quick Development Cycle
```bash
cm journey record dev-cycle
cargo fmt
cargo clippy
cargo test
cargo run
# Ctrl+D

# Later:
cm  # If default_journey = "dev-cycle"
```

### 2. Safe Experimentation
```bash
cm anchor save safe-point
cm mutiny yolo  # Go wild for 30 min
# ... experiment ...
cm anchor restore safe-point  # If things go wrong
```

### 3. Team Workflow Sharing
```bash
cm journey record onboarding
# Show new developer the full setup
cm journey export onboarding team-onboarding.json
# Share the file
```

### 4. Performance Tracking
```bash
# After each build, cm automatically tracks metrics
cm tide show  # View beautiful charts
cm tide export metrics.csv  # For external analysis
```

### 5. Dependency Auditing
```bash
cm map analyze  # Quick dependency check
cm map path vulnerable-crate my-crate  # Trace dependencies
```

### 6. Build Optimization
```bash
# Get recommendations for your system
cm optimize recommendations

# Apply aggressive optimizations
cm optimize aggressive

# Check what was changed
cm optimize status

# Restore if needed
cm optimize restore
```

### 7. Auto-Versioning
```bash
# Initialize with custom version
cm version init 2.0.0

# Version auto-increments on every build
cm check    # 2.0.0 -> 2.0.1
cm build    # 2.0.1 -> 2.0.2

# Manual version bump
cm version increment minor  # 2.0.2 -> 2.1.0
```

---