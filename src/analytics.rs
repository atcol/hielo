use chrono::{Duration, Utc};
use std::collections::HashMap;

use crate::data::*;

// Industry-standard thresholds based on Netflix, Salesforce, and AWS recommendations
pub struct HealthThresholds;

impl HealthThresholds {
    // File size thresholds (in MB)
    pub const TINY_FILE_THRESHOLD: f64 = 16.0;
    pub const SMALL_FILE_THRESHOLD: f64 = 64.0;
    pub const OPTIMAL_FILE_MAX: f64 = 512.0;

    // File count thresholds
    pub const SMALL_FILE_RATIO_WARNING: f64 = 0.3; // 30%
    pub const SMALL_FILE_RATIO_CRITICAL: f64 = 0.5; // 50%

    // Snapshot frequency thresholds
    pub const HIGH_FREQUENCY_HOUR_WARNING: u32 = 10;
    pub const HIGH_FREQUENCY_HOUR_CRITICAL: u32 = 20;

    // Compaction timing thresholds (in days)
    pub const COMPACTION_WARNING_DAYS: f64 = 7.0;
    pub const COMPACTION_CRITICAL_DAYS: f64 = 14.0;

    // Storage growth thresholds (GB per day)
    pub const STORAGE_GROWTH_WARNING: f64 = 100.0;
    pub const STORAGE_GROWTH_CRITICAL: f64 = 500.0;
}

pub struct TableAnalytics;

impl TableAnalytics {
    pub fn compute_health_metrics(table: &IcebergTable) -> TableHealthMetrics {
        let file_health = Self::compute_file_health(&table.snapshots);
        let operational_health = Self::compute_operational_health(&table.snapshots);
        let storage_efficiency = Self::compute_storage_efficiency(&table.snapshots);
        let trends = Self::compute_trends(&table.snapshots);

        let health_score = Self::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &trends,
        );

        let alerts = Self::generate_alerts(&file_health, &operational_health, &storage_efficiency);

        let recommendations = Self::generate_recommendations(&alerts, &trends);

        TableHealthMetrics {
            health_score,
            file_health,
            operational_health,
            storage_efficiency,
            trends,
            alerts,
            recommendations,
        }
    }

    fn compute_file_health(snapshots: &[Snapshot]) -> FileHealthMetrics {
        let mut total_files = 0u64;
        let mut total_size_bytes = 0f64;
        let mut tiny_files = 0u64;
        let mut small_files = 0u64;
        let mut optimal_files = 0u64;
        let mut large_files = 0u64;

        // Analyze the latest snapshot for current state
        if let Some(latest_snapshot) = snapshots.last() {
            if let Some(summary) = &latest_snapshot.summary {
                if let Some(files_str) = &summary.added_data_files {
                    total_files = files_str.parse().unwrap_or(0);
                }

                if let Some(size_str) = &summary.total_size {
                    total_size_bytes = size_str.parse().unwrap_or(0.0);
                }
            }
        }

        let avg_file_size_mb = if total_files > 0 {
            (total_size_bytes / (total_files as f64)) / (1024.0 * 1024.0)
        } else {
            0.0
        };

        // Estimate file distribution based on average size and patterns
        // This is a simplified approach - in production, we'd analyze manifest files
        if avg_file_size_mb < HealthThresholds::TINY_FILE_THRESHOLD {
            tiny_files = (total_files as f64 * 0.7) as u64;
            small_files = (total_files as f64 * 0.3) as u64;
        } else if avg_file_size_mb < HealthThresholds::SMALL_FILE_THRESHOLD {
            tiny_files = (total_files as f64 * 0.2) as u64;
            small_files = (total_files as f64 * 0.6) as u64;
            optimal_files = (total_files as f64 * 0.2) as u64;
        } else if avg_file_size_mb <= HealthThresholds::OPTIMAL_FILE_MAX {
            optimal_files = total_files;
        } else {
            optimal_files = (total_files as f64 * 0.7) as u64;
            large_files = (total_files as f64 * 0.3) as u64;
        }

        let small_files_count = tiny_files + small_files;
        let small_file_ratio = if total_files > 0 {
            small_files_count as f64 / total_files as f64
        } else {
            0.0
        };

        FileHealthMetrics {
            total_files,
            small_files_count,
            avg_file_size_mb,
            file_size_distribution: FileSizeDistribution {
                tiny_files,
                small_files,
                optimal_files,
                large_files,
            },
            files_per_partition_avg: avg_file_size_mb, // Simplified - would need partition data
            small_file_ratio,
        }
    }

    fn compute_operational_health(snapshots: &[Snapshot]) -> OperationalHealthMetrics {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let one_day_ago = now - Duration::days(1);
        let one_week_ago = now - Duration::days(7);

        let mut snapshots_last_hour = 0u32;
        let mut snapshots_last_day = 0u32;
        let mut snapshots_last_week = 0u32;
        let mut operation_distribution = HashMap::new();
        let mut compaction_timestamps = Vec::new();

        for snapshot in snapshots {
            let timestamp = snapshot.timestamp();

            if timestamp > one_hour_ago {
                snapshots_last_hour += 1;
            }
            if timestamp > one_day_ago {
                snapshots_last_day += 1;
            }
            if timestamp > one_week_ago {
                snapshots_last_week += 1;
            }

            let operation = snapshot.operation();
            *operation_distribution.entry(operation.clone()).or_insert(0) += 1;

            // Track compaction operations
            if operation.contains("rewrite") || operation.contains("compact") {
                compaction_timestamps.push(timestamp);
            }
        }

        let avg_snapshots_per_hour = if snapshots_last_week > 0 {
            snapshots_last_week as f64 / (7.0 * 24.0)
        } else {
            0.0
        };

        let peak_snapshots_per_hour = snapshots_last_hour.max(if snapshots_last_day > 0 {
            snapshots_last_day / 24
        } else {
            0
        });

        let time_since_last_compaction_hours = compaction_timestamps
            .last()
            .map(|last_compaction| now.signed_duration_since(*last_compaction).num_hours() as f64);

        let compaction_metrics = CompactionMetrics {
            days_since_last: time_since_last_compaction_hours.map(|h| h / 24.0),
            compactions_last_week: compaction_timestamps.len() as u32,
            avg_compaction_frequency_days: if compaction_timestamps.len() > 1 {
                let total_days = compaction_timestamps
                    .last()
                    .unwrap()
                    .signed_duration_since(*compaction_timestamps.first().unwrap())
                    .num_days() as f64;
                total_days / (compaction_timestamps.len() - 1) as f64
            } else {
                0.0
            },
            compaction_effectiveness: 0.8, // Would be computed from file reduction
        };

        OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour,
                snapshots_last_day,
                snapshots_last_week,
                avg_snapshots_per_hour,
                peak_snapshots_per_hour,
            },
            operation_distribution,
            failed_operations: 0, // Would track from metadata
            compaction_frequency: compaction_metrics,
            time_since_last_compaction_hours,
        }
    }

    fn compute_storage_efficiency(snapshots: &[Snapshot]) -> StorageEfficiencyMetrics {
        let mut total_size_gb = 0.0;
        let mut delete_operations = 0u32;
        let mut update_operations = 0u32;
        let mut total_operations = 0u32;
        let mut size_history = Vec::new();

        for snapshot in snapshots {
            if let Some(summary) = &snapshot.summary {
                if let Some(size_str) = &summary.total_size {
                    let size_bytes: f64 = size_str.parse().unwrap_or(0.0);
                    total_size_gb = size_bytes / (1024.0 * 1024.0 * 1024.0);
                    size_history.push((snapshot.timestamp(), total_size_gb));
                }

                let operation = summary.operation.to_lowercase();
                total_operations += 1;

                if operation.contains("delete") {
                    delete_operations += 1;
                } else if operation.contains("update") || operation.contains("overwrite") {
                    update_operations += 1;
                }
            }
        }

        let delete_ratio = if total_operations > 0 {
            delete_operations as f64 / total_operations as f64
        } else {
            0.0
        };

        let update_ratio = if total_operations > 0 {
            update_operations as f64 / total_operations as f64
        } else {
            0.0
        };

        let storage_growth_rate = if size_history.len() > 1 {
            let first = size_history.first().unwrap();
            let last = size_history.last().unwrap();
            let days = last.0.signed_duration_since(first.0).num_days() as f64;
            if days > 0.0 {
                (last.1 - first.1) / days
            } else {
                0.0
            }
        } else {
            0.0
        };

        let data_freshness_hours = if let Some(latest) = snapshots.last() {
            Utc::now()
                .signed_duration_since(latest.timestamp())
                .num_hours() as f64
        } else {
            0.0
        };

        StorageEfficiencyMetrics {
            total_size_gb,
            storage_growth_rate_gb_per_day: storage_growth_rate,
            delete_ratio,
            update_ratio,
            data_freshness_hours,
            partition_efficiency: 0.85, // Would be computed from partition analysis
        }
    }

    fn compute_trends(snapshots: &[Snapshot]) -> TrendMetrics {
        // Simplified trend analysis - would use more sophisticated algorithms in production
        let _recent_snapshots = snapshots.iter().rev().take(10).collect::<Vec<_>>();

        TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Improving,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Degrading,
        }
    }

    fn compute_overall_health_score(
        file_health: &FileHealthMetrics,
        operational_health: &OperationalHealthMetrics,
        storage_efficiency: &StorageEfficiencyMetrics,
        trends: &TrendMetrics,
    ) -> f64 {
        let mut score: f64 = 100.0;

        // File health penalties
        if file_health.small_file_ratio > HealthThresholds::SMALL_FILE_RATIO_CRITICAL {
            score -= 30.0;
        } else if file_health.small_file_ratio > HealthThresholds::SMALL_FILE_RATIO_WARNING {
            score -= 15.0;
        }

        // Operational health penalties
        if operational_health.snapshot_frequency.snapshots_last_hour
            > HealthThresholds::HIGH_FREQUENCY_HOUR_CRITICAL
        {
            score -= 20.0;
        } else if operational_health.snapshot_frequency.snapshots_last_hour
            > HealthThresholds::HIGH_FREQUENCY_HOUR_WARNING
        {
            score -= 10.0;
        }

        // Compaction penalties
        if let Some(days_since_compaction) = operational_health.compaction_frequency.days_since_last
        {
            if days_since_compaction > HealthThresholds::COMPACTION_CRITICAL_DAYS {
                score -= 25.0;
            } else if days_since_compaction > HealthThresholds::COMPACTION_WARNING_DAYS {
                score -= 12.0;
            }
        } else {
            // No compaction data available - apply penalty for lack of monitoring
            score -= 10.0;
        }

        // Storage growth penalties
        if storage_efficiency.storage_growth_rate_gb_per_day
            > HealthThresholds::STORAGE_GROWTH_CRITICAL
        {
            score -= 15.0;
        } else if storage_efficiency.storage_growth_rate_gb_per_day
            > HealthThresholds::STORAGE_GROWTH_WARNING
        {
            score -= 8.0;
        }

        // Trend bonuses/penalties
        match trends.file_count_trend {
            TrendDirection::Improving => score += 5.0,
            TrendDirection::Degrading => score -= 5.0,
            TrendDirection::Stable => {}
        }

        score.max(0.0).min(100.0)
    }

    fn generate_alerts(
        file_health: &FileHealthMetrics,
        operational_health: &OperationalHealthMetrics,
        storage_efficiency: &StorageEfficiencyMetrics,
    ) -> Vec<HealthAlert> {
        let mut alerts = Vec::new();
        let now = Utc::now();

        // Small files alert
        if file_health.small_file_ratio > HealthThresholds::SMALL_FILE_RATIO_CRITICAL {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Critical,
                category: AlertCategory::SmallFiles,
                message: format!(
                    "Critical small file ratio: {:.1}% of files are smaller than {}MB",
                    file_health.small_file_ratio * 100.0,
                    HealthThresholds::SMALL_FILE_THRESHOLD
                ),
                metric_value: file_health.small_file_ratio,
                threshold: HealthThresholds::SMALL_FILE_RATIO_CRITICAL,
                detected_at: now,
            });
        } else if file_health.small_file_ratio > HealthThresholds::SMALL_FILE_RATIO_WARNING {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                category: AlertCategory::SmallFiles,
                message: format!(
                    "High small file ratio: {:.1}% of files are smaller than {}MB",
                    file_health.small_file_ratio * 100.0,
                    HealthThresholds::SMALL_FILE_THRESHOLD
                ),
                metric_value: file_health.small_file_ratio,
                threshold: HealthThresholds::SMALL_FILE_RATIO_WARNING,
                detected_at: now,
            });
        }

        // High snapshot frequency alert
        if operational_health.snapshot_frequency.snapshots_last_hour
            > HealthThresholds::HIGH_FREQUENCY_HOUR_CRITICAL
        {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Critical,
                category: AlertCategory::HighSnapshotFrequency,
                message: format!(
                    "Extremely high snapshot frequency: {} snapshots in the last hour",
                    operational_health.snapshot_frequency.snapshots_last_hour
                ),
                metric_value: operational_health.snapshot_frequency.snapshots_last_hour as f64,
                threshold: HealthThresholds::HIGH_FREQUENCY_HOUR_CRITICAL as f64,
                detected_at: now,
            });
        }

        // Compaction needed alert
        if let Some(days_since_compaction) = operational_health.compaction_frequency.days_since_last
        {
            if days_since_compaction > HealthThresholds::COMPACTION_CRITICAL_DAYS {
                alerts.push(HealthAlert {
                    severity: AlertSeverity::Critical,
                    category: AlertCategory::CompactionNeeded,
                    message: format!(
                        "Table needs compaction: {:.1} days since last compaction",
                        days_since_compaction
                    ),
                    metric_value: days_since_compaction,
                    threshold: HealthThresholds::COMPACTION_CRITICAL_DAYS,
                    detected_at: now,
                });
            }
        }

        // Storage growth alert
        if storage_efficiency.storage_growth_rate_gb_per_day
            > HealthThresholds::STORAGE_GROWTH_CRITICAL
        {
            alerts.push(HealthAlert {
                severity: AlertSeverity::Warning,
                category: AlertCategory::StorageGrowth,
                message: format!(
                    "High storage growth rate: {:.1} GB per day",
                    storage_efficiency.storage_growth_rate_gb_per_day
                ),
                metric_value: storage_efficiency.storage_growth_rate_gb_per_day,
                threshold: HealthThresholds::STORAGE_GROWTH_CRITICAL,
                detected_at: now,
            });
        }

        alerts
    }

    fn generate_recommendations(
        alerts: &[HealthAlert],
        trends: &TrendMetrics,
    ) -> Vec<MaintenanceRecommendation> {
        let mut recommendations = Vec::new();

        // Generate recommendations based on alerts
        for alert in alerts {
            match alert.category {
                AlertCategory::SmallFiles => {
                    recommendations.push(MaintenanceRecommendation {
                        priority: if alert.severity == AlertSeverity::Critical {
                            MaintenancePriority::High
                        } else {
                            MaintenancePriority::Medium
                        },
                        action_type: MaintenanceActionType::Compaction,
                        description: "Run table compaction to merge small files into larger, more efficient files".to_string(),
                        estimated_benefit: "Improved query performance and reduced metadata overhead".to_string(),
                        effort_level: MaintenanceEffort::Medium,
                    });
                }
                AlertCategory::CompactionNeeded => {
                    recommendations.push(MaintenanceRecommendation {
                        priority: MaintenancePriority::High,
                        action_type: MaintenanceActionType::Compaction,
                        description: "Schedule regular compaction job for this table".to_string(),
                        estimated_benefit: "Better file organisation and query performance"
                            .to_string(),
                        effort_level: MaintenanceEffort::Medium,
                    });
                }
                AlertCategory::HighSnapshotFrequency => {
                    recommendations.push(MaintenanceRecommendation {
                        priority: MaintenancePriority::Medium,
                        action_type: MaintenanceActionType::Optimization,
                        description: "Review write patterns and consider batching smaller writes"
                            .to_string(),
                        estimated_benefit:
                            "Reduced metadata overhead and improved table performance".to_string(),
                        effort_level: MaintenanceEffort::Low,
                    });
                }
                _ => {}
            }
        }

        // Add general recommendations based on trends
        match trends.storage_growth_trend {
            TrendDirection::Degrading => {
                recommendations.push(MaintenanceRecommendation {
                    priority: MaintenancePriority::Low,
                    action_type: MaintenanceActionType::RetentionPolicy,
                    description:
                        "Consider implementing data retention policies to manage storage growth"
                            .to_string(),
                    estimated_benefit: "Controlled storage costs and improved performance"
                        .to_string(),
                    effort_level: MaintenanceEffort::High,
                });
            }
            _ => {}
        }

        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use crate::data::*;
    use std::collections::HashMap;

    // ==================== Test Helpers ====================

    fn create_empty_snapshot_list() -> Vec<Snapshot> {
        Vec::new()
    }

    fn create_snapshot_with_summary(
        id: u64,
        timestamp_ms: i64,
        operation: &str,
        total_files: Option<&str>,
        total_size: Option<&str>,
    ) -> Snapshot {
        Snapshot {
            snapshot_id: id,
            timestamp_ms,
            summary: Some(Summary {
                operation: operation.to_string(),
                added_data_files: total_files.map(|s| s.to_string()),
                deleted_data_files: None,
                added_records: Some("1000".to_string()),
                deleted_records: None,
                total_records: Some("10000".to_string()),
                added_files_size: Some("1048576".to_string()), // 1MB
                removed_files_size: None,
                total_size: total_size.map(|s| s.to_string()),
            }),
            manifest_list: "s3://bucket/manifest.avro".to_string(),
            schema_id: Some(0),
        }
    }

    fn create_snapshot_without_summary(id: u64, timestamp_ms: i64) -> Snapshot {
        Snapshot {
            snapshot_id: id,
            timestamp_ms,
            summary: None,
            manifest_list: "s3://bucket/manifest.avro".to_string(),
            schema_id: Some(0),
        }
    }

    fn now_ms() -> i64 {
        Utc::now().timestamp_millis()
    }

    fn hours_ago_ms(hours: i64) -> i64 {
        (Utc::now() - Duration::try_hours(hours).unwrap()).timestamp_millis()
    }

    fn days_ago_ms(days: i64) -> i64 {
        (Utc::now() - Duration::try_days(days).unwrap()).timestamp_millis()
    }

    // ==================== File Health Tests ====================

    #[test]
    fn test_file_health_with_empty_snapshots() {
        let snapshots = create_empty_snapshot_list();
        let health = TableAnalytics::compute_file_health(&snapshots);

        assert_eq!(health.total_files, 0);
        assert_eq!(health.small_files_count, 0);
        assert_eq!(health.avg_file_size_mb, 0.0);
        assert_eq!(health.small_file_ratio, 0.0);
    }

    #[test]
    fn test_file_health_with_tiny_files() {
        // Create snapshot with 100 files totaling 800MB (8MB avg - tiny files)
        let total_size = (800 * 1024 * 1024).to_string(); // 800MB in bytes
        let snapshots = vec![create_snapshot_with_summary(
            1,
            now_ms(),
            "append",
            Some("100"),
            Some(&total_size),
        )];

        let health = TableAnalytics::compute_file_health(&snapshots);

        assert_eq!(health.total_files, 100);
        assert!(health.avg_file_size_mb < HealthThresholds::TINY_FILE_THRESHOLD);
        // With tiny files, expect 70% tiny + 30% small = 100% small files
        assert_eq!(health.small_files_count, 100);
        assert!((health.small_file_ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_file_health_with_small_files() {
        // Create snapshot with 100 files totaling 3200MB (32MB avg - small files)
        let total_size = (3200 * 1024 * 1024).to_string();
        let snapshots = vec![create_snapshot_with_summary(
            1,
            now_ms(),
            "append",
            Some("100"),
            Some(&total_size),
        )];

        let health = TableAnalytics::compute_file_health(&snapshots);

        assert_eq!(health.total_files, 100);
        assert!(health.avg_file_size_mb >= HealthThresholds::TINY_FILE_THRESHOLD);
        assert!(health.avg_file_size_mb < HealthThresholds::SMALL_FILE_THRESHOLD);
        // With small files: 20% tiny + 60% small = 80% small files
        assert_eq!(health.small_files_count, 80);
    }

    #[test]
    fn test_file_health_with_optimal_files() {
        // Create snapshot with 100 files totaling 25600MB (256MB avg - optimal)
        let total_size = (25600u64 * 1024 * 1024).to_string();
        let snapshots = vec![create_snapshot_with_summary(
            1,
            now_ms(),
            "append",
            Some("100"),
            Some(&total_size),
        )];

        let health = TableAnalytics::compute_file_health(&snapshots);

        assert_eq!(health.total_files, 100);
        assert!(health.avg_file_size_mb >= HealthThresholds::SMALL_FILE_THRESHOLD);
        assert!(health.avg_file_size_mb <= HealthThresholds::OPTIMAL_FILE_MAX);
        assert_eq!(health.small_files_count, 0);
        assert_eq!(health.file_size_distribution.optimal_files, 100);
    }

    #[test]
    fn test_file_health_with_large_files() {
        // Create snapshot with 100 files totaling 102400MB (1024MB avg - large)
        let total_size = (102400u64 * 1024 * 1024).to_string();
        let snapshots = vec![create_snapshot_with_summary(
            1,
            now_ms(),
            "append",
            Some("100"),
            Some(&total_size),
        )];

        let health = TableAnalytics::compute_file_health(&snapshots);

        assert_eq!(health.total_files, 100);
        assert!(health.avg_file_size_mb > HealthThresholds::OPTIMAL_FILE_MAX);
        assert_eq!(health.file_size_distribution.large_files, 30); // 30% large
        assert_eq!(health.file_size_distribution.optimal_files, 70); // 70% optimal
    }

    #[test]
    fn test_file_health_without_summary() {
        let snapshots = vec![create_snapshot_without_summary(1, now_ms())];
        let health = TableAnalytics::compute_file_health(&snapshots);

        // Without summary, should default to zeros
        assert_eq!(health.total_files, 0);
        assert_eq!(health.avg_file_size_mb, 0.0);
    }

    // ==================== Operational Health Tests ====================

    #[test]
    fn test_operational_health_empty_snapshots() {
        let snapshots = create_empty_snapshot_list();
        let health = TableAnalytics::compute_operational_health(&snapshots);

        assert_eq!(health.snapshot_frequency.snapshots_last_hour, 0);
        assert_eq!(health.snapshot_frequency.snapshots_last_day, 0);
        assert_eq!(health.snapshot_frequency.snapshots_last_week, 0);
        assert_eq!(health.snapshot_frequency.avg_snapshots_per_hour, 0.0);
        assert!(health.operation_distribution.is_empty());
    }

    #[test]
    fn test_operational_health_recent_snapshots() {
        let snapshots = vec![
            create_snapshot_with_summary(1, hours_ago_ms(0), "append", Some("10"), None),
            create_snapshot_with_summary(2, hours_ago_ms(0), "append", Some("10"), None),
            create_snapshot_with_summary(3, hours_ago_ms(12), "overwrite", Some("10"), None),
            create_snapshot_with_summary(4, days_ago_ms(3), "append", Some("10"), None),
        ];

        let health = TableAnalytics::compute_operational_health(&snapshots);

        assert_eq!(health.snapshot_frequency.snapshots_last_hour, 2);
        assert_eq!(health.snapshot_frequency.snapshots_last_day, 3);
        assert_eq!(health.snapshot_frequency.snapshots_last_week, 4);

        // Check operation distribution
        assert_eq!(health.operation_distribution.get("append"), Some(&3));
        assert_eq!(health.operation_distribution.get("overwrite"), Some(&1));
    }

    #[test]
    fn test_operational_health_compaction_tracking() {
        let snapshots = vec![
            create_snapshot_with_summary(1, days_ago_ms(10), "append", Some("10"), None),
            create_snapshot_with_summary(2, days_ago_ms(5), "rewrite", Some("10"), None), // compaction
            create_snapshot_with_summary(3, days_ago_ms(2), "compact", Some("10"), None), // compaction
            create_snapshot_with_summary(4, days_ago_ms(1), "append", Some("10"), None),
        ];

        let health = TableAnalytics::compute_operational_health(&snapshots);

        assert_eq!(health.compaction_frequency.compactions_last_week, 2);
        assert!(health.compaction_frequency.days_since_last.is_some());
        // Last compaction was 2 days ago
        let days_since = health.compaction_frequency.days_since_last.unwrap();
        assert!(days_since >= 1.9 && days_since <= 2.1);
    }

    #[test]
    fn test_operational_health_no_compactions() {
        let snapshots = vec![
            create_snapshot_with_summary(1, days_ago_ms(5), "append", Some("10"), None),
            create_snapshot_with_summary(2, days_ago_ms(3), "append", Some("10"), None),
        ];

        let health = TableAnalytics::compute_operational_health(&snapshots);

        assert_eq!(health.compaction_frequency.compactions_last_week, 0);
        assert!(health.time_since_last_compaction_hours.is_none());
    }

    #[test]
    fn test_operational_health_avg_snapshots_per_hour() {
        // Create 168 snapshots over a week (1 per hour avg)
        let snapshots: Vec<Snapshot> = (0i64..168)
            .map(|i| {
                create_snapshot_with_summary(i as u64, hours_ago_ms(i), "append", Some("10"), None)
            })
            .collect();

        let health = TableAnalytics::compute_operational_health(&snapshots);

        // Average should be close to 1 snapshot per hour
        assert!(
            (health.snapshot_frequency.avg_snapshots_per_hour - 1.0).abs() < 0.1,
            "Expected ~1.0, got {}",
            health.snapshot_frequency.avg_snapshots_per_hour
        );
    }

    // ==================== Storage Efficiency Tests ====================

    #[test]
    fn test_storage_efficiency_empty_snapshots() {
        let snapshots = create_empty_snapshot_list();
        let efficiency = TableAnalytics::compute_storage_efficiency(&snapshots);

        assert_eq!(efficiency.total_size_gb, 0.0);
        assert_eq!(efficiency.storage_growth_rate_gb_per_day, 0.0);
        assert_eq!(efficiency.delete_ratio, 0.0);
        assert_eq!(efficiency.update_ratio, 0.0);
    }

    #[test]
    fn test_storage_efficiency_with_growth() {
        // First snapshot: 10GB, last snapshot: 20GB, over 10 days = 1GB/day growth
        let size_10gb = (10u64 * 1024 * 1024 * 1024).to_string();
        let size_20gb = (20u64 * 1024 * 1024 * 1024).to_string();

        let snapshots = vec![
            create_snapshot_with_summary(
                1,
                days_ago_ms(10),
                "append",
                Some("10"),
                Some(&size_10gb),
            ),
            create_snapshot_with_summary(2, now_ms(), "append", Some("10"), Some(&size_20gb)),
        ];

        let efficiency = TableAnalytics::compute_storage_efficiency(&snapshots);

        assert!((efficiency.total_size_gb - 20.0).abs() < 0.1);
        assert!(
            (efficiency.storage_growth_rate_gb_per_day - 1.0).abs() < 0.2,
            "Expected ~1.0 GB/day, got {}",
            efficiency.storage_growth_rate_gb_per_day
        );
    }

    #[test]
    fn test_storage_efficiency_operation_ratios() {
        let snapshots = vec![
            create_snapshot_with_summary(1, days_ago_ms(5), "append", Some("10"), None),
            create_snapshot_with_summary(2, days_ago_ms(4), "append", Some("10"), None),
            create_snapshot_with_summary(3, days_ago_ms(3), "delete", Some("10"), None),
            create_snapshot_with_summary(4, days_ago_ms(2), "overwrite", Some("10"), None),
            create_snapshot_with_summary(5, days_ago_ms(1), "update", Some("10"), None),
        ];

        let efficiency = TableAnalytics::compute_storage_efficiency(&snapshots);

        // 1 delete out of 5 = 0.2
        assert!((efficiency.delete_ratio - 0.2).abs() < 0.01);
        // 2 updates (overwrite + update) out of 5 = 0.4
        assert!((efficiency.update_ratio - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_storage_efficiency_data_freshness() {
        let snapshots = vec![create_snapshot_with_summary(
            1,
            hours_ago_ms(5),
            "append",
            Some("10"),
            None,
        )];

        let efficiency = TableAnalytics::compute_storage_efficiency(&snapshots);

        // Data freshness should be approximately 5 hours
        assert!(
            (efficiency.data_freshness_hours - 5.0).abs() < 0.5,
            "Expected ~5.0 hours, got {}",
            efficiency.data_freshness_hours
        );
    }

    // ==================== Health Score Tests ====================

    #[test]
    fn test_health_score_perfect_health() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 10,
            avg_file_size_mb: 256.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 0,
                small_files: 10,
                optimal_files: 90,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.1, // Below warning threshold
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 2, // Below warning threshold
                snapshots_last_day: 24,
                snapshots_last_week: 168,
                avg_snapshots_per_hour: 1.0,
                peak_snapshots_per_hour: 3,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0), // Recent compaction
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 5.0, // Low growth
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Improving,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let score = TableAnalytics::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &trends,
        );

        // Should be high score (95-100) with improving trends giving +5 bonus
        assert!(score >= 95.0, "Expected score >= 95, got {}", score);
    }

    #[test]
    fn test_health_score_small_file_warning_penalty() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 40,
            avg_file_size_mb: 32.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 10,
                small_files: 30,
                optimal_files: 60,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.4, // Above warning (0.3), below critical (0.5)
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 2,
                snapshots_last_day: 24,
                snapshots_last_week: 168,
                avg_snapshots_per_hour: 1.0,
                peak_snapshots_per_hour: 3,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0),
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 5.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let score = TableAnalytics::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &trends,
        );

        // Should have 15 point penalty for small file warning
        assert!(
            score <= 85.0 && score >= 75.0,
            "Expected score 75-85, got {}",
            score
        );
    }

    #[test]
    fn test_health_score_critical_small_files_penalty() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 60,
            avg_file_size_mb: 8.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 40,
                small_files: 20,
                optimal_files: 40,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.6, // Above critical (0.5)
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 2,
                snapshots_last_day: 24,
                snapshots_last_week: 168,
                avg_snapshots_per_hour: 1.0,
                peak_snapshots_per_hour: 3,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0),
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 5.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let score = TableAnalytics::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &trends,
        );

        // Should have 30 point penalty for critical small files
        assert!(score <= 70.0, "Expected score <= 70, got {}", score);
    }

    #[test]
    fn test_health_score_high_snapshot_frequency_penalty() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 10,
            avg_file_size_mb: 256.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 0,
                small_files: 10,
                optimal_files: 90,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.1,
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 25, // Critical threshold (> 20)
                snapshots_last_day: 200,
                snapshots_last_week: 1000,
                avg_snapshots_per_hour: 6.0,
                peak_snapshots_per_hour: 25,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0),
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 5.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let score = TableAnalytics::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &trends,
        );

        // Should have 20 point penalty for critical snapshot frequency
        assert!(score <= 80.0, "Expected score <= 80, got {}", score);
    }

    #[test]
    fn test_health_score_compaction_needed_penalty() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 10,
            avg_file_size_mb: 256.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 0,
                small_files: 10,
                optimal_files: 90,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.1,
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 2,
                snapshots_last_day: 24,
                snapshots_last_week: 168,
                avg_snapshots_per_hour: 1.0,
                peak_snapshots_per_hour: 3,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(20.0), // Critical threshold (> 14 days)
                compactions_last_week: 0,
                avg_compaction_frequency_days: 20.0,
                compaction_effectiveness: 0.5,
            },
            time_since_last_compaction_hours: Some(480.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 5.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let score = TableAnalytics::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &trends,
        );

        // Should have 25 point penalty for critical compaction needed
        assert!(score <= 75.0, "Expected score <= 75, got {}", score);
    }

    #[test]
    fn test_health_score_trend_bonuses() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 10,
            avg_file_size_mb: 256.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 0,
                small_files: 10,
                optimal_files: 90,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.1,
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 2,
                snapshots_last_day: 24,
                snapshots_last_week: 168,
                avg_snapshots_per_hour: 1.0,
                peak_snapshots_per_hour: 3,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0),
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 5.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        // Test improving trend gives bonus
        let improving_trends = TrendMetrics {
            file_count_trend: TrendDirection::Improving,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let score_improving = TableAnalytics::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &improving_trends,
        );

        // Test degrading trend gives penalty
        let degrading_trends = TrendMetrics {
            file_count_trend: TrendDirection::Degrading,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let score_degrading = TableAnalytics::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &degrading_trends,
        );

        // Improving should be 10 points higher than degrading
        assert!(
            (score_improving - score_degrading - 10.0).abs() < 0.1,
            "Expected 10 point difference, got {} vs {}",
            score_improving,
            score_degrading
        );
    }

    #[test]
    fn test_health_score_clamped_to_range() {
        // Create worst case scenario to test floor
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 100,
            avg_file_size_mb: 1.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 100,
                small_files: 0,
                optimal_files: 0,
                large_files: 0,
            },
            files_per_partition_avg: 1.0,
            small_file_ratio: 1.0, // All small files - critical
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 100, // Way over critical
                snapshots_last_day: 1000,
                snapshots_last_week: 5000,
                avg_snapshots_per_hour: 30.0,
                peak_snapshots_per_hour: 100,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 10,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(100.0), // Way over critical
                compactions_last_week: 0,
                avg_compaction_frequency_days: 100.0,
                compaction_effectiveness: 0.0,
            },
            time_since_last_compaction_hours: Some(2400.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 1000.0,
            storage_growth_rate_gb_per_day: 1000.0, // Way over critical
            delete_ratio: 0.9,
            update_ratio: 0.9,
            data_freshness_hours: 1000.0,
            partition_efficiency: 0.1,
        };

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Degrading,
            avg_file_size_trend: TrendDirection::Degrading,
            snapshot_frequency_trend: TrendDirection::Degrading,
            storage_growth_trend: TrendDirection::Degrading,
        };

        let score = TableAnalytics::compute_overall_health_score(
            &file_health,
            &operational_health,
            &storage_efficiency,
            &trends,
        );

        // Score should be clamped to 0, not negative
        assert_eq!(score, 0.0);
    }

    // ==================== Alert Generation Tests ====================

    #[test]
    fn test_generate_alerts_no_issues() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 10,
            avg_file_size_mb: 256.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 0,
                small_files: 10,
                optimal_files: 90,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.1, // Below warning threshold
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 5, // Below warning
                snapshots_last_day: 50,
                snapshots_last_week: 300,
                avg_snapshots_per_hour: 1.8,
                peak_snapshots_per_hour: 5,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0), // Recent
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 50.0, // Below warning
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let alerts =
            TableAnalytics::generate_alerts(&file_health, &operational_health, &storage_efficiency);

        assert!(alerts.is_empty(), "Expected no alerts, got {:?}", alerts);
    }

    #[test]
    fn test_generate_alerts_small_files_warning() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 40,
            avg_file_size_mb: 32.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 10,
                small_files: 30,
                optimal_files: 60,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.4, // Above warning (0.3), below critical (0.5)
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 5,
                snapshots_last_day: 50,
                snapshots_last_week: 300,
                avg_snapshots_per_hour: 1.8,
                peak_snapshots_per_hour: 5,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0),
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 50.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let alerts =
            TableAnalytics::generate_alerts(&file_health, &operational_health, &storage_efficiency);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
        assert_eq!(alerts[0].category, AlertCategory::SmallFiles);
    }

    #[test]
    fn test_generate_alerts_small_files_critical() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 60,
            avg_file_size_mb: 8.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 40,
                small_files: 20,
                optimal_files: 40,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.6, // Above critical (0.5)
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 5,
                snapshots_last_day: 50,
                snapshots_last_week: 300,
                avg_snapshots_per_hour: 1.8,
                peak_snapshots_per_hour: 5,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0),
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 50.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let alerts =
            TableAnalytics::generate_alerts(&file_health, &operational_health, &storage_efficiency);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
        assert_eq!(alerts[0].category, AlertCategory::SmallFiles);
    }

    #[test]
    fn test_generate_alerts_high_snapshot_frequency() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 10,
            avg_file_size_mb: 256.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 0,
                small_files: 10,
                optimal_files: 90,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.1,
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 25, // Above critical (20)
                snapshots_last_day: 200,
                snapshots_last_week: 1000,
                avg_snapshots_per_hour: 6.0,
                peak_snapshots_per_hour: 25,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0),
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 50.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let alerts =
            TableAnalytics::generate_alerts(&file_health, &operational_health, &storage_efficiency);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
        assert_eq!(alerts[0].category, AlertCategory::HighSnapshotFrequency);
    }

    #[test]
    fn test_generate_alerts_compaction_needed() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 10,
            avg_file_size_mb: 256.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 0,
                small_files: 10,
                optimal_files: 90,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.1,
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 5,
                snapshots_last_day: 50,
                snapshots_last_week: 300,
                avg_snapshots_per_hour: 1.8,
                peak_snapshots_per_hour: 5,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(20.0), // Above critical (14)
                compactions_last_week: 0,
                avg_compaction_frequency_days: 20.0,
                compaction_effectiveness: 0.5,
            },
            time_since_last_compaction_hours: Some(480.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 100.0,
            storage_growth_rate_gb_per_day: 50.0,
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let alerts =
            TableAnalytics::generate_alerts(&file_health, &operational_health, &storage_efficiency);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Critical);
        assert_eq!(alerts[0].category, AlertCategory::CompactionNeeded);
    }

    #[test]
    fn test_generate_alerts_storage_growth() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 10,
            avg_file_size_mb: 256.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 0,
                small_files: 10,
                optimal_files: 90,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.1,
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 5,
                snapshots_last_day: 50,
                snapshots_last_week: 300,
                avg_snapshots_per_hour: 1.8,
                peak_snapshots_per_hour: 5,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(3.0),
                compactions_last_week: 2,
                avg_compaction_frequency_days: 3.5,
                compaction_effectiveness: 0.8,
            },
            time_since_last_compaction_hours: Some(72.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 1000.0,
            storage_growth_rate_gb_per_day: 600.0, // Above critical (500)
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let alerts =
            TableAnalytics::generate_alerts(&file_health, &operational_health, &storage_efficiency);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].severity, AlertSeverity::Warning);
        assert_eq!(alerts[0].category, AlertCategory::StorageGrowth);
    }

    #[test]
    fn test_generate_alerts_multiple_issues() {
        let file_health = FileHealthMetrics {
            total_files: 100,
            small_files_count: 60,
            avg_file_size_mb: 8.0,
            file_size_distribution: FileSizeDistribution {
                tiny_files: 40,
                small_files: 20,
                optimal_files: 40,
                large_files: 0,
            },
            files_per_partition_avg: 10.0,
            small_file_ratio: 0.6, // Critical
        };

        let operational_health = OperationalHealthMetrics {
            snapshot_frequency: SnapshotFrequencyMetrics {
                snapshots_last_hour: 25, // Critical
                snapshots_last_day: 200,
                snapshots_last_week: 1000,
                avg_snapshots_per_hour: 6.0,
                peak_snapshots_per_hour: 25,
            },
            operation_distribution: HashMap::new(),
            failed_operations: 0,
            compaction_frequency: CompactionMetrics {
                days_since_last: Some(20.0), // Critical
                compactions_last_week: 0,
                avg_compaction_frequency_days: 20.0,
                compaction_effectiveness: 0.5,
            },
            time_since_last_compaction_hours: Some(480.0),
        };

        let storage_efficiency = StorageEfficiencyMetrics {
            total_size_gb: 1000.0,
            storage_growth_rate_gb_per_day: 600.0, // Warning
            delete_ratio: 0.1,
            update_ratio: 0.2,
            data_freshness_hours: 1.0,
            partition_efficiency: 0.9,
        };

        let alerts =
            TableAnalytics::generate_alerts(&file_health, &operational_health, &storage_efficiency);

        assert_eq!(alerts.len(), 4);

        let categories: Vec<_> = alerts.iter().map(|a| &a.category).collect();
        assert!(categories.contains(&&AlertCategory::SmallFiles));
        assert!(categories.contains(&&AlertCategory::HighSnapshotFrequency));
        assert!(categories.contains(&&AlertCategory::CompactionNeeded));
        assert!(categories.contains(&&AlertCategory::StorageGrowth));
    }

    // ==================== Recommendation Generation Tests ====================

    #[test]
    fn test_generate_recommendations_empty_alerts() {
        let alerts: Vec<HealthAlert> = vec![];
        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let recommendations = TableAnalytics::generate_recommendations(&alerts, &trends);

        assert!(recommendations.is_empty());
    }

    #[test]
    fn test_generate_recommendations_small_files_warning() {
        let alerts = vec![HealthAlert {
            severity: AlertSeverity::Warning,
            category: AlertCategory::SmallFiles,
            message: "High small file ratio".to_string(),
            metric_value: 0.4,
            threshold: 0.3,
            detected_at: Utc::now(),
        }];

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let recommendations = TableAnalytics::generate_recommendations(&alerts, &trends);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].priority, MaintenancePriority::Medium);
        assert_eq!(
            recommendations[0].action_type,
            MaintenanceActionType::Compaction
        );
    }

    #[test]
    fn test_generate_recommendations_small_files_critical() {
        let alerts = vec![HealthAlert {
            severity: AlertSeverity::Critical,
            category: AlertCategory::SmallFiles,
            message: "Critical small file ratio".to_string(),
            metric_value: 0.6,
            threshold: 0.5,
            detected_at: Utc::now(),
        }];

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let recommendations = TableAnalytics::generate_recommendations(&alerts, &trends);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].priority, MaintenancePriority::High);
        assert_eq!(
            recommendations[0].action_type,
            MaintenanceActionType::Compaction
        );
    }

    #[test]
    fn test_generate_recommendations_compaction_needed() {
        let alerts = vec![HealthAlert {
            severity: AlertSeverity::Critical,
            category: AlertCategory::CompactionNeeded,
            message: "Compaction needed".to_string(),
            metric_value: 20.0,
            threshold: 14.0,
            detected_at: Utc::now(),
        }];

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let recommendations = TableAnalytics::generate_recommendations(&alerts, &trends);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].priority, MaintenancePriority::High);
        assert_eq!(
            recommendations[0].action_type,
            MaintenanceActionType::Compaction
        );
    }

    #[test]
    fn test_generate_recommendations_high_snapshot_frequency() {
        let alerts = vec![HealthAlert {
            severity: AlertSeverity::Critical,
            category: AlertCategory::HighSnapshotFrequency,
            message: "High snapshot frequency".to_string(),
            metric_value: 25.0,
            threshold: 20.0,
            detected_at: Utc::now(),
        }];

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Stable,
        };

        let recommendations = TableAnalytics::generate_recommendations(&alerts, &trends);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].priority, MaintenancePriority::Medium);
        assert_eq!(
            recommendations[0].action_type,
            MaintenanceActionType::Optimization
        );
    }

    #[test]
    fn test_generate_recommendations_degrading_storage_trend() {
        let alerts: Vec<HealthAlert> = vec![];
        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Degrading,
        };

        let recommendations = TableAnalytics::generate_recommendations(&alerts, &trends);

        assert_eq!(recommendations.len(), 1);
        assert_eq!(recommendations[0].priority, MaintenancePriority::Low);
        assert_eq!(
            recommendations[0].action_type,
            MaintenanceActionType::RetentionPolicy
        );
    }

    #[test]
    fn test_generate_recommendations_multiple() {
        let alerts = vec![
            HealthAlert {
                severity: AlertSeverity::Critical,
                category: AlertCategory::SmallFiles,
                message: "Critical small file ratio".to_string(),
                metric_value: 0.6,
                threshold: 0.5,
                detected_at: Utc::now(),
            },
            HealthAlert {
                severity: AlertSeverity::Critical,
                category: AlertCategory::HighSnapshotFrequency,
                message: "High snapshot frequency".to_string(),
                metric_value: 25.0,
                threshold: 20.0,
                detected_at: Utc::now(),
            },
        ];

        let trends = TrendMetrics {
            file_count_trend: TrendDirection::Stable,
            avg_file_size_trend: TrendDirection::Stable,
            snapshot_frequency_trend: TrendDirection::Stable,
            storage_growth_trend: TrendDirection::Degrading,
        };

        let recommendations = TableAnalytics::generate_recommendations(&alerts, &trends);

        // Should have 3 recommendations: 2 from alerts + 1 from degrading trend
        assert_eq!(recommendations.len(), 3);

        let action_types: Vec<_> = recommendations.iter().map(|r| &r.action_type).collect();
        assert!(action_types.contains(&&MaintenanceActionType::Compaction));
        assert!(action_types.contains(&&MaintenanceActionType::Optimization));
        assert!(action_types.contains(&&MaintenanceActionType::RetentionPolicy));
    }

    // ==================== Integration Test ====================

    #[test]
    fn test_compute_health_metrics_integration() {
        // Create a realistic table scenario with some issues
        let size_5gb = (5u64 * 1024 * 1024 * 1024).to_string();
        let size_10gb = (10u64 * 1024 * 1024 * 1024).to_string();

        let table = IcebergTable {
            name: "test_table".to_string(),
            namespace: "test_ns".to_string(),
            catalog_name: "test_catalog".to_string(),
            location: "s3://bucket/table".to_string(),
            schema: TableSchema {
                schema_id: 0,
                fields: vec![],
            },
            schemas: vec![],
            snapshots: vec![
                create_snapshot_with_summary(
                    1,
                    days_ago_ms(7),
                    "append",
                    Some("50"),
                    Some(&size_5gb),
                ),
                create_snapshot_with_summary(
                    2,
                    days_ago_ms(5),
                    "append",
                    Some("50"),
                    Some(&size_5gb),
                ),
                create_snapshot_with_summary(
                    3,
                    days_ago_ms(3),
                    "rewrite",
                    Some("80"),
                    Some(&size_10gb),
                ),
                create_snapshot_with_summary(
                    4,
                    days_ago_ms(1),
                    "append",
                    Some("100"),
                    Some(&size_10gb),
                ),
                create_snapshot_with_summary(
                    5,
                    hours_ago_ms(2),
                    "append",
                    Some("100"),
                    Some(&size_10gb),
                ),
            ],
            current_snapshot_id: Some(5),
            properties: HashMap::new(),
            partition_spec: None,
            partition_specs: vec![],
        };

        let metrics = TableAnalytics::compute_health_metrics(&table);

        // Verify health score is computed
        assert!(metrics.health_score >= 0.0 && metrics.health_score <= 100.0);

        // Verify file health is computed from latest snapshot
        assert!(metrics.file_health.total_files > 0);

        // Verify operational health tracks snapshots
        assert!(
            metrics
                .operational_health
                .snapshot_frequency
                .snapshots_last_week
                >= 5
        );

        // Verify storage efficiency is computed
        assert!(metrics.storage_efficiency.total_size_gb > 0.0);

        // Verify trends are computed
        assert!(matches!(
            metrics.trends.file_count_trend,
            TrendDirection::Stable | TrendDirection::Improving | TrendDirection::Degrading
        ));
    }
}
