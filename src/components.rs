use dioxus::prelude::*;
use crate::data::{IcebergTable, NestedField, DataType};

#[component]
pub fn TableInfoTab(table: IcebergTable) -> Element {
    rsx! {
        div {
            class: "space-y-6",
            
            // Table Overview
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                        "Table Overview"
                    }
                    dl {
                        class: "grid grid-cols-1 gap-x-4 gap-y-6 sm:grid-cols-2",
                        div {
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Name"
                            }
                            dd {
                                class: "mt-1 text-sm text-gray-900 font-mono",
                                "{table.name}"
                            }
                        }
                        div {
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Namespace"
                            }
                            dd {
                                class: "mt-1 text-sm text-gray-900 font-mono",
                                "{table.namespace}"
                            }
                        }
                        div {
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Location"
                            }
                            dd {
                                class: "mt-1 text-sm text-gray-900 font-mono break-all",
                                "{table.location}"
                            }
                        }
                        div {
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Current Snapshot"
                            }
                            dd {
                                class: "mt-1 text-sm text-gray-900",
                                {table.current_snapshot_id.map_or("None".to_string(), |id| id.to_string())}
                            }
                        }
                        div {
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Schema ID"
                            }
                            dd {
                                class: "mt-1 text-sm text-gray-900",
                                "{table.schema.schema_id}"
                            }
                        }
                        div {
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Total Snapshots"
                            }
                            dd {
                                class: "mt-1 text-sm text-gray-900",
                                "{table.snapshots.len()}"
                            }
                        }
                    }
                }
            }

            // Schema
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                        "Schema"
                    }
                    div {
                        class: "overflow-x-auto",
                        table {
                            class: "min-w-full divide-y divide-gray-200",
                            thead {
                                class: "bg-gray-50",
                                tr {
                                    th {
                                        class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "ID"
                                    }
                                    th {
                                        class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Name"
                                    }
                                    th {
                                        class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Type"
                                    }
                                    th {
                                        class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Required"
                                    }
                                    th {
                                        class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Description"
                                    }
                                }
                            }
                            tbody {
                                class: "bg-white divide-y divide-gray-200",
                                for field in &table.schema.fields {
                                    SchemaFieldRow { field: field.clone(), depth: 0 }
                                }
                            }
                        }
                    }
                }
            }

            // Table Properties
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                        "Table Properties"
                    }
                    div {
                        class: "overflow-x-auto",
                        table {
                            class: "min-w-full divide-y divide-gray-200",
                            thead {
                                class: "bg-gray-50",
                                tr {
                                    th {
                                        class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Property"
                                    }
                                    th {
                                        class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                        "Value"
                                    }
                                }
                            }
                            tbody {
                                class: "bg-white divide-y divide-gray-200",
                                for (key, value) in &table.properties {
                                    tr {
                                        td {
                                            class: "px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 font-mono",
                                            "{key}"
                                        }
                                        td {
                                            class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500 font-mono",
                                            "{value}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SchemaFieldRow(field: NestedField, depth: usize) -> Element {
    let indent_class = format!("pl-{}", depth * 4);
    
    rsx! {
        tr {
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900",
                "{field.id}"
            }
            td {
                class: format!("px-6 py-4 whitespace-nowrap text-sm text-gray-900 {}", indent_class),
                span {
                    class: if depth > 0 { "text-gray-600" } else { "font-medium" },
                    "{field.name}"
                }
            }
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                span {
                    class: "inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800",
                    {field.field_type.to_string()}
                }
            }
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                if field.required {
                    span {
                        class: "inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-red-100 text-red-800",
                        "Required"
                    }
                } else {
                    span {
                        class: "inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-gray-100 text-gray-800",
                        "Optional"
                    }
                }
            }
            td {
                class: "px-6 py-4 text-sm text-gray-500",
                {field.doc.unwrap_or_else(|| "—".to_string())}
            }
        }
        
        // Render nested fields for struct types
        if let DataType::Struct { fields } = &field.field_type {
            for nested_field in fields {
                SchemaFieldRow { field: nested_field.clone(), depth: depth + 1 }
            }
        }
    }
}

#[component]
pub fn SnapshotTimelineTab(table: IcebergTable) -> Element {
    let mut sorted_snapshots = table.snapshots.clone();
    sorted_snapshots.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms)); // Most recent first

    rsx! {
        div {
            class: "space-y-6",
            
            // Timeline Header
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900 mb-2",
                        "Snapshot History"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "Timeline showing all table snapshots from most recent to oldest"
                    }
                }
            }

            // Timeline
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    div {
                        class: "flow-root",
                        ul {
                            role: "list",
                            class: "relative",
                            for (index, snapshot) in sorted_snapshots.iter().enumerate() {
                                li {
                                    class: "timeline-item",
                                    div {
                                        class: "relative flex space-x-3",
                                        div {
                                            class: "min-w-0 flex-1",
                                            div {
                                                class: "flex items-center justify-between",
                                                div {
                                                    class: "flex items-center space-x-3",
                                                    h4 {
                                                        class: "text-sm font-medium text-gray-900",
                                                        "Snapshot {snapshot.snapshot_id}"
                                                    }
                                                    span {
                                                        class: format!(
                                                            "inline-flex px-2 py-1 text-xs font-semibold rounded-full {}",
                                                            match snapshot.operation().as_str() {
                                                                "append" => "bg-green-100 text-green-800",
                                                                "overwrite" => "bg-yellow-100 text-yellow-800",
                                                                "delete" => "bg-red-100 text-red-800",
                                                                _ => "bg-gray-100 text-gray-800",
                                                            }
                                                        ),
                                                        "{snapshot.operation()}"
                                                    }
                                                    if table.current_snapshot_id == Some(snapshot.snapshot_id) {
                                                        span {
                                                            class: "inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800",
                                                            "CURRENT"
                                                        }
                                                    }
                                                }
                                                p {
                                                    class: "text-sm text-gray-500",
                                                    {snapshot.timestamp().format("%Y-%m-%d %H:%M:%S UTC").to_string()}
                                                }
                                            }
                                            div {
                                                class: "mt-2 grid grid-cols-1 gap-x-4 gap-y-2 sm:grid-cols-4",
                                                div {
                                                    class: "text-sm",
                                                    span {
                                                        class: "font-medium text-gray-500",
                                                        "Records Added: "
                                                    }
                                                    span {
                                                        class: "text-gray-900",
                                                        "{snapshot.records_added()}"
                                                    }
                                                }
                                                div {
                                                    class: "text-sm",
                                                    span {
                                                        class: "font-medium text-gray-500",
                                                        "Size Change: "
                                                    }
                                                    span {
                                                        class: "text-gray-900",
                                                        "{snapshot.size_change()}"
                                                    }
                                                }
                                                if let Some(summary) = &snapshot.summary {
                                                    div {
                                                        class: "text-sm",
                                                        span {
                                                            class: "font-medium text-gray-500",
                                                            "Files Added: "
                                                        }
                                                        span {
                                                            class: "text-gray-900",
                                                            {summary.added_data_files.clone().unwrap_or_else(|| "0".to_string())}
                                                        }
                                                    }
                                                    div {
                                                        class: "text-sm",
                                                        span {
                                                            class: "font-medium text-gray-500",
                                                            "Total Records: "
                                                        }
                                                        span {
                                                            class: "text-gray-900",
                                                            {summary.total_records.clone().unwrap_or_else(|| "N/A".to_string())}
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(summary) = &snapshot.summary {
                                                if !snapshot.manifest_list.is_empty() {
                                                    div {
                                                        class: "mt-2",
                                                        p {
                                                            class: "text-xs text-gray-400 font-mono break-all",
                                                            "Manifest: {snapshot.manifest_list}"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Summary Statistics
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                        "Timeline Summary"
                    }
                    dl {
                        class: "grid grid-cols-1 gap-x-4 gap-y-6 sm:grid-cols-3",
                        div {
                            class: "text-center",
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Total Snapshots"
                            }
                            dd {
                                class: "mt-1 text-2xl font-semibold text-gray-900",
                                "{table.snapshots.len()}"
                            }
                        }
                        div {
                            class: "text-center",
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Operations"
                            }
                            dd {
                                class: "mt-1 text-sm text-gray-900",
                                {
                                    let mut ops = std::collections::HashMap::new();
                                    for snapshot in &table.snapshots {
                                        *ops.entry(snapshot.operation()).or_insert(0) += 1;
                                    }
                                    ops.iter()
                                        .map(|(k, v)| format!("{}: {}", k, v))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                }
                            }
                        }
                        div {
                            class: "text-center",
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                "Time Span"
                            }
                            dd {
                                class: "mt-1 text-sm text-gray-900",
                                {
                                    if let (Some(oldest), Some(newest)) = (
                                        table.snapshots.iter().min_by_key(|s| s.timestamp_ms),
                                        table.snapshots.iter().max_by_key(|s| s.timestamp_ms),
                                    ) {
                                        let days = (newest.timestamp_ms - oldest.timestamp_ms) / (24 * 60 * 60 * 1000);
                                        format!("{} days", days)
                                    } else {
                                        "N/A".to_string()
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
