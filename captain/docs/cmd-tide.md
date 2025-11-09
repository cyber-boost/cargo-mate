## Tide Charts Commands

**Source**: `cargo-mate/captain/src/cmd/tide.rs` and `cargo-mate/captain/src/cmd/smune.rs:TideAction`

### `cm tide show`
**Description**: Show interactive performance charts (TUI)

**Source**: `tide.rs:handle_tide()` - `TideAction::Show` case

**Usage**:
```bash
cm tide show
```

**Implementation** (verified in `tide.rs:5-7`):
- Creates `TideCharts` instance
- Calls `charts.show_interactive()` method
- Uses `captain::tide::TideCharts` module for TUI display

**Controls**:
- `Tab` / `Shift+Tab`: Switch between views
- `q` / `Esc`: Quit
- Views: Overview, Performance, Dependencies, Trends

---

### `cm tide analyze`
**Description**: Analyze build performance and dependencies

**Source**: `tide.rs:handle_tide()` - `TideAction::Analyze` case

**Usage**:
```bash
cm tide analyze
```

**Implementation** (verified in `tide.rs:8-10`):
- Creates `TideCharts` instance
- Calls `charts.analyze_dependencies()` method
- Uses `captain::tide::TideCharts` module for analysis

**Output**:
```
🔍 Analyzing dependency compile times...
✅ Timing data collected. Check target/cargo-timings/ for detailed report.
```

---

### `cm tide export <path>`
**Description**: Export build metrics to CSV

**Source**: `tide.rs:handle_tide()` - `TideAction::Export` case

**Usage**:
```bash
cm tide export ./metrics.csv
```

**Implementation** (verified in `tide.rs:11-13`):
- Creates `TideCharts` instance
- Calls `charts.export_csv(&path)` method
- Exports metrics to specified CSV file path

**CSV Format**:
```csv
timestamp,command,duration,success,errors,warnings
2024-01-20T14:30:00Z,build,45.2,true,0,3
```

---

## Implementation Module

- **TideCharts**: `cargo-mate/captain/src/captain/tide.rs` - Performance tracking and visualization

## All Tide Actions

Verified from `smune.rs:TideAction` enum:
- `Show` - Interactive TUI display
- `Analyze` - Dependency analysis
- `Export { path: PathBuf }` - CSV export

---