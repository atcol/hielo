use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IcebergTable {
    pub name: String,
    pub namespace: String,
    pub catalog_name: String, // Track which catalog this table came from
    pub location: String,
    pub schema: TableSchema,
    pub schemas: Vec<TableSchema>, // Historical schemas
    pub snapshots: Vec<Snapshot>,
    pub current_snapshot_id: Option<u64>,
    pub properties: HashMap<String, String>,
    pub partition_spec: Option<PartitionSpec>,
    pub partition_specs: Vec<PartitionSpec>, // Historical partition specs
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableSchema {
    pub schema_id: i32,
    pub fields: Vec<NestedField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NestedField {
    pub id: i32,
    pub name: String,
    pub required: bool,
    pub field_type: DataType,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataType {
    Boolean,
    Integer,
    Long,
    Float,
    Double,
    Date,
    Time,
    Timestamp,
    TimestampTz,
    String,
    Uuid,
    Binary,
    Decimal {
        precision: u32,
        scale: u32,
    },
    Struct {
        fields: Vec<NestedField>,
    },
    List {
        element: Box<DataType>,
    },
    Map {
        key: Box<DataType>,
        value: Box<DataType>,
    },
}

impl DataType {
    pub fn to_string(&self) -> String {
        match self {
            DataType::Boolean => "boolean".to_string(),
            DataType::Integer => "int".to_string(),
            DataType::Long => "long".to_string(),
            DataType::Float => "float".to_string(),
            DataType::Double => "double".to_string(),
            DataType::Date => "date".to_string(),
            DataType::Time => "time".to_string(),
            DataType::Timestamp => "timestamp".to_string(),
            DataType::TimestampTz => "timestamptz".to_string(),
            DataType::String => "string".to_string(),
            DataType::Uuid => "uuid".to_string(),
            DataType::Binary => "binary".to_string(),
            DataType::Decimal { precision, scale } => format!("decimal({}, {})", precision, scale),
            DataType::Struct { .. } => "struct".to_string(),
            DataType::List { .. } => "list".to_string(),
            DataType::Map { .. } => "map".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub snapshot_id: u64,
    pub timestamp_ms: i64,
    pub summary: Option<Summary>,
    pub manifest_list: String,
    pub schema_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Summary {
    pub operation: String,
    pub added_data_files: Option<String>,
    pub deleted_data_files: Option<String>,
    pub added_records: Option<String>,
    pub deleted_records: Option<String>,
    pub total_records: Option<String>,
    pub added_files_size: Option<String>,
    pub removed_files_size: Option<String>,
    pub total_size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionSpec {
    pub spec_id: i32,
    pub fields: Vec<PartitionField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionField {
    pub source_id: i32,
    pub field_id: i32,
    pub name: String,
    pub transform: PartitionTransform,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PartitionTransform {
    Identity,
    Bucket { num_buckets: i32 },
    Truncate { width: i32 },
    Year,
    Month,
    Day,
    Hour,
    Void,
}

impl PartitionTransform {
    pub fn to_string(&self) -> String {
        match self {
            PartitionTransform::Identity => "identity".to_string(),
            PartitionTransform::Bucket { num_buckets } => format!("bucket[{}]", num_buckets),
            PartitionTransform::Truncate { width } => format!("truncate[{}]", width),
            PartitionTransform::Year => "year".to_string(),
            PartitionTransform::Month => "month".to_string(),
            PartitionTransform::Day => "day".to_string(),
            PartitionTransform::Hour => "hour".to_string(),
            PartitionTransform::Void => "void".to_string(),
        }
    }
}

impl Snapshot {
    pub fn timestamp(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.timestamp_ms).unwrap_or_else(Utc::now)
    }

    pub fn operation(&self) -> String {
        self.summary
            .as_ref()
            .map(|s| s.operation.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn records_added(&self) -> String {
        self.summary
            .as_ref()
            .and_then(|s| s.added_records.clone())
            .unwrap_or_else(|| "0".to_string())
    }

    pub fn size_change(&self) -> String {
        if let Some(summary) = &self.summary {
            if let (Some(added), Some(removed)) =
                (&summary.added_files_size, &summary.removed_files_size)
            {
                format!("+{} -{}", added, removed)
            } else if let Some(added) = &summary.added_files_size {
                format!("+{}", added)
            } else {
                "N/A".to_string()
            }
        } else {
            "N/A".to_string()
        }
    }
}

// Health Analytics Data Structures

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableHealthMetrics {
    pub health_score: f64,
    pub file_health: FileHealthMetrics,
    pub operational_health: OperationalHealthMetrics,
    pub storage_efficiency: StorageEfficiencyMetrics,
    pub trends: TrendMetrics,
    pub alerts: Vec<HealthAlert>,
    pub recommendations: Vec<MaintenanceRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileHealthMetrics {
    pub total_files: u64,
    pub small_files_count: u64,
    pub avg_file_size_mb: f64,
    pub file_size_distribution: FileSizeDistribution,
    pub files_per_partition_avg: f64,
    pub small_file_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileSizeDistribution {
    pub tiny_files: u64,    // < 16MB
    pub small_files: u64,   // 16MB - 64MB
    pub optimal_files: u64, // 64MB - 512MB
    pub large_files: u64,   // > 512MB
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationalHealthMetrics {
    pub snapshot_frequency: SnapshotFrequencyMetrics,
    pub operation_distribution: HashMap<String, u32>,
    pub failed_operations: u32,
    pub compaction_frequency: CompactionMetrics,
    pub time_since_last_compaction_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotFrequencyMetrics {
    pub snapshots_last_hour: u32,
    pub snapshots_last_day: u32,
    pub snapshots_last_week: u32,
    pub avg_snapshots_per_hour: f64,
    pub peak_snapshots_per_hour: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactionMetrics {
    pub days_since_last: Option<f64>,
    pub compactions_last_week: u32,
    pub avg_compaction_frequency_days: f64,
    pub compaction_effectiveness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageEfficiencyMetrics {
    pub total_size_gb: f64,
    pub storage_growth_rate_gb_per_day: f64,
    pub delete_ratio: f64,
    pub update_ratio: f64,
    pub data_freshness_hours: f64,
    pub partition_efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrendMetrics {
    pub file_count_trend: TrendDirection,
    pub avg_file_size_trend: TrendDirection,
    pub snapshot_frequency_trend: TrendDirection,
    pub storage_growth_trend: TrendDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthAlert {
    pub severity: AlertSeverity,
    pub category: AlertCategory,
    pub message: String,
    pub metric_value: f64,
    pub threshold: f64,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertCategory {
    SmallFiles,
    HighSnapshotFrequency,
    StorageGrowth,
    CompactionNeeded,
    PerformanceDegradation,
    DataFreshness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaintenanceRecommendation {
    pub priority: MaintenancePriority,
    pub action_type: MaintenanceActionType,
    pub description: String,
    pub estimated_benefit: String,
    pub effort_level: MaintenanceEffort,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaintenancePriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaintenanceActionType {
    Compaction,
    PartitionEvolution,
    SchemaEvolution,
    RetentionPolicy,
    Optimization,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaintenanceEffort {
    Low,     // < 1 hour
    Medium,  // 1-4 hours
    High,    // 1-2 days
    Complex, // > 2 days
}
