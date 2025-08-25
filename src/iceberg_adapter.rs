use crate::data::{DataType, IcebergTable, NestedField, Snapshot, Summary, TableSchema};
use anyhow::Result;
use iceberg::spec::{PrimitiveType, SchemaRef, Type};
use iceberg::table::Table;
use std::collections::HashMap;

/// Convert an iceberg-rust Table to our internal IcebergTable representation
pub fn convert_iceberg_table(table: &Table, namespace: String) -> Result<IcebergTable> {
    let metadata = table.metadata();

    // Convert schema
    let schema = convert_schema(metadata.current_schema())?;

    // Convert snapshots
    let snapshots = metadata
        .snapshots()
        .map(|snapshot| convert_snapshot(snapshot))
        .collect::<Result<Vec<_>>>()?;

    // Get table properties
    let properties = metadata.properties().clone();

    // Get current snapshot ID
    let current_snapshot_id = metadata.current_snapshot().map(|s| s.snapshot_id() as u64);

    Ok(IcebergTable {
        name: table.identifier().name().to_string(),
        namespace,
        location: metadata.location().to_string(),
        schema,
        snapshots,
        current_snapshot_id,
        properties,
    })
}

fn convert_schema(schema: &SchemaRef) -> Result<TableSchema> {
    let fields = schema
        .as_struct()
        .fields()
        .iter()
        .map(|field| convert_field(field))
        .collect::<Result<Vec<_>>>()?;

    Ok(TableSchema {
        schema_id: schema.schema_id(),
        fields,
    })
}

fn convert_field(field: &iceberg::spec::NestedField) -> Result<NestedField> {
    Ok(NestedField {
        id: field.id,
        name: field.name.clone(),
        required: field.required,
        field_type: convert_data_type(&field.field_type)?,
        doc: field.doc.clone(),
    })
}

fn convert_data_type(iceberg_type: &Type) -> Result<DataType> {
    match iceberg_type {
        Type::Primitive(primitive) => Ok(convert_primitive_type(primitive)),
        Type::Struct(struct_type) => {
            let fields = struct_type
                .fields()
                .iter()
                .map(|field| convert_field(field))
                .collect::<Result<Vec<_>>>()?;
            Ok(DataType::Struct { fields })
        }
        Type::List(list_type) => {
            let element_type = convert_data_type(&list_type.element_field.field_type)?;
            Ok(DataType::List {
                element: Box::new(element_type),
            })
        }
        Type::Map(map_type) => {
            let key_type = convert_data_type(&map_type.key_field.field_type)?;
            let value_type = convert_data_type(&map_type.value_field.field_type)?;
            Ok(DataType::Map {
                key: Box::new(key_type),
                value: Box::new(value_type),
            })
        }
    }
}

fn convert_primitive_type(primitive: &PrimitiveType) -> DataType {
    match primitive {
        PrimitiveType::Boolean => DataType::Boolean,
        PrimitiveType::Int => DataType::Integer,
        PrimitiveType::Long => DataType::Long,
        PrimitiveType::Float => DataType::Float,
        PrimitiveType::Double => DataType::Double,
        PrimitiveType::Date => DataType::Date,
        PrimitiveType::Time => DataType::Time,
        PrimitiveType::Timestamp => DataType::Timestamp,
        PrimitiveType::Timestamptz => DataType::TimestampTz,
        PrimitiveType::String => DataType::String,
        PrimitiveType::Uuid => DataType::Uuid,
        PrimitiveType::Fixed(_) => DataType::Binary,
        PrimitiveType::Binary => DataType::Binary,
        PrimitiveType::Decimal { precision, scale } => DataType::Decimal {
            precision: *precision as u32,
            scale: *scale as u32,
        },
        PrimitiveType::TimestampNs => todo!(),
        PrimitiveType::TimestamptzNs => todo!(),
    }
}

fn convert_snapshot(snapshot: &iceberg::spec::Snapshot) -> Result<Snapshot> {
    let summary = Some(convert_summary(&snapshot.summary().additional_properties));

    Ok(Snapshot {
        snapshot_id: snapshot.snapshot_id() as u64,
        timestamp_ms: snapshot.timestamp_ms(),
        summary,
        manifest_list: snapshot.manifest_list().to_string(),
        schema_id: snapshot.schema_id().map(|id| id),
    })
}

fn convert_summary(summary: &HashMap<String, String>) -> Summary {
    Summary {
        operation: summary
            .get("operation")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string()),
        added_data_files: summary.get("added-data-files").cloned(),
        deleted_data_files: summary.get("deleted-data-files").cloned(),
        added_records: summary.get("added-records").cloned(),
        deleted_records: summary.get("deleted-records").cloned(),
        total_records: summary.get("total-records").cloned(),
        added_files_size: summary.get("added-files-size").cloned(),
        removed_files_size: summary.get("removed-files-size").cloned(),
        total_size: summary.get("total-size").cloned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_primitive_types() {
        assert_eq!(
            convert_primitive_type(&PrimitiveType::Boolean),
            DataType::Boolean
        );
        assert_eq!(
            convert_primitive_type(&PrimitiveType::Int),
            DataType::Integer
        );
        assert_eq!(
            convert_primitive_type(&PrimitiveType::String),
            DataType::String
        );

        let decimal = PrimitiveType::Decimal {
            precision: 10,
            scale: 2,
        };
        assert_eq!(
            convert_primitive_type(&decimal),
            DataType::Decimal {
                precision: 10,
                scale: 2
            }
        );
    }
}
