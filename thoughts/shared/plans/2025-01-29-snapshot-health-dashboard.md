# Snapshot Tab Health Dashboard Implementation Plan

## Overview

Add a comprehensive table health monitoring dashboard to the snapshot tab, providing users with industry-standard Iceberg table health metrics, performance indicators, and proactive maintenance recommendations. This enhancement transforms the snapshot tab from a basic timeline view into a powerful table monitoring and optimization tool.

## Current State Analysis

**Existing Implementation:**
- Basic snapshot timeline with filtering capabilities (`SnapshotTimelineTab` in `components.rs:785-1203`)
- Current summary statistics: total snapshots, operation counts, time span
- Individual snapshot details: ID, timestamp, records added, size change, files added
- Rich underlying data available but underutilized in `Summary` struct

**Available Data Not Currently Displayed:**
- File-level metrics: `added_files_size`, `removed_files_size`, `total_size`, `added_data_files`, `deleted_data_files`
- Schema evolution tracking via `schema_id` changes
- Partition specification evolution via `partition_specs`
- Table properties that may contain maintenance configuration
- Temporal patterns across snapshots for trend analysis

**Key Constraints:**
- All metrics must be computed client-side from existing snapshot data
- UI performance must remain smooth with 1000+ snapshots
- Must maintain existing filtering and timeline functionality

## Desired End State

A comprehensive health dashboard positioned above the existing snapshot timeline that provides:

1. **Health Score**: Overall table health indicator (0-100 scale)
2. **File Health Metrics**: Small file detection, average file sizes, compaction recommendations
3. **Performance Indicators**: Query planning proxies, metadata bloat detection
4. **Ingestion Analytics**: Pattern analysis, anomaly detection, rate trends
5. **Storage Efficiency**: Active vs total storage ratios, growth trends
6. **Maintenance Recommendations**: Actionable optimization suggestions

### Verification Criteria:
- Health dashboard loads within 500ms for tables with 1000+ snapshots
- All metrics update in real-time as filters are applied
- Health score accurately reflects critical thresholds from industry best practices
- Maintenance recommendations appear when actionable thresholds are exceeded

## What We're NOT Doing

- Real-time monitoring or alerting (this is historical analysis only)
- Integration with external monitoring systems or databases
- Automated compaction or maintenance operations
- Cross-table comparisons or benchmarking
- Query performance correlation (would require external query logs)
- Iceberg metadata table queries (working only with snapshot history)

## Implementation Approach

**Strategy**: Add comprehensive analytics computation layer that processes snapshot data to generate health insights, displayed in an expandable dashboard section above the existing timeline.

**Technical Approach**:
1. Create analytics computation functions that process `Vec<Snapshot>` data
2. Add new UI components for health dashboard sections
3. Integrate dashboard with existing filter system for real-time updates
4. Use industry-standard thresholds for health scoring and recommendations

## Phase 1: Core Analytics Engine

### Overview
Build the foundational analytics computation layer that processes snapshot data to generate all health metrics.

### Changes Required:

#### 1. Analytics Data Structures
**File**: `src/data.rs`
**Changes**: Add comprehensive health analytics structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableHealthMetrics {
    pub health_score: u8, // 0-100
    pub file_health: FileHealthMetrics,
    pub performance_indicators: PerformanceIndicators,
    pub ingestion_analytics: IngestionAnalytics,
    pub storage_efficiency: StorageEfficiency,
    pub maintenance_recommendations: Vec<MaintenanceRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileHealthMetrics {
    pub average_file_size_mb: f64,
    pub small_file_percentage: f64,
    pub total_files: u64,
    pub files_under_128mb: u64,
    pub files_under_64mb: u64,
    pub largest_file_size_mb: f64,
    pub smallest_file_size_mb: f64,
    pub compaction_urgency: CompactionUrgency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompactionUrgency {
    Healthy,
    Warning,     // >10 files per partition OR avg size <128MB
    Critical,    // >1000 files with avg <64MB
    Emergency,   // >10000 files OR avg <32MB
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PerformanceIndicators {
    pub metadata_bloat_score: u8, // 0-100, higher = more bloated
    pub schema_evolution_count: u32,
    pub partition_spec_changes: u32,
    pub snapshot_density: f64, // snapshots per day
    pub manifest_fragmentation_estimate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestionAnalytics {
    pub daily_ingestion_rate: Vec<IngestionDataPoint>,
    pub operation_pattern: OperationPattern,
    pub anomaly_score: u8, // 0-100
    pub batch_size_trend: BatchSizeTrend,
    pub commit_frequency_trend: CommitFrequencyTrend,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestionDataPoint {
    pub date: String, // YYYY-MM-DD
    pub records_added: u64,
    pub files_added: u32,
    pub size_added_bytes: u64,
    pub commit_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationPattern {
    StreamingAppend,  // Frequent small appends
    BatchOverwrite,   // Periodic large overwrites
    Mixed,           // Combination of operations
    Irregular,       // No clear pattern
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageEfficiency {
    pub total_storage_gb: f64,
    pub active_storage_gb: f64,
    pub deleted_storage_gb: f64,
    pub storage_efficiency_ratio: f64, // active / total
    pub growth_trend_gb_per_day: f64,
    pub cleanup_potential_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaintenanceRecommendation {
    pub priority: RecommendationPriority,
    pub action: String,
    pub reason: String,
    pub estimated_benefit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}
```

#### 2. Analytics Computation Engine
**File**: `src/analytics.rs` (new file)
**Changes**: Core analytics computation functions

```rust
use crate::data::{Snapshot, TableHealthMetrics, FileHealthMetrics, CompactionUrgency,
                  PerformanceIndicators, IngestionAnalytics, StorageEfficiency,
                  MaintenanceRecommendation, RecommendationPriority, IngestionDataPoint,
                  OperationPattern, BatchSizeTrend, CommitFrequencyTrend};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

pub fn compute_table_health_metrics(snapshots: &[Snapshot]) -> TableHealthMetrics {
    let file_health = compute_file_health_metrics(snapshots);
    let performance_indicators = compute_performance_indicators(snapshots);
    let ingestion_analytics = compute_ingestion_analytics(snapshots);
    let storage_efficiency = compute_storage_efficiency(snapshots);

    let health_score = compute_overall_health_score(
        &file_health,
        &performance_indicators,
        &ingestion_analytics,
        &storage_efficiency,
    );

    let maintenance_recommendations = generate_maintenance_recommendations(
        &file_health,
        &performance_indicators,
        &storage_efficiency,
    );

    TableHealthMetrics {
        health_score,
        file_health,
        performance_indicators,
        ingestion_analytics,
        storage_efficiency,
        maintenance_recommendations,
    }
}

fn compute_file_health_metrics(snapshots: &[Snapshot]) -> FileHealthMetrics {
    // Industry thresholds from AWS/Netflix best practices
    const SMALL_FILE_THRESHOLD_MB: f64 = 128.0;
    const CRITICAL_FILE_THRESHOLD_MB: f64 = 64.0;

    let mut total_files = 0u64;
    let mut total_size_bytes = 0u64;
    let mut files_under_128mb = 0u64;
    let mut files_under_64mb = 0u64;
    let mut file_sizes: Vec<f64> = Vec::new();

    for snapshot in snapshots {
        if let Some(summary) = &snapshot.summary {
            if let Some(files_str) = &summary.added_data_files {
                if let Ok(files) = files_str.parse::<u32>() {
                    total_files += files as u64;
                }
            }

            if let Some(size_str) = &summary.added_files_size {
                if let Ok(size_bytes) = size_str.parse::<u64>() {
                    total_size_bytes += size_bytes;

                    // Estimate individual file sizes
                    if let Some(files_str) = &summary.added_data_files {
                        if let Ok(files) = files_str.parse::<u32>() {
                            if files > 0 {
                                let avg_file_size_bytes = size_bytes / files as u64;
                                let avg_file_size_mb = avg_file_size_bytes as f64 / (1024.0 * 1024.0);
                                file_sizes.push(avg_file_size_mb);

                                if avg_file_size_mb < SMALL_FILE_THRESHOLD_MB {
                                    files_under_128mb += files as u64;
                                }
                                if avg_file_size_mb < CRITICAL_FILE_THRESHOLD_MB {
                                    files_under_64mb += files as u64;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let average_file_size_mb = if total_files > 0 {
        (total_size_bytes as f64 / total_files as f64) / (1024.0 * 1024.0)
    } else {
        0.0
    };

    let small_file_percentage = if total_files > 0 {
        (files_under_128mb as f64 / total_files as f64) * 100.0
    } else {
        0.0
    };

    let compaction_urgency = determine_compaction_urgency(
        total_files,
        average_file_size_mb,
        small_file_percentage,
    );

    FileHealthMetrics {
        average_file_size_mb,
        small_file_percentage,
        total_files,
        files_under_128mb,
        files_under_64mb,
        largest_file_size_mb: file_sizes.iter().fold(0.0, |acc, &x| acc.max(x)),
        smallest_file_size_mb: file_sizes.iter().fold(f64::INFINITY, |acc, &x| acc.min(x)),
        compaction_urgency,
    }
}

fn determine_compaction_urgency(
    total_files: u64,
    average_file_size_mb: f64,
    small_file_percentage: f64,
) -> CompactionUrgency {
    // Based on AWS/Netflix production thresholds
    if total_files > 10000 || average_file_size_mb < 32.0 {
        CompactionUrgency::Emergency
    } else if total_files > 1000 && average_file_size_mb < 64.0 {
        CompactionUrgency::Critical
    } else if small_file_percentage > 50.0 || average_file_size_mb < 128.0 {
        CompactionUrgency::Warning
    } else {
        CompactionUrgency::Healthy
    }
}

// Additional computation functions...
```

### Success Criteria:

#### Automated Verification:
- [ ] Analytics computation completes in <100ms for 1000 snapshots: `cargo test analytics_performance`
- [ ] All unit tests pass: `cargo test analytics`
- [ ] Type checking passes: `cargo check`
- [ ] Linting passes: `cargo clippy`

#### Manual Verification:
- [ ] Analytics functions produce sensible results for test data
- [ ] Compaction urgency levels match industry thresholds
- [ ] Health score computation reflects table condition accurately

---

## Phase 2: Health Dashboard UI Components

### Overview
Create the visual dashboard components that display health metrics in an intuitive, actionable format.

### Changes Required:

#### 1. Health Dashboard Component
**File**: `src/components.rs`
**Changes**: Add comprehensive health dashboard above snapshot timeline

```rust
#[component]
fn TableHealthDashboard(
    health_metrics: TableHealthMetrics,
    on_expand_section: EventHandler<String>,
    expanded_sections: Signal<std::collections::HashSet<String>>,
) -> Element {
    rsx! {
        div {
            class: "bg-gradient-to-r from-blue-50 to-indigo-50 border border-blue-200 rounded-lg p-6 mb-6",

            // Health Score Header
            div {
                class: "flex items-center justify-between mb-6",
                div {
                    class: "flex items-center gap-3",
                    div {
                        class: format!("w-16 h-16 rounded-full flex items-center justify-center text-2xl font-bold text-white {}",
                            match health_metrics.health_score {
                                90..=100 => "bg-green-500",
                                70..=89 => "bg-yellow-500",
                                50..=69 => "bg-orange-500",
                                _ => "bg-red-500"
                            }
                        ),
                        "{health_metrics.health_score}"
                    }
                    div {
                        h3 {
                            class: "text-xl font-bold text-gray-900",
                            "Table Health Score"
                        }
                        p {
                            class: "text-sm text-gray-600",
                            {
                                match health_metrics.health_score {
                                    90..=100 => "Excellent - Table is optimally maintained",
                                    70..=89 => "Good - Minor optimizations recommended",
                                    50..=69 => "Fair - Maintenance needed",
                                    _ => "Poor - Immediate attention required"
                                }
                            }
                        }
                    }
                }

                // Quick Actions
                div {
                    class: "flex gap-2",
                    for recommendation in health_metrics.maintenance_recommendations.iter().take(2) {
                        button {
                            class: format!("px-3 py-1 text-xs rounded-full font-medium {}",
                                match recommendation.priority {
                                    RecommendationPriority::Critical => "bg-red-100 text-red-800 border border-red-200",
                                    RecommendationPriority::High => "bg-orange-100 text-orange-800 border border-orange-200",
                                    _ => "bg-blue-100 text-blue-800 border border-blue-200"
                                }
                            ),
                            title: "{recommendation.reason}",
                            "{recommendation.action}"
                        }
                    }
                }
            }

            // Metrics Grid
            div {
                class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6",

                FileHealthCard { file_health: health_metrics.file_health.clone() }
                PerformanceIndicatorCard { indicators: health_metrics.performance_indicators.clone() }
                IngestionAnalyticsCard { analytics: health_metrics.ingestion_analytics.clone() }
                StorageEfficiencyCard { efficiency: health_metrics.storage_efficiency.clone() }
            }

            // Expandable Sections
            if !health_metrics.maintenance_recommendations.is_empty() {
                MaintenanceRecommendationsSection {
                    recommendations: health_metrics.maintenance_recommendations.clone(),
                    expanded: expanded_sections.read().contains("maintenance"),
                    on_toggle: move |_| {
                        let mut sections = expanded_sections.write();
                        if sections.contains("maintenance") {
                            sections.remove("maintenance");
                        } else {
                            sections.insert("maintenance".to_string());
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FileHealthCard(file_health: FileHealthMetrics) -> Element {
    rsx! {
        div {
            class: "bg-white rounded-lg border border-gray-200 p-4",
            div {
                class: "flex items-center gap-2 mb-3",
                div {
                    class: format!("w-3 h-3 rounded-full {}",
                        match file_health.compaction_urgency {
                            CompactionUrgency::Healthy => "bg-green-500",
                            CompactionUrgency::Warning => "bg-yellow-500",
                            CompactionUrgency::Critical => "bg-orange-500",
                            CompactionUrgency::Emergency => "bg-red-500"
                        }
                    )
                }
                h4 {
                    class: "font-semibold text-gray-900",
                    "File Health"
                }
            }

            div {
                class: "space-y-2 text-sm",
                div {
                    class: "flex justify-between",
                    span { class: "text-gray-600", "Avg file size:" }
                    span { class: "font-medium", "{file_health.average_file_size_mb:.1} MB" }
                }
                div {
                    class: "flex justify-between",
                    span { class: "text-gray-600", "Small files:" }
                    span {
                        class: format!("font-medium {}",
                            if file_health.small_file_percentage > 50.0 { "text-red-600" } else { "text-gray-900" }
                        ),
                        "{file_health.small_file_percentage:.1}%"
                    }
                }
                div {
                    class: "flex justify-between",
                    span { class: "text-gray-600", "Total files:" }
                    span { class: "font-medium", "{file_health.total_files:,}" }
                }
            }
        }
    }
}

// Additional UI components for other metrics...
```

#### 2. Integration with Snapshot Timeline
**File**: `src/components.rs`
**Changes**: Integrate dashboard into existing `SnapshotTimelineTab`

```rust
// In SnapshotTimelineTab function, add dashboard above existing content
pub fn SnapshotTimelineTab(table: IcebergTable) -> Element {
    // ... existing state and logic ...

    // Compute health metrics from filtered snapshots
    let health_metrics = use_memo(move || {
        let filtered = apply_snapshot_filters(&table.snapshots, &filters());
        crate::analytics::compute_table_health_metrics(&filtered)
    });

    let mut expanded_dashboard_sections = use_signal(std::collections::HashSet::<String>::new);

    rsx! {
        div {
            class: "flex flex-col h-full",

            // Add Health Dashboard
            TableHealthDashboard {
                health_metrics: health_metrics(),
                expanded_sections: expanded_dashboard_sections,
                on_expand_section: move |section: String| {
                    // Handle section expansion
                }
            }

            // Existing snapshot timeline content...
            div {
                class: "flex-1 overflow-y-auto",
                // ... rest of existing timeline implementation
            }
        }
    }
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Component renders without errors: `cargo check`
- [ ] UI tests pass: `npm run test` (if UI tests exist)
- [ ] Dashboard components compile: `cargo build`

#### Manual Verification:
- [ ] Health dashboard displays above snapshot timeline
- [ ] Health score colors match severity (green=good, red=poor)
- [ ] All metric cards show appropriate data
- [ ] Dashboard sections expand/collapse correctly
- [ ] Performance remains smooth with 1000+ snapshots

---

## Phase 3: Advanced Analytics and Real-time Updates

### Overview
Add sophisticated analytics features including trend analysis, anomaly detection, and real-time filtering integration.

### Changes Required:

#### 1. Advanced Analytics Functions
**File**: `src/analytics.rs`
**Changes**: Add trend analysis and anomaly detection

```rust
pub fn compute_ingestion_analytics(snapshots: &[Snapshot]) -> IngestionAnalytics {
    let daily_data = group_snapshots_by_day(snapshots);
    let daily_ingestion_rate = compute_daily_ingestion_rates(&daily_data);

    let operation_pattern = analyze_operation_pattern(snapshots);
    let anomaly_score = detect_ingestion_anomalies(&daily_ingestion_rate);
    let batch_size_trend = analyze_batch_size_trend(snapshots);
    let commit_frequency_trend = analyze_commit_frequency(snapshots);

    IngestionAnalytics {
        daily_ingestion_rate,
        operation_pattern,
        anomaly_score,
        batch_size_trend,
        commit_frequency_trend,
    }
}

fn detect_ingestion_anomalies(daily_rates: &[IngestionDataPoint]) -> u8 {
    if daily_rates.len() < 7 {
        return 0; // Not enough data
    }

    // Calculate statistical outliers using median absolute deviation
    let records: Vec<u64> = daily_rates.iter().map(|d| d.records_added).collect();
    let median = calculate_median(&records);
    let mad = calculate_median_absolute_deviation(&records, median);

    let mut anomaly_count = 0;
    let threshold = 3.0; // 3 MAD threshold for outliers

    for &value in &records {
        let deviation = ((value as f64 - median).abs()) / mad;
        if deviation > threshold {
            anomaly_count += 1;
        }
    }

    // Convert to 0-100 score
    let anomaly_percentage = (anomaly_count as f64 / records.len() as f64) * 100.0;
    (anomaly_percentage.min(100.0)) as u8
}

fn analyze_operation_pattern(snapshots: &[Snapshot]) -> OperationPattern {
    let mut append_count = 0;
    let mut overwrite_count = 0;
    let mut delete_count = 0;

    for snapshot in snapshots {
        match snapshot.operation().to_lowercase().as_str() {
            "append" => append_count += 1,
            "overwrite" => overwrite_count += 1,
            "delete" => delete_count += 1,
            _ => {}
        }
    }

    let total = append_count + overwrite_count + delete_count;
    if total == 0 {
        return OperationPattern::Irregular;
    }

    let append_ratio = append_count as f64 / total as f64;
    let overwrite_ratio = overwrite_count as f64 / total as f64;

    if append_ratio > 0.8 {
        OperationPattern::StreamingAppend
    } else if overwrite_ratio > 0.6 {
        OperationPattern::BatchOverwrite
    } else if append_ratio > 0.4 && overwrite_ratio > 0.2 {
        OperationPattern::Mixed
    } else {
        OperationPattern::Irregular
    }
}

// Additional advanced analytics functions...
```

#### 2. Real-time Filter Integration
**File**: `src/components.rs`
**Changes**: Update health metrics when filters change

```rust
// In SnapshotTimelineTab, ensure health metrics update with filters
let health_metrics = use_memo(move || {
    let filtered_snapshots = apply_snapshot_filters(&table.snapshots, &filters());
    crate::analytics::compute_table_health_metrics(&filtered_snapshots)
});

// Add filter change indicator to dashboard
rsx! {
    div {
        class: "flex flex-col h-full",

        TableHealthDashboard {
            health_metrics: health_metrics(),
            expanded_sections: expanded_dashboard_sections,
            on_expand_section: move |section: String| {
                // Handle section expansion
            }
        }

        // Show filter status if active
        if !filters().is_default() {
            div {
                class: "bg-blue-50 border border-blue-200 rounded px-3 py-2 mb-4",
                div {
                    class: "flex items-center gap-2 text-sm text-blue-800",
                    svg {
                        class: "w-4 h-4",
                        fill: "currentColor",
                        view_box: "0 0 20 20",
                        path { d: "M3 4a1 1 0 011-1h12a1 1 0 011 1v2a1 1 0 01-.293.707L12 11.414V15a1 1 0 01-.293.707l-2 2A1 1 0 019 17v-5.586L3.293 6.707A1 1 0 013 6V4z" }
                    }
                    span { "Health metrics reflect filtered snapshots ({filtered_snapshots.len()} of {table.snapshots.len()})" }
                }
            }
        }

        // Rest of timeline...
    }
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Advanced analytics complete in <200ms: `cargo test advanced_analytics_performance`
- [ ] Anomaly detection accuracy verified: `cargo test anomaly_detection`
- [ ] All unit tests pass: `cargo test analytics`
- [ ] Memory usage remains stable: `cargo test memory_usage`

#### Manual Verification:
- [ ] Trend analysis shows meaningful patterns
- [ ] Anomaly detection highlights unusual ingestion patterns
- [ ] Health metrics update immediately when filters change
- [ ] Performance remains smooth during real-time updates
- [ ] Advanced insights provide actionable information

---

## Testing Strategy

### Unit Tests:
- Analytics computation accuracy with known test data
- Health score calculation edge cases (empty data, single snapshot)
- Compaction urgency thresholds match industry standards
- Anomaly detection with synthetic outlier data
- Performance benchmarks for large snapshot collections

### Integration Tests:
- End-to-end health dashboard rendering with real table data
- Filter integration and real-time metric updates
- UI responsiveness with maximum expected data volumes
- Cross-browser compatibility for dashboard visualizations

### Manual Testing Steps:
1. Load table with known small file problem - verify critical health score and recommendations
2. Apply date filters - confirm health metrics update to reflect filtered timeframe
3. Test with tables having different operation patterns - verify pattern detection accuracy
4. Load table with 1000+ snapshots - verify performance remains acceptable
5. Verify maintenance recommendations appear for problematic tables
6. Test dashboard section expansion/collapse functionality

## Performance Considerations

**Analytics Computation Optimization:**
- Use efficient algorithms for statistical calculations (median, MAD)
- Cache computed metrics until underlying data changes
- Process snapshots in streaming fashion for memory efficiency
- Use memoization for expensive trend calculations

**UI Performance:**
- Lazy-load expanded dashboard sections
- Throttle real-time updates during rapid filter changes
- Use React.memo equivalent patterns for expensive renders
- Optimize metric card re-renders with proper dependency tracking

**Memory Management:**
- Process large snapshot collections in chunks
- Clean up intermediate calculations
- Use efficient data structures for temporal analysis
- Avoid keeping full history in memory for trending

## Migration Notes

**Data Structure Evolution:**
- New analytics structs are additive, no breaking changes to existing `Snapshot` or `Summary`
- Health metrics are computed on-demand, no persistent storage changes required
- Existing filter and timeline functionality remains unchanged

**UI Migration:**
- Dashboard is inserted above existing timeline, maintaining current user workflows
- Existing snapshot tab keyboard shortcuts and interactions preserved
- New dashboard sections are collapsible to avoid overwhelming current users

**Performance Impact:**
- Analytics computation adds ~50-100ms for typical tables (100-500 snapshots)
- UI rendering adds ~20-30ms for dashboard display
- Memory usage increases by ~10-20% for health metric storage
- No impact on table loading or initial page render performance

## References

- Industry best practices: AWS Prescriptive Guidance on Apache Iceberg Monitoring
- Netflix production insights: 1.5M table deployment patterns
- Salesforce scale metrics: 4M tables, 50PB data experience
- Apache Iceberg metadata tables documentation for native monitoring capabilities
- Current implementation: `src/components.rs:785-1203` (SnapshotTimelineTab)
- Data structures: `src/data.rs:89-108` (Snapshot, Summary)
- Filter system: `src/components.rs:694-776` (apply_snapshot_filters)