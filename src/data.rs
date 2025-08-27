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
