use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IcebergTable {
    pub name: String,
    pub namespace: String,
    pub location: String,
    pub schema: TableSchema,
    pub snapshots: Vec<Snapshot>,
    pub current_snapshot_id: Option<u64>,
    pub properties: HashMap<String, String>,
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
    Decimal { precision: u32, scale: u32 },
    Struct { fields: Vec<NestedField> },
    List { element: Box<DataType> },
    Map { key: Box<DataType>, value: Box<DataType> },
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

impl Snapshot {
    pub fn timestamp(&self) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(self.timestamp_ms)
            .unwrap_or_else(Utc::now)
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
            if let (Some(added), Some(removed)) = (&summary.added_files_size, &summary.removed_files_size) {
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

// Sample data generation for demo purposes
pub fn generate_sample_table() -> IcebergTable {
    let now = Utc::now();
    
    let schema = TableSchema {
        schema_id: 1,
        fields: vec![
            NestedField {
                id: 1,
                name: "user_id".to_string(),
                required: true,
                field_type: DataType::Long,
                doc: Some("Unique identifier for the user".to_string()),
            },
            NestedField {
                id: 2,
                name: "email".to_string(),
                required: true,
                field_type: DataType::String,
                doc: Some("User's email address".to_string()),
            },
            NestedField {
                id: 3,
                name: "created_at".to_string(),
                required: true,
                field_type: DataType::TimestampTz,
                doc: Some("Account creation timestamp".to_string()),
            },
            NestedField {
                id: 4,
                name: "age".to_string(),
                required: false,
                field_type: DataType::Integer,
                doc: Some("User's age in years".to_string()),
            },
            NestedField {
                id: 5,
                name: "balance".to_string(),
                required: false,
                field_type: DataType::Decimal { precision: 10, scale: 2 },
                doc: Some("Account balance".to_string()),
            },
            NestedField {
                id: 6,
                name: "preferences".to_string(),
                required: false,
                field_type: DataType::Struct {
                    fields: vec![
                        NestedField {
                            id: 7,
                            name: "theme".to_string(),
                            required: false,
                            field_type: DataType::String,
                            doc: Some("UI theme preference".to_string()),
                        },
                        NestedField {
                            id: 8,
                            name: "notifications".to_string(),
                            required: false,
                            field_type: DataType::Boolean,
                            doc: Some("Notification preferences".to_string()),
                        },
                    ],
                },
                doc: Some("User preferences structure".to_string()),
            },
        ],
    };

    let snapshots = vec![
        Snapshot {
            snapshot_id: 1001,
            timestamp_ms: (now - Duration::days(30)).timestamp_millis(),
            summary: Some(Summary {
                operation: "append".to_string(),
                added_data_files: Some("5".to_string()),
                deleted_data_files: Some("0".to_string()),
                added_records: Some("10000".to_string()),
                deleted_records: Some("0".to_string()),
                total_records: Some("10000".to_string()),
                added_files_size: Some("2.4MB".to_string()),
                removed_files_size: Some("0B".to_string()),
                total_size: Some("2.4MB".to_string()),
            }),
            manifest_list: "s3://bucket/warehouse/users/metadata/snap-1001-manifest-list.avro".to_string(),
            schema_id: Some(1),
        },
        Snapshot {
            snapshot_id: 1002,
            timestamp_ms: (now - Duration::days(25)).timestamp_millis(),
            summary: Some(Summary {
                operation: "append".to_string(),
                added_data_files: Some("3".to_string()),
                deleted_data_files: Some("0".to_string()),
                added_records: Some("5000".to_string()),
                deleted_records: Some("0".to_string()),
                total_records: Some("15000".to_string()),
                added_files_size: Some("1.2MB".to_string()),
                removed_files_size: Some("0B".to_string()),
                total_size: Some("3.6MB".to_string()),
            }),
            manifest_list: "s3://bucket/warehouse/users/metadata/snap-1002-manifest-list.avro".to_string(),
            schema_id: Some(1),
        },
        Snapshot {
            snapshot_id: 1003,
            timestamp_ms: (now - Duration::days(20)).timestamp_millis(),
            summary: Some(Summary {
                operation: "delete".to_string(),
                added_data_files: Some("2".to_string()),
                deleted_data_files: Some("1".to_string()),
                added_records: Some("0".to_string()),
                deleted_records: Some("500".to_string()),
                total_records: Some("14500".to_string()),
                added_files_size: Some("0.8MB".to_string()),
                removed_files_size: Some("0.4MB".to_string()),
                total_size: Some("4.0MB".to_string()),
            }),
            manifest_list: "s3://bucket/warehouse/users/metadata/snap-1003-manifest-list.avro".to_string(),
            schema_id: Some(1),
        },
        Snapshot {
            snapshot_id: 1004,
            timestamp_ms: (now - Duration::days(15)).timestamp_millis(),
            summary: Some(Summary {
                operation: "overwrite".to_string(),
                added_data_files: Some("8".to_string()),
                deleted_data_files: Some("6".to_string()),
                added_records: Some("14500".to_string()),
                deleted_records: Some("14500".to_string()),
                total_records: Some("14500".to_string()),
                added_files_size: Some("3.2MB".to_string()),
                removed_files_size: Some("4.0MB".to_string()),
                total_size: Some("3.2MB".to_string()),
            }),
            manifest_list: "s3://bucket/warehouse/users/metadata/snap-1004-manifest-list.avro".to_string(),
            schema_id: Some(1),
        },
        Snapshot {
            snapshot_id: 1005,
            timestamp_ms: (now - Duration::days(5)).timestamp_millis(),
            summary: Some(Summary {
                operation: "append".to_string(),
                added_data_files: Some("4".to_string()),
                deleted_data_files: Some("0".to_string()),
                added_records: Some("7500".to_string()),
                deleted_records: Some("0".to_string()),
                total_records: Some("22000".to_string()),
                added_files_size: Some("1.8MB".to_string()),
                removed_files_size: Some("0B".to_string()),
                total_size: Some("5.0MB".to_string()),
            }),
            manifest_list: "s3://bucket/warehouse/users/metadata/snap-1005-manifest-list.avro".to_string(),
            schema_id: Some(1),
        },
    ];

    let mut properties = HashMap::new();
    properties.insert("write.format.default".to_string(), "parquet".to_string());
    properties.insert("write.parquet.compression-codec".to_string(), "zstd".to_string());
    properties.insert("commit.retry.num-retries".to_string(), "3".to_string());
    properties.insert("commit.manifest.target-size-bytes".to_string(), "8388608".to_string());

    IcebergTable {
        name: "users".to_string(),
        namespace: "analytics.prod".to_string(),
        location: "s3://data-lake/warehouse/analytics/prod/users".to_string(),
        schema,
        snapshots,
        current_snapshot_id: Some(1005),
        properties,
    }
}
