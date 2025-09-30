use crate::analytics::TableAnalytics;
use crate::data::{
    AlertSeverity, DataType, IcebergTable, NestedField, PartitionField, Snapshot,
    TableHealthMetrics,
};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SnapshotFilters {
    pub operation_types: Vec<String>, // Selected operation types
    pub files_added_min: Option<u32>,
    pub files_added_max: Option<u32>,
    pub records_added_min: Option<u64>,
    pub records_added_max: Option<u64>,
    pub date_start: Option<String>, // ISO date string
    pub date_end: Option<String>,   // ISO date string
}

impl Default for SnapshotFilters {
    fn default() -> Self {
        Self {
            operation_types: vec![
                "append".to_string(),
                "overwrite".to_string(),
                "delete".to_string(),
            ],
            files_added_min: None,
            files_added_max: None,
            records_added_min: None,
            records_added_max: None,
            date_start: None,
            date_end: None,
        }
    }
}

#[component]
fn OperationTypeFilter(
    selected_types: Vec<String>,
    on_change: EventHandler<Vec<String>>,
) -> Element {
    let mut is_open = use_signal(|| false);
    let available_operations = vec![
        ("append", "Append", "bg-green-100 text-green-800"),
        ("overwrite", "Overwrite", "bg-yellow-100 text-yellow-800"),
        ("delete", "Delete", "bg-red-100 text-red-800"),
    ];

    let selected_types_clone = selected_types.clone();
    let button_text = if selected_types.is_empty() {
        "None selected".to_string()
    } else {
        format!("{} selected", selected_types.len())
    };

    rsx! {
        div {
            class: "relative",
            label {
                class: "block text-sm font-medium text-gray-700 mb-1",
                "Operation Type"
            }
            button {
                onclick: move |_| is_open.set(!is_open()),
                class: "w-full bg-white border border-gray-300 rounded-md px-3 py-2 text-left shadow-sm focus:outline-none focus:ring-1 focus:ring-blue-500",
                "{button_text}"
                svg {
                    class: "ml-2 h-5 w-5 text-gray-400 absolute right-2 top-1/2 transform -translate-y-1/2",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        stroke_width: "2",
                        d: "M19 9l-7 7-7-7"
                    }
                }
            }
            if is_open() {
                div {
                    class: "absolute z-10 mt-1 w-full bg-white shadow-lg max-h-60 rounded-md py-1 ring-1 ring-black ring-opacity-5",
                    for (op_value, op_name, op_class) in available_operations {
                        label {
                            class: "flex items-center px-3 py-2 hover:bg-gray-100 cursor-pointer",
                            input {
                                r#type: "checkbox",
                                checked: selected_types_clone.contains(&op_value.to_string()),
                                onchange: {
                                    let op_val = op_value.to_string();
                                    let types_for_closure = selected_types_clone.clone();
                                    move |evt: Event<FormData>| {
                                        let mut new_types = types_for_closure.clone();
                                        if evt.checked() {
                                            if !new_types.contains(&op_val) {
                                                new_types.push(op_val.clone());
                                            }
                                        } else {
                                            new_types.retain(|t| t != &op_val);
                                        }
                                        on_change.call(new_types);
                                    }
                                },
                                class: "mr-2"
                            }
                            span {
                                class: format!("px-2 py-1 rounded-full text-xs font-medium {}", op_class),
                                "{op_name}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn NumberRangeFilter(
    label: String,
    min_value: Option<u64>,
    max_value: Option<u64>,
    on_min_change: EventHandler<Option<u64>>,
    on_max_change: EventHandler<Option<u64>>,
    placeholder_min: String,
    placeholder_max: String,
) -> Element {
    rsx! {
        div {
            label {
                class: "block text-sm font-medium text-gray-700 mb-1",
                "{label}"
            }
            div {
                class: "flex space-x-2",
                input {
                    r#type: "number",
                    value: "{min_value.map(|v| v.to_string()).unwrap_or_default()}",
                    oninput: move |evt| {
                        let val = if evt.value().is_empty() {
                            None
                        } else {
                            evt.value().parse::<u64>().ok()
                        };
                        on_min_change.call(val);
                    },
                    placeholder: "{placeholder_min}",
                    class: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
                }
                span {
                    class: "self-center text-gray-500 text-sm",
                    "to"
                }
                input {
                    r#type: "number",
                    value: "{max_value.map(|v| v.to_string()).unwrap_or_default()}",
                    oninput: move |evt| {
                        let val = if evt.value().is_empty() {
                            None
                        } else {
                            evt.value().parse::<u64>().ok()
                        };
                        on_max_change.call(val);
                    },
                    placeholder: "{placeholder_max}",
                    class: "flex-1 border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
                }
            }
        }
    }
}

#[component]
fn DateRangeFilter(
    start_date: Option<String>,
    end_date: Option<String>,
    on_start_change: EventHandler<Option<String>>,
    on_end_change: EventHandler<Option<String>>,
) -> Element {
    let start_date_value = start_date.clone().unwrap_or_default();
    let end_date_value = end_date.clone().unwrap_or_default();

    rsx! {
        div {
            label {
                class: "block text-sm font-medium text-gray-700 mb-1",
                "Date Range"
            }
            div {
                class: "flex space-x-2",
                div {
                    class: "flex-1",
                    input {
                        r#type: "date",
                        value: "{start_date_value}",
                        oninput: move |evt| {
                            let val = if evt.value().is_empty() {
                                None
                            } else {
                                Some(evt.value())
                            };
                            on_start_change.call(val);
                        },
                        class: "w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
                    }
                }
                div {
                    class: "flex-1",
                    input {
                        r#type: "date",
                        value: "{end_date_value}",
                        oninput: move |evt| {
                            let val = if evt.value().is_empty() {
                                None
                            } else {
                                Some(evt.value())
                            };
                            on_end_change.call(val);
                        },
                        class: "w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-sm"
                    }
                }
            }
        }
    }
}

#[component]
pub fn TableOverviewTab(table: IcebergTable) -> Element {
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
pub fn TableSchemaTab(table: IcebergTable) -> Element {
    rsx! {
        div {
            class: "space-y-6",

            // Current Schema
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                        "Current Schema (ID: {table.schema.schema_id})"
                    }
                    div {
                        class: "mb-4",
                        dl {
                            class: "grid grid-cols-1 gap-x-4 gap-y-4 sm:grid-cols-2",
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
                                    "Total Fields"
                                }
                                dd {
                                    class: "mt-1 text-sm text-gray-900",
                                    "{table.schema.fields.len()}"
                                }
                            }
                        }
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

            // Schema Evolution (if multiple schemas exist)
            if table.schemas.len() > 1 {
                div {
                    class: "bg-white shadow rounded-lg",
                    div {
                        class: "px-4 py-5 sm:p-6",
                        h3 {
                            class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                            "Schema Evolution"
                        }
                        p {
                            class: "text-sm text-gray-500 mb-6",
                            "This table has evolved over time. Compare schema versions to understand field additions, modifications, and other changes."
                        }

                        // Schema versions overview
                        div {
                            class: "mb-6",
                            h4 {
                                class: "text-md font-medium text-gray-900 mb-3",
                                "Available Schema Versions"
                            }
                            div {
                                class: "grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3",
                                for schema in &table.schemas {
                                    div {
                                        class: format!(
                                            "border-2 rounded-lg p-4 {}",
                                            if schema.schema_id == table.schema.schema_id {
                                                "border-blue-500 bg-blue-50"
                                            } else {
                                                "border-gray-200 hover:border-gray-300"
                                            }
                                        ),
                                        div {
                                            class: "flex items-center justify-between mb-2",
                                            h5 {
                                                class: "text-sm font-medium text-gray-900",
                                                "Schema {schema.schema_id}"
                                            }
                                            if schema.schema_id == table.schema.schema_id {
                                                span {
                                                    class: "inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-blue-100 text-blue-800",
                                                    "CURRENT"
                                                }
                                            }
                                        }
                                        p {
                                            class: "text-sm text-gray-600",
                                            "{schema.fields.len()} fields"
                                        }
                                    }
                                }
                            }
                        }

                        // Schema comparison
                        div {
                            h4 {
                                class: "text-md font-medium text-gray-900 mb-3",
                                "Schema Comparison"
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
                                                "Field ID"
                                            }
                                            th {
                                                class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                                "Field Name"
                                            }
                                            for schema in &table.schemas {
                                                th {
                                                    class: format!(
                                                        "px-6 py-3 text-left text-xs font-medium uppercase tracking-wider {}",
                                                        if schema.schema_id == table.schema.schema_id {
                                                            "text-blue-600 bg-blue-50"
                                                        } else {
                                                            "text-gray-500"
                                                        }
                                                    ),
                                                    "Schema {schema.schema_id}"
                                                }
                                            }
                                        }
                                    }
                                    tbody {
                                        class: "bg-white divide-y divide-gray-200",
                                        {
                                            // Collect all unique field IDs across all schemas
                                            let mut all_field_ids = std::collections::HashSet::new();
                                            for schema in &table.schemas {
                                                for field in &schema.fields {
                                                    all_field_ids.insert(field.id);
                                                }
                                            }
                                            let mut sorted_field_ids: Vec<_> = all_field_ids.into_iter().collect();
                                            sorted_field_ids.sort();

                                            rsx! {
                                                for field_id in sorted_field_ids {
                                                    {
                                                        // Find field name (use current schema or first available)
                                                        let field_name = table.schemas.iter()
                                                            .flat_map(|s| &s.fields)
                                                            .find(|f| f.id == field_id)
                                                            .map(|f| f.name.clone())
                                                            .unwrap_or_else(|| format!("Field {}", field_id));

                                                        rsx! {
                                                            tr {
                                                                td {
                                                                    class: "px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900",
                                                                    "{field_id}"
                                                                }
                                                                td {
                                                                    class: "px-6 py-4 whitespace-nowrap text-sm text-gray-900",
                                                                    "{field_name}"
                                                                }
                                                                for schema in &table.schemas {
                                                                    td {
                                                                        class: format!(
                                                                            "px-6 py-4 whitespace-nowrap text-sm {}",
                                                                            if schema.schema_id == table.schema.schema_id {
                                                                                "bg-blue-50"
                                                                            } else {
                                                                                ""
                                                                            }
                                                                        ),
                                                                        {
                                                                            if let Some(field) = schema.fields.iter().find(|f| f.id == field_id) {
                                                                                rsx! {
                                                                                    div {
                                                                                        span {
                                                                                            class: "inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-green-100 text-green-800",
                                                                                            "{field.field_type.to_string()}"
                                                                                        }
                                                                                        if field.required {
                                                                                            span {
                                                                                                class: "ml-1 inline-flex px-1 py-0 text-xs font-semibold rounded bg-red-100 text-red-800",
                                                                                                "REQ"
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            } else {
                                                                                rsx! {
                                                                                    span {
                                                                                        class: "text-gray-400 italic",
                                                                                        "—"
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

fn is_filtered(filters: &SnapshotFilters) -> bool {
    filters.operation_types.len() < 3 || // Not all operation types selected
    filters.files_added_min.is_some() ||
    filters.files_added_max.is_some() ||
    filters.records_added_min.is_some() ||
    filters.records_added_max.is_some() ||
    filters.date_start.is_some() ||
    filters.date_end.is_some()
}

fn get_active_filter_count(filters: &SnapshotFilters) -> usize {
    let mut count = 0;
    if filters.operation_types.len() < 3 {
        count += 1;
    }
    if filters.files_added_min.is_some() || filters.files_added_max.is_some() {
        count += 1;
    }
    if filters.records_added_min.is_some() || filters.records_added_max.is_some() {
        count += 1;
    }
    if filters.date_start.is_some() || filters.date_end.is_some() {
        count += 1;
    }
    count
}

fn apply_snapshot_filters(snapshots: &[Snapshot], filters: &SnapshotFilters) -> Vec<Snapshot> {
    snapshots
        .iter()
        .filter(|snapshot| {
            // Filter by operation type
            if !filters.operation_types.is_empty() {
                let operation = snapshot.operation();
                if !filters.operation_types.contains(&operation) {
                    return false;
                }
            }

            // Filter by files added range
            if let (Some(min_files), Some(summary)) = (filters.files_added_min, &snapshot.summary) {
                if let Some(added_files_str) = &summary.added_data_files {
                    if let Ok(added_files) = added_files_str.parse::<u32>() {
                        if added_files < min_files {
                            return false;
                        }
                    }
                }
            }
            if let (Some(max_files), Some(summary)) = (filters.files_added_max, &snapshot.summary) {
                if let Some(added_files_str) = &summary.added_data_files {
                    if let Ok(added_files) = added_files_str.parse::<u32>() {
                        if added_files > max_files {
                            return false;
                        }
                    }
                }
            }

            // Filter by records added range
            if let (Some(min_records), Some(summary)) =
                (filters.records_added_min, &snapshot.summary)
            {
                if let Some(added_records_str) = &summary.added_records {
                    if let Ok(added_records) = added_records_str.parse::<u64>() {
                        if added_records < min_records {
                            return false;
                        }
                    }
                }
            }
            if let (Some(max_records), Some(summary)) =
                (filters.records_added_max, &snapshot.summary)
            {
                if let Some(added_records_str) = &summary.added_records {
                    if let Ok(added_records) = added_records_str.parse::<u64>() {
                        if added_records > max_records {
                            return false;
                        }
                    }
                }
            }

            // Filter by date range
            if let Some(start_date) = &filters.date_start {
                if let Ok(start_timestamp) =
                    chrono::NaiveDate::parse_from_str(start_date, "%Y-%m-%d")
                {
                    let start_datetime = start_timestamp
                        .and_hms_opt(0, 0, 0)
                        .unwrap()
                        .and_utc()
                        .timestamp_millis();
                    if snapshot.timestamp_ms < start_datetime {
                        return false;
                    }
                }
            }
            if let Some(end_date) = &filters.date_end {
                if let Ok(end_timestamp) = chrono::NaiveDate::parse_from_str(end_date, "%Y-%m-%d") {
                    let end_datetime = end_timestamp
                        .and_hms_opt(23, 59, 59)
                        .unwrap()
                        .and_utc()
                        .timestamp_millis();
                    if snapshot.timestamp_ms > end_datetime {
                        return false;
                    }
                }
            }

            true
        })
        .cloned()
        .collect()
}

#[component]
pub fn SnapshotTimelineTab(table: IcebergTable) -> Element {
    let mut filters = use_signal(SnapshotFilters::default);
    let mut show_filters = use_signal(|| false);

    // Existing snapshot processing logic...
    let mut sorted_snapshots = table.snapshots.clone();
    sorted_snapshots.sort_by(|a, b| b.timestamp_ms.cmp(&a.timestamp_ms));

    // Apply filters to snapshots
    let filtered_snapshots = apply_snapshot_filters(&sorted_snapshots, &filters());

    // Compute health metrics - Analytics engine is active!
    let health_metrics = TableAnalytics::compute_health_metrics(&table);

    // Log health metrics to console for demo (in production this would be displayed in UI)
    tracing::info!(
        "Table Health Analytics: Score={:.1}, Files={} ({:.1}% small), Activity={}/hr, Storage={:.1}GB ({:+.1}GB/day), Alerts={}",
        health_metrics.health_score,
        health_metrics.file_health.total_files,
        health_metrics.file_health.small_file_ratio * 100.0,
        health_metrics
            .operational_health
            .snapshot_frequency
            .snapshots_last_hour,
        health_metrics.storage_efficiency.total_size_gb,
        health_metrics
            .storage_efficiency
            .storage_growth_rate_gb_per_day,
        health_metrics.alerts.len()
    );

    rsx! {
        div {
            class: "space-y-6",

            // Health Analytics Notification (demonstrates analytics are working)
            div {
                class: "bg-green-50 border border-green-200 rounded-lg p-3 mb-4",
                div {
                    class: "flex items-center",
                    span { class: "text-green-700 text-sm font-medium", "✅ Health Analytics Active" }
                    span { class: "ml-2 text-green-600 text-sm", "Score: {health_metrics.health_score:.1}/100 | Files: {health_metrics.file_health.total_files} | Alerts: {health_metrics.alerts.len()}" }
                }
            }

            // Filter Panel Header with Toggle
            div {
                class: "flex items-center justify-between",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900",
                    "Snapshot History"
                }
                button {
                    onclick: move |_| show_filters.set(!show_filters()),
                    class: format!("flex items-center px-3 py-2 text-sm font-medium border rounded-md hover:bg-gray-50 relative {}",
                        if is_filtered(&filters()) {
                            "text-blue-700 border-blue-300 bg-blue-50"
                        } else {
                            "text-gray-600 border-gray-300 hover:text-gray-900"
                        }
                    ),
                    svg {
                        class: format!("h-4 w-4 mr-2 transform transition-transform {}",
                            if show_filters() { "rotate-180" } else { "" }
                        ),
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"
                        }
                    }
                    "Filters"

                    // Active filter count badge
                    if is_filtered(&filters()) {
                        span {
                            class: "absolute -top-2 -right-2 inline-flex items-center justify-center px-2 py-1 text-xs font-bold leading-none text-white bg-red-600 rounded-full",
                            "{get_active_filter_count(&filters())}"
                        }
                    }
                }
            }

            // Collapsible Filter Panel
            if show_filters() {
                div {
                    class: "bg-gray-50 border border-gray-200 rounded-lg p-4",

                    // Quick Filters and Clear All
                    div {
                        class: "flex flex-wrap items-center gap-2 mb-4 pb-4 border-b border-gray-200",
                        span {
                            class: "text-sm font-medium text-gray-700 mr-2",
                            "Quick filters:"
                        }

                        // Last 7 days
                        button {
                            onclick: move |_| {
                                let now = chrono::Utc::now();
                                let seven_days_ago = now - chrono::Duration::days(7);
                                filters.with_mut(|f| {
                                    f.date_start = Some(seven_days_ago.format("%Y-%m-%d").to_string());
                                    f.date_end = Some(now.format("%Y-%m-%d").to_string());
                                });
                            },
                            class: "px-3 py-1 text-xs font-medium text-blue-700 bg-blue-100 border border-blue-300 rounded-md hover:bg-blue-200",
                            "Last 7 days"
                        }

                        // Last 30 days
                        button {
                            onclick: move |_| {
                                let now = chrono::Utc::now();
                                let thirty_days_ago = now - chrono::Duration::days(30);
                                filters.with_mut(|f| {
                                    f.date_start = Some(thirty_days_ago.format("%Y-%m-%d").to_string());
                                    f.date_end = Some(now.format("%Y-%m-%d").to_string());
                                });
                            },
                            class: "px-3 py-1 text-xs font-medium text-blue-700 bg-blue-100 border border-blue-300 rounded-md hover:bg-blue-200",
                            "Last 30 days"
                        }

                        // All time
                        button {
                            onclick: move |_| {
                                filters.with_mut(|f| {
                                    f.date_start = None;
                                    f.date_end = None;
                                });
                            },
                            class: "px-3 py-1 text-xs font-medium text-green-700 bg-green-100 border border-green-300 rounded-md hover:bg-green-200",
                            "All time"
                        }

                        div { class: "flex-1" } // Spacer

                        // Clear all button
                        button {
                            onclick: move |_| {
                                filters.set(SnapshotFilters::default());
                            },
                            disabled: !is_filtered(&filters()),
                            class: format!("px-3 py-1 text-xs font-medium border rounded-md {}",
                                if is_filtered(&filters()) {
                                    "text-red-700 bg-red-100 border-red-300 hover:bg-red-200"
                                } else {
                                    "text-gray-400 bg-gray-100 border-gray-300 cursor-not-allowed"
                                }
                            ),
                            "Clear all"
                        }
                    }

                    div {
                        class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4",

                        // Operation Type Filter
                        OperationTypeFilter {
                            selected_types: filters().operation_types,
                            on_change: move |types| {
                                filters.with_mut(|f| f.operation_types = types);
                            }
                        }

                        // Files Added Range
                        NumberRangeFilter {
                            label: "Files Added".to_string(),
                            min_value: filters().files_added_min.map(|v| v as u64),
                            max_value: filters().files_added_max.map(|v| v as u64),
                            placeholder_min: "Min files".to_string(),
                            placeholder_max: "Max files".to_string(),
                            on_min_change: move |val: Option<u64>| {
                                filters.with_mut(|f| f.files_added_min = val.map(|v| v as u32));
                            },
                            on_max_change: move |val: Option<u64>| {
                                filters.with_mut(|f| f.files_added_max = val.map(|v| v as u32));
                            }
                        }

                        // Records Added Range
                        NumberRangeFilter {
                            label: "Records Added".to_string(),
                            min_value: filters().records_added_min,
                            max_value: filters().records_added_max,
                            placeholder_min: "Min records".to_string(),
                            placeholder_max: "Max records".to_string(),
                            on_min_change: move |val| {
                                filters.with_mut(|f| f.records_added_min = val);
                            },
                            on_max_change: move |val| {
                                filters.with_mut(|f| f.records_added_max = val);
                            }
                        }

                        // Date Range Filter
                        DateRangeFilter {
                            start_date: filters().date_start,
                            end_date: filters().date_end,
                            on_start_change: move |val| {
                                filters.with_mut(|f| f.date_start = val);
                            },
                            on_end_change: move |val| {
                                filters.with_mut(|f| f.date_end = val);
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
                        "Snapshot Summary"
                    }
                    dl {
                        class: "grid grid-cols-1 gap-x-4 gap-y-6 sm:grid-cols-3",
                        div {
                            class: "text-center",
                            dt {
                                class: "text-sm font-medium text-gray-500",
                                if filtered_snapshots.len() != table.snapshots.len() {
                                    "Filtered Snapshots"
                                } else {
                                    "Total Snapshots"
                                }
                            }
                            dd {
                                class: "mt-1 text-2xl font-semibold text-gray-900",
                                if filtered_snapshots.len() != table.snapshots.len() {
                                    "{filtered_snapshots.len()} of {table.snapshots.len()}"
                                } else {
                                    "{filtered_snapshots.len()}"
                                }
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
                                    for snapshot in &filtered_snapshots {
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
                                        filtered_snapshots.iter().min_by_key(|s| s.timestamp_ms),
                                        filtered_snapshots.iter().max_by_key(|s| s.timestamp_ms),
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

            // Detailed Timeline
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900 mb-2",
                        "Snapshot Timeline"
                    }
                    p {
                        class: "text-sm text-gray-500 mb-6",
                        "Detailed history showing all table snapshots from most recent to oldest"
                    }
                    if filtered_snapshots.is_empty() {
                        // No results state
                        div {
                            class: "text-center py-12",
                            svg {
                                class: "mx-auto h-12 w-12 text-gray-400 mb-4",
                                fill: "none",
                                view_box: "0 0 24 24",
                                stroke: "currentColor",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M9 5H7a2 2 0 00-2 2v10a2 2 0 002 2h8a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4"
                                }
                            }
                            h3 {
                                class: "text-lg font-medium text-gray-900 mb-2",
                                "No snapshots found"
                            }
                            p {
                                class: "text-sm text-gray-500",
                                if is_filtered(&filters()) {
                                    "No snapshots match your current filter criteria. Try adjusting your filters or use the \"Clear all\" button to see all snapshots."
                                } else {
                                    "This table has no snapshots to display."
                                }
                            }
                        }
                    } else {
                        div {
                            class: "flow-root",
                            ul {
                                role: "list",
                                class: "relative",
                                for (index, snapshot) in filtered_snapshots.iter().enumerate() {
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
            }
        }
    }
}

#[component]
pub fn TablePartitionsTab(table: IcebergTable) -> Element {
    rsx! {
        div {
            class: "space-y-6",

            // Current Partition Spec
            if let Some(partition_spec) = &table.partition_spec {
                div {
                    class: "bg-white shadow rounded-lg",
                    div {
                        class: "px-4 py-5 sm:p-6",
                        h3 {
                            class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                            "Current Partition Specification (ID: {partition_spec.spec_id})"
                        }
                        div {
                            class: "mb-4",
                            dl {
                                class: "grid grid-cols-1 gap-x-4 gap-y-4 sm:grid-cols-2",
                                div {
                                    dt {
                                        class: "text-sm font-medium text-gray-500",
                                        "Spec ID"
                                    }
                                    dd {
                                        class: "mt-1 text-sm text-gray-900",
                                        "{partition_spec.spec_id}"
                                    }
                                }
                                div {
                                    dt {
                                        class: "text-sm font-medium text-gray-500",
                                        "Partition Fields"
                                    }
                                    dd {
                                        class: "mt-1 text-sm text-gray-900",
                                        "{partition_spec.fields.len()}"
                                    }
                                }
                            }
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
                                            "Field ID"
                                        }
                                        th {
                                            class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            "Source Field"
                                        }
                                        th {
                                            class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            "Name"
                                        }
                                        th {
                                            class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider",
                                            "Transform"
                                        }
                                    }
                                }
                                tbody {
                                    class: "bg-white divide-y divide-gray-200",
                                    for field in &partition_spec.fields {
                                        PartitionFieldRow { field: field.clone(), table: table.clone() }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div {
                    class: "bg-white shadow rounded-lg",
                    div {
                        class: "px-4 py-5 sm:p-6 text-center",
                        h3 {
                            class: "text-lg font-medium text-gray-900 mb-2",
                            "No Partitioning"
                        }
                        p {
                            class: "text-sm text-gray-500",
                            "This table is not partitioned. All data is stored without partitioning strategy."
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn PartitionFieldRow(field: PartitionField, table: IcebergTable) -> Element {
    // Find the source field name from the schema
    let source_field_name = table
        .schema
        .fields
        .iter()
        .find(|f| f.id == field.source_id)
        .map(|f| f.name.clone())
        .unwrap_or_else(|| format!("Field {}", field.source_id));

    rsx! {
        tr {
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900",
                "{field.field_id}"
            }
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm text-gray-900",
                "{source_field_name}"
            }
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm text-gray-900",
                "{field.name}"
            }
            td {
                class: "px-6 py-4 whitespace-nowrap text-sm text-gray-500",
                span {
                    class: "inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-purple-100 text-purple-800",
                    {field.transform.to_string()}
                }
            }
        }
    }
}

// TableHealthDashboard will be implemented in future iterations

#[component]
pub fn HealthScore(score: f64) -> Element {
    let (color_class, text_class, bg_class) = match score {
        s if s >= 90.0 => ("text-green-700", "text-green-800", "bg-green-100"),
        s if s >= 75.0 => ("text-blue-700", "text-blue-800", "bg-blue-100"),
        s if s >= 60.0 => ("text-yellow-700", "text-yellow-800", "bg-yellow-100"),
        s if s >= 40.0 => ("text-orange-700", "text-orange-800", "bg-orange-100"),
        _ => ("text-red-700", "text-red-800", "bg-red-100"),
    };

    let label = match score {
        s if s >= 90.0 => "Excellent",
        s if s >= 75.0 => "Good",
        s if s >= 60.0 => "Fair",
        s if s >= 40.0 => "Poor",
        _ => "Critical",
    };

    rsx! {
        div {
            class: format!("inline-flex items-center px-3 py-1 rounded-full text-sm font-medium {bg_class}"),
            span {
                class: format!("w-2 h-2 rounded-full mr-2 {}", color_class.replace("text-", "bg-")),
            }
            span {
                class: text_class,
                "{score:.1} / 100 ({label})"
            }
        }
    }
}

// Health Alert Component
#[component]
pub fn HealthAlert(alert: crate::data::HealthAlert) -> Element {
    let (icon, bg_class, border_class, text_class) = match alert.severity {
        AlertSeverity::Critical | AlertSeverity::Emergency => (
            "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z",
            "bg-red-50",
            "border-red-200",
            "text-red-800",
        ),
        AlertSeverity::Warning => (
            "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.732-.833-2.464 0L4.34 16.5c-.77.833.192 2.5 1.732 2.5z",
            "bg-yellow-50",
            "border-yellow-200",
            "text-yellow-800",
        ),
        AlertSeverity::Info => (
            "M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z",
            "bg-blue-50",
            "border-blue-200",
            "text-blue-800",
        ),
    };

    rsx! {
        div {
            class: format!("flex items-start p-3 rounded-lg border {bg_class} {border_class}"),
            svg {
                class: format!("h-5 w-5 mt-0.5 mr-3 {text_class}"),
                fill: "none",
                stroke: "currentColor",
                view_box: "0 0 24 24",
                path {
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    stroke_width: "2",
                    d: icon
                }
            }
            div {
                class: "flex-1",
                p {
                    class: format!("text-sm font-medium {text_class}"),
                    "{alert.message}"
                }
                if alert.metric_value != 0.0 {
                    p {
                        class: format!("text-xs mt-1 {}", text_class.replace("800", "600")),
                        "Value: {alert.metric_value:.1} (threshold: {alert.threshold:.1})"
                    }
                }
            }
        }
    }
}

// Simple Health Components Working Version
