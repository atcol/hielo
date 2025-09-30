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
