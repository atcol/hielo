# CLAUDE.md - Hielo Project Guide

This document provides comprehensive guidance for AI assistants working on the Hielo codebase.

## Project Overview

**Hielo** (Spanish for "ice") is a native desktop application for visualizing Apache Iceberg table metadata and snapshot history. Built with Rust and Dioxus, it provides a cross-platform GUI for exploring Iceberg tables from REST and AWS Glue catalogs.

### Key Features
- Multiple catalog support (REST and AWS Glue catalogs)
- Schema visualization with nested field support and schema evolution tracking
- Partition specification viewing with transform functions
- Snapshot timeline with filtering and table health analytics
- Persistent catalog configuration (stored in `~/.hielo/config.json`)

## Technology Stack

- **Language**: Rust (2024 edition)
- **UI Framework**: Dioxus 0.6 (desktop target)
- **Styling**: Tailwind CSS (loaded via CDN)
- **Iceberg Integration**: iceberg-rs 0.6.0 with REST and Glue catalog support
- **Async Runtime**: Tokio

## Project Structure

```
hielo/
├── src/
│   ├── main.rs           # Application entry point, main App component, navigation
│   ├── catalog.rs        # Catalog management (REST/Glue connections, namespace/table listing)
│   ├── catalog_ui.rs     # Catalog connection forms and table browser UI
│   ├── components.rs     # Table view components (Overview, Schema, Partitions, Snapshots)
│   ├── data.rs           # Core data types (IcebergTable, Schema, Snapshot, health metrics)
│   ├── analytics.rs      # Table health analytics engine (scoring, alerts, recommendations)
│   ├── config.rs         # Configuration persistence (~/.hielo/config.json)
│   └── iceberg_adapter.rs # Conversion from iceberg-rs types to internal types
├── Cargo.toml            # Rust dependencies and project config
├── Dioxus.toml           # Dioxus framework configuration
└── .github/workflows/    # CI/CD (build, test, cross-platform releases)
```

## Module Responsibilities

### `main.rs`
- Application state management (`AppState`, `AppTab`, `TableViewTab`)
- Main `App` component with tab-based navigation
- Left navigation pane with catalog/namespace/table tree
- Global search modal (Ctrl+K)
- Event handlers for table loading and tab management

### `catalog.rs`
- `CatalogManager` - manages catalog connections and queries
- `CatalogConfig` - configuration for REST/Glue catalogs
- `CatalogConnection` - active connection with catalog trait object
- Async methods: `connect_catalog`, `list_namespaces`, `list_tables`, `load_table`

### `catalog_ui.rs`
- `CatalogConnectionScreen` - initial connection screen
- `RestCatalogForm` / `GlueCatalogForm` - catalog-specific connection forms
- `TableBrowser` - namespace and table selection UI
- `SavedCatalogsSection` - quick connect to saved catalogs

### `components.rs`
- `TableOverviewTab` - table metadata and properties display
- `TableSchemaTab` - schema visualization with evolution comparison
- `TablePartitionsTab` - partition specification display
- `SnapshotTimelineTab` - snapshot history with filtering and health analytics
- Health analytics display components (`HealthScoreBadge`, `HealthCategoryCard`)

### `data.rs`
- `IcebergTable` - main table representation with all metadata
- `TableSchema`, `NestedField`, `DataType` - schema types
- `Snapshot`, `Summary` - snapshot information
- `PartitionSpec`, `PartitionField`, `PartitionTransform` - partitioning
- Health analytics types (`TableHealthMetrics`, `FileHealthMetrics`, etc.)

### `analytics.rs`
- `TableAnalytics::compute_health_metrics()` - main entry point for health analysis
- Health scoring based on industry best practices (Netflix, Salesforce, AWS)
- Alert generation for small files, high snapshot frequency, compaction needs
- Maintenance recommendations with priority and effort levels

### `config.rs`
- `AppConfig` - persistent configuration with catalog list
- Load/save to `~/.hielo/config.json`
- Catalog CRUD operations with uniqueness validation

### `iceberg_adapter.rs`
- `convert_iceberg_table()` - main conversion function
- Type conversions from iceberg-rs to internal representations
- Schema, snapshot, and partition spec conversion helpers

## Key Data Types

### Core Table Structure
```rust
IcebergTable {
    name: String,
    namespace: String,
    catalog_name: String,
    location: String,
    schema: TableSchema,
    schemas: Vec<TableSchema>,      // Historical schemas
    snapshots: Vec<Snapshot>,
    current_snapshot_id: Option<u64>,
    properties: HashMap<String, String>,
    partition_spec: Option<PartitionSpec>,
    partition_specs: Vec<PartitionSpec>, // Historical specs
}
```

### Catalog Types
```rust
enum CatalogType { Rest, Glue }

CatalogConfig {
    catalog_type: CatalogType,
    name: String,
    config: HashMap<String, String>, // uri, warehouse, region, etc.
}
```

## Development Commands

```bash
# Run in development mode
cargo run

# Build release binary
cargo build --release

# Run tests
cargo test

# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt

# Run clippy lints
cargo clippy -- -D warnings

# Run all checks (what CI does)
cargo check --all-targets --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Build Dependencies

### Linux (Ubuntu/Debian)
```bash
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

### macOS
```bash
xcode-select --install
```

### Windows
- WebView2 (pre-installed on Windows 10/11)
- Visual Studio Build Tools with C++ tools

## Code Conventions

### Dioxus Component Patterns

1. **Component Definition**: Use `#[component]` attribute with PascalCase names
```rust
#[component]
fn MyComponent(prop1: String, prop2: Signal<SomeType>) -> Element {
    rsx! { /* ... */ }
}
```

2. **State Management**: Use Dioxus signals for reactive state
```rust
let mut my_state = use_signal(|| initial_value);
my_state.set(new_value);  // Update
my_state()                 // Read
```

3. **Async Operations**: Use `spawn` for async operations
```rust
spawn(async move {
    let result = some_async_operation().await;
    my_signal.set(result);
});
```

4. **Event Handlers**: Use `EventHandler<T>` for callbacks
```rust
on_table_selected: EventHandler<(String, String, String)>
on_table_selected.call((catalog, namespace, table));
```

### Styling Conventions
- Tailwind CSS classes via CDN
- Format strings for conditional classes:
```rust
class: format!("base-class {}", if condition { "active" } else { "inactive" })
```

### Error Handling
- Use `anyhow::Result` for fallible operations
- Custom `CatalogError` enum for catalog-specific errors
- Log errors with `log::error!()` / `tracing::info!()`

### Naming Conventions
- Modules: snake_case (`catalog_ui.rs`)
- Types: PascalCase (`IcebergTable`)
- Functions: snake_case (`load_table`)
- Components: PascalCase (`TableOverviewTab`)

## Testing

The project uses standard Rust testing:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test code
    }
}
```

Key test areas:
- `config.rs` - Configuration persistence and catalog management
- `iceberg_adapter.rs` - Type conversions

Run tests with: `cargo test`

## CI/CD Pipeline

Located in `.github/workflows/`:

1. **ci.yml** - Main CI workflow
   - `check` job: `cargo check`
   - `quality` job: `cargo fmt --check` and `cargo clippy`
   - `build` job: Cross-platform builds (Linux, macOS, Windows)

2. **build.yml** - Reusable cross-platform build workflow
   - Linux x86_64, macOS x86_64/ARM64, Windows x86_64
   - Binary stripping for release builds
   - SHA256 checksums for artifacts

3. **build-release.yml** - Release builds (triggered on tags)

## Architecture Notes

### State Flow
1. User connects to catalog via `CatalogConnectionScreen`
2. `CatalogManager` stores connection and persists config
3. Navigation pane shows catalogs -> namespaces -> tables
4. Table selection triggers `load_table` which:
   - Calls `catalog.load_table()` for iceberg-rs Table
   - Converts to internal `IcebergTable` via `iceberg_adapter`
   - Opens new tab with table view

### Catalog Connection Flow
```
CatalogConnectionScreen
    -> RestCatalogForm / GlueCatalogForm
    -> CatalogManager.connect_catalog()
    -> Creates Arc<dyn Catalog> (iceberg-rs)
    -> Stores CatalogConnection
    -> Persists to ~/.hielo/config.json
```

### Table Loading Flow
```
LeftNavigationPane (table click)
    -> load_table closure
    -> CatalogManager.load_table()
    -> iceberg_adapter::convert_iceberg_table()
    -> Creates AppTab::Table
    -> Switches to new tab
```

## Common Tasks

### Adding a New Table View Tab
1. Add variant to `TableViewTab` enum in `main.rs`
2. Create component in `components.rs`
3. Add button and match arm in table view section of `main.rs`

### Adding a New Catalog Type
1. Add variant to `CatalogType` enum in `catalog.rs`
2. Create connection method in `CatalogManager`
3. Add form component in `catalog_ui.rs`
4. Update catalog type selection UI

### Modifying Health Analytics
1. Update thresholds in `analytics.rs` (`HealthThresholds`)
2. Modify `compute_health_metrics()` for new metrics
3. Add alert generation in `generate_alerts()`
4. Update UI components in `components.rs`

## Performance Considerations

- Catalog/namespace data is loaded lazily on expansion
- Table metadata is cached per tab (no auto-refresh)
- Large snapshot lists are filtered client-side
- Debounced filtering in navigation pane (300ms)

## Security Notes

- Auth tokens are hidden in UI display (`***HIDDEN***`)
- AWS credentials use standard SDK credential chain
- No sensitive data logged (tokens sanitized)
- Config file stored in user's home directory with default permissions
