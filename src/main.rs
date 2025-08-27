#![allow(non_snake_case)]

use dioxus::prelude::*;

mod catalog;
mod catalog_ui;
mod components;
mod config;
mod data;
mod iceberg_adapter;

use catalog::CatalogManager;
use catalog_ui::{CatalogBrowser, CatalogConnectionScreen};
use components::{SnapshotTimelineTab, TableOverviewTab, TablePartitionsTab, TableSchemaTab};
use data::IcebergTable;

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    CatalogConnection,
    Connected, // After connecting, show the tabbed interface
}

#[derive(Debug, Clone, PartialEq)]
enum AppTab {
    Catalog,
    Table { table: IcebergTable, tab_id: String },
}

#[derive(Debug, Clone, PartialEq)]
enum TableViewTab {
    Overview,
    Schema,
    Partitions,
    SnapshotHistory,
}

fn main() {
    dioxus_logger::init(log::LevelFilter::Info).expect("failed to init logger");

    LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new().with_window(
            dioxus::desktop::WindowBuilder::new().with_title("Hielo - Apache Iceberg Table Viewer"),
        ))
        .launch(App);
}

fn App() -> Element {
    let mut app_state = use_signal(|| AppState::CatalogConnection);
    let mut open_tabs = use_signal(|| vec![AppTab::Catalog]);
    let mut active_tab_index = use_signal(|| 0usize);
    let mut table_view_tab = use_signal(|| TableViewTab::Overview);
    let catalog_manager = use_signal(CatalogManager::new);
    let mut loading_table = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);
    let mut show_global_search = use_signal(|| false);
    let mut global_search_query = use_signal(|| String::new());

    let load_table = move |(catalog_name, namespace, table_name): (String, String, String)| {
        spawn(async move {
            loading_table.set(true);
            error_message.set(None);

            match catalog_manager
                .read()
                .load_table(&catalog_name, &namespace, &table_name)
                .await
            {
                Ok(iceberg_table) => {
                    match iceberg_adapter::convert_iceberg_table(
                        &iceberg_table,
                        namespace.clone(),
                        catalog_name.clone(),
                    ) {
                        Ok(hielo_table) => {
                            // Create a unique tab ID
                            let tab_id = format!("{}.{}", namespace, table_name);
                            let new_tab = AppTab::Table {
                                table: hielo_table,
                                tab_id: tab_id.clone(),
                            };

                            // Check if tab already exists
                            let existing_index = open_tabs.read().iter().position(|tab| {
                                if let AppTab::Table {
                                    tab_id: existing_id,
                                    ..
                                } = tab
                                {
                                    existing_id == &tab_id
                                } else {
                                    false
                                }
                            });

                            if let Some(index) = existing_index {
                                // Switch to existing tab
                                active_tab_index.set(index);
                            } else {
                                // Add new tab and switch to it
                                let mut tabs = open_tabs.read().clone();
                                tabs.push(new_tab);
                                let new_index = tabs.len() - 1;
                                open_tabs.set(tabs);
                                active_tab_index.set(new_index);
                            }

                            // Ensure we're in connected state
                            app_state.set(AppState::Connected);
                        }
                        Err(e) => {
                            error_message.set(Some(format!("Failed to convert table: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to load table: {}", e)));
                }
            }
            loading_table.set(false);
        });
    };

    let on_catalog_connected = move |_| {
        app_state.set(AppState::Connected);
        active_tab_index.set(0); // Switch to catalog tab
    };

    rsx! {
        div {
            class: "min-h-screen bg-gray-100",
            tabindex: "0", // Make div focusable for keyboard events
            onkeydown: move |event| {
                // Handle CTRL+F to open global search (only when connected)
                let key_str = format!("{:?}", event.key());
                if event.modifiers().ctrl() && key_str.contains("\"f\"") {
                    if matches!(app_state(), AppState::Connected) {
                        show_global_search.set(true);
                        global_search_query.set(String::new());
                    }
                }
            },

            // Loading overlay
            if loading_table() {
                div {
                    class: "fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center",
                    div {
                        class: "bg-white p-8 rounded-lg shadow-lg",
                        div {
                            class: "flex items-center space-x-4",
                            div {
                                class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"
                            }
                            div {
                                class: "text-lg font-medium text-gray-900",
                                "Loading table..."
                            }
                        }
                    }
                }
            }

            // Error message
            if let Some(error) = error_message() {
                div {
                    class: "fixed top-4 right-4 z-40 bg-red-50 border border-red-200 rounded-md p-4 max-w-md",
                    div {
                        class: "flex",
                        div {
                            class: "flex-shrink-0",
                            svg {
                                class: "h-5 w-5 text-red-400",
                                fill: "currentColor",
                                view_box: "0 0 20 20",
                                path {
                                    fill_rule: "evenodd",
                                    d: "M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z",
                                    clip_rule: "evenodd"
                                }
                            }
                        }
                        div {
                            class: "ml-3",
                            p {
                                class: "text-sm font-medium text-red-800",
                                "Error"
                            }
                            p {
                                class: "text-sm text-red-700",
                                "{error}"
                            }
                        }
                        div {
                            class: "ml-auto pl-3",
                            button {
                                onclick: move |_| error_message.set(None),
                                class: "inline-flex text-red-400 hover:text-red-600",
                                svg {
                                    class: "h-5 w-5",
                                    fill: "currentColor",
                                    view_box: "0 0 20 20",
                                    path {
                                        fill_rule: "evenodd",
                                        d: "M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z",
                                        clip_rule: "evenodd"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Global search modal (CTRL+F)
            if show_global_search() {
                GlobalSearchModal {
                    catalog_manager: catalog_manager,
                    search_query: global_search_query(),
                    on_search_change: move |query: String| global_search_query.set(query),
                    on_table_selected: load_table,
                    on_close: move |_| {
                        show_global_search.set(false);
                        global_search_query.set(String::new());
                    }
                }
            }

            // Main content based on app state
            match app_state() {
                AppState::CatalogConnection => rsx! {
                    CatalogConnectionScreen {
                        catalog_manager: catalog_manager,
                        on_catalog_connected: on_catalog_connected,
                        on_table_selected: load_table
                    }
                },
                AppState::Connected => rsx! {
                    // Header
                    header {
                        class: "bg-white shadow-sm border-b",
                        div {
                            class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                            div {
                                class: "flex justify-between items-center py-6",
                                div {
                                    class: "flex items-center space-x-4",
                                    h1 {
                                        class: "text-3xl font-bold text-gray-900",
                                        "🧊 Hielo"
                                    }
                                }

                                // Home button
                                button {
                                    onclick: move |_| {
                                        app_state.set(AppState::CatalogConnection);
                                        open_tabs.set(vec![AppTab::Catalog]);
                                        active_tab_index.set(0);
                                    },
                                    class: "flex items-center px-3 py-2 border border-gray-300 rounded-md text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 hover:text-gray-900 transition-colors",
                                    svg {
                                        class: "h-4 w-4 mr-2",
                                        fill: "none",
                                        stroke: "currentColor",
                                        view_box: "0 0 24 24",
                                        path {
                                            stroke_linecap: "round",
                                            stroke_linejoin: "round",
                                            stroke_width: "2",
                                            d: "M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"
                                        }
                                    }
                                    "Home"
                                }
                            }
                        }
                    }

                    // Tab Navigation
                    div {
                        class: "bg-white border-b",
                        div {
                            class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                            nav {
                                class: "flex space-x-0 overflow-x-auto",
                                for (index, tab) in open_tabs().iter().enumerate() {
                                    div {
                                        class: "flex items-center",
                                        button {
                                            onclick: move |_| {
                                                if index < open_tabs.read().len() {
                                                    active_tab_index.set(index);
                                                }
                                            },
                                            class: format!(
                                                "py-4 px-4 border-b-2 font-medium text-sm transition-colors whitespace-nowrap flex items-center {}",
                                                if active_tab_index() == index {
                                                    "border-blue-500 text-blue-600 bg-blue-50"
                                                } else {
                                                    "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                                }
                                            ),
                                            match tab {
                                                AppTab::Catalog => "🔍 Catalog".to_string(),
                                                AppTab::Table { table, .. } => format!("📊 {}.{}", table.namespace, table.name),
                                            }
                                        }
                                        if index > 0 {
                                            button {
                                                onclick: move |_| {
                                                    let mut tabs = open_tabs.read().clone();
                                                    if tabs.len() > 1 && index > 0 { // Don't close catalog tab
                                                        tabs.remove(index);
                                                        open_tabs.set(tabs.clone());

                                                        // Adjust active tab index if necessary
                                                        let current_active = *active_tab_index.read();
                                                        if current_active >= index {
                                                            let new_active = if current_active > 0 { current_active - 1 } else { 0 };
                                                            active_tab_index.set(new_active);
                                                        }
                                                    }
                                                },
                                                class: "ml-2 p-1 rounded-full hover:bg-gray-200 text-gray-400 hover:text-gray-600",
                                                "×"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Tab Content
                    main {
                        class: "max-w-7xl mx-auto py-6 px-4 sm:px-6 lg:px-8",
                        if let Some(current_tab) = open_tabs().get(active_tab_index()) {
                            match current_tab {
                                AppTab::Catalog => rsx! {
                                    CatalogBrowser {
                                        catalog_manager: catalog_manager,
                                        on_table_selected: load_table,
                                        on_home_requested: move |_| {
                                            app_state.set(AppState::CatalogConnection);
                                            open_tabs.set(vec![AppTab::Catalog]);
                                            active_tab_index.set(0);
                                        }
                                    }
                                },
                                AppTab::Table { table, .. } => rsx! {
                                    // Table sub-tabs
                                    div {
                                        class: "mb-6",
                                        nav {
                                            class: "flex justify-between items-center border-b border-gray-200",
                                            div {
                                                class: "flex space-x-8",
                                            button {
                                                class: format!(
                                                    "py-2 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                                    if *table_view_tab.read() == TableViewTab::Overview {
                                                        "border-blue-500 text-blue-600"
                                                    } else {
                                                        "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                                    }
                                                ),
                                                onclick: move |_| table_view_tab.set(TableViewTab::Overview),
                                                "📊 Overview"
                                            }
                                            button {
                                                class: format!(
                                                    "py-2 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                                    if *table_view_tab.read() == TableViewTab::Schema {
                                                        "border-blue-500 text-blue-600"
                                                    } else {
                                                        "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                                    }
                                                ),
                                                onclick: move |_| table_view_tab.set(TableViewTab::Schema),
                                                "🏗️ Schema"
                                            }
                                            if table.partition_spec.is_some() {
                                                button {
                                                    class: format!(
                                                        "py-2 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                                        if *table_view_tab.read() == TableViewTab::Partitions {
                                                            "border-blue-500 text-blue-600"
                                                        } else {
                                                            "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                                        }
                                                    ),
                                                    onclick: move |_| table_view_tab.set(TableViewTab::Partitions),
                                                    "🧩 Partitions"
                                                }
                                            }
                                            button {
                                                class: format!(
                                                    "py-2 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                                    if *table_view_tab.read() == TableViewTab::SnapshotHistory {
                                                        "border-blue-500 text-blue-600"
                                                    } else {
                                                        "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                                    }
                                                ),
                                                onclick: move |_| table_view_tab.set(TableViewTab::SnapshotHistory),
                                                "📈 Snapshot History"
                                            }
                                            }

                                            // Refresh button
                                            button {
                                                class: "flex items-center px-3 py-2 border border-gray-300 rounded-md text-sm font-medium text-gray-700 bg-white hover:bg-gray-50 hover:text-gray-900 transition-colors",
                                                onclick: {
                                                    let table_name = table.name.clone();
                                                    let namespace = table.namespace.clone();
                                                    let catalog_name = table.catalog_name.clone();
                                                    move |_| {
                                                        load_table((catalog_name.clone(), namespace.clone(), table_name.clone()));
                                                    }
                                                },
                                                disabled: loading_table(),
                                                if loading_table() {
                                                    svg {
                                                        class: "animate-spin -ml-1 mr-2 h-4 w-4",
                                                        fill: "none",
                                                        view_box: "0 0 24 24",
                                                        circle {
                                                            class: "opacity-25",
                                                            cx: "12",
                                                            cy: "12",
                                                            r: "10",
                                                            stroke: "currentColor",
                                                            stroke_width: "4"
                                                        }
                                                        path {
                                                            class: "opacity-75",
                                                            fill: "currentColor",
                                                            d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                                                        }
                                                    }
                                                } else {
                                                    svg {
                                                        class: "h-4 w-4 mr-2",
                                                        fill: "none",
                                                        stroke: "currentColor",
                                                        view_box: "0 0 24 24",
                                                        path {
                                                            stroke_linecap: "round",
                                                            stroke_linejoin: "round",
                                                            stroke_width: "2",
                                                            d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"
                                                        }
                                                    }
                                                }
                                                "Refresh"
                                            }
                                        }
                                    }

                                    // Table content
                                    match *table_view_tab.read() {
                                        TableViewTab::Overview => rsx! {
                                            TableOverviewTab { table: table.clone() }
                                        },
                                        TableViewTab::Schema => rsx! {
                                            TableSchemaTab { table: table.clone() }
                                        },
                                        TableViewTab::Partitions => rsx! {
                                            TablePartitionsTab { table: table.clone() }
                                        },
                                        TableViewTab::SnapshotHistory => rsx! {
                                            SnapshotTimelineTab { table: table.clone() }
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Include Tailwind CSS
        style {
            "
            @import url('https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css');
            
            .timeline-item {{
                position: relative;
                padding-left: 2rem;
                margin-bottom: 2rem;
            }}
            
            .timeline-item::before {{
                content: '';
                position: absolute;
                left: 0.5rem;
                top: 0.5rem;
                width: 0.75rem;
                height: 0.75rem;
                background-color: #3b82f6;
                border-radius: 50%;
                border: 2px solid white;
                box-shadow: 0 0 0 2px #3b82f6;
            }}
            
            .timeline-item::after {{
                content: '';
                position: absolute;
                left: 0.875rem;
                top: 1.25rem;
                width: 2px;
                height: calc(100% + 1rem);
                background-color: #e5e7eb;
            }}
            
            .timeline-item:last-child::after {{
                display: none;
            }}
            "
        }
    }
}

#[component]
fn GlobalSearchModal(
    catalog_manager: Signal<CatalogManager>,
    search_query: String,
    on_search_change: EventHandler<String>,
    on_table_selected: EventHandler<(String, String, String)>,
    on_close: EventHandler<()>,
) -> Element {
    let mut all_tables = use_signal(|| Vec::<catalog::TableReference>::new());
    let mut loading = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);

    // Load all tables from all namespaces when modal opens
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            error_message.set(None);

            let connections = catalog_manager.read().get_connections().to_vec();
            if let Some(connection) = connections.first() {
                let catalog_name = connection.config.name.clone();

                match catalog_manager.read().list_namespaces(&catalog_name).await {
                    Ok(namespaces) => {
                        let mut tables = Vec::new();

                        for namespace in namespaces {
                            match catalog_manager
                                .read()
                                .list_tables(&catalog_name, &namespace)
                                .await
                            {
                                Ok(namespace_tables) => {
                                    tables.extend(namespace_tables);
                                }
                                Err(e) => {
                                    error_message.set(Some(format!(
                                        "Failed to load tables from namespace '{}': {}",
                                        namespace, e
                                    )));
                                }
                            }
                        }

                        all_tables.set(tables);
                    }
                    Err(e) => {
                        error_message.set(Some(format!("Failed to load namespaces: {}", e)));
                    }
                }
            } else {
                error_message.set(Some("No catalog connection found".to_string()));
            }

            loading.set(false);
        });
    });

    // Filter tables based on search query
    let query_clone = search_query.clone();
    let filtered_tables: Vec<catalog::TableReference> = if query_clone.is_empty() {
        all_tables()
    } else {
        let query_lower = query_clone.to_lowercase();
        all_tables()
            .into_iter()
            .filter(|table| {
                table.full_name.to_lowercase().contains(&query_lower)
                    || table.name.to_lowercase().contains(&query_lower)
                    || table.namespace.to_lowercase().contains(&query_lower)
            })
            .collect()
    };

    rsx! {
        // Modal overlay
        div {
            class: "fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-start justify-center pt-20",
            onclick: move |_| on_close.call(()),

            // Modal content
            div {
                class: "bg-white rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-96 flex flex-col",
                onclick: |e| e.stop_propagation(), // Prevent closing when clicking inside modal

                // Header
                div {
                    class: "flex items-center justify-between p-4 border-b border-gray-200",
                    h3 {
                        class: "text-lg font-medium text-gray-900",
                        "🔍 Find Table (Ctrl+F)"
                    }
                    button {
                        onclick: move |_| on_close.call(()),
                        class: "text-gray-400 hover:text-gray-600",
                        "✕"
                    }
                }

                // Search input
                div {
                    class: "p-4 border-b border-gray-200",
                    input {
                        r#type: "text",
                        placeholder: "Search by table name or namespace.table_name...",
                        value: search_query,
                        oninput: move |evt| on_search_change.call(evt.value()),
                        onkeydown: move |event| {
                            let key_str = format!("{:?}", event.key());
                            if key_str.contains("Escape") {
                                on_close.call(());
                            }
                        },
                        class: "w-full px-3 py-2 border border-gray-300 rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                        autofocus: true
                    }
                }

                // Results
                div {
                    class: "flex-1 overflow-y-auto",
                    if loading() {
                        div {
                            class: "flex items-center justify-center py-8",
                            div {
                                class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"
                            }
                        }
                    } else if let Some(error) = error_message() {
                        div {
                            class: "p-4 text-red-600 text-sm",
                            "Error: {error}"
                        }
                    } else if filtered_tables.is_empty() {
                        div {
                            class: "p-4 text-gray-500 text-sm text-center",
                            if query_clone.is_empty() {
                                "No tables found"
                            } else {
                                "No tables match your search"
                            }
                        }
                    } else {
                        div {
                            class: "divide-y divide-gray-200",
                            for table in filtered_tables.iter().take(10) { // Limit to first 10 results
                                button {
                                    onclick: {
                                        let table_clone = table.clone();
                                        let connections = catalog_manager.read().get_connections().to_vec();
                                        let catalog_name = if let Some(connection) = connections.first() {
                                            connection.config.name.clone()
                                        } else {
                                            "unknown".to_string()
                                        };
                                        move |_| {
                                            // Only allow selection of Iceberg tables
                                            if table_clone.table_type == catalog::TableType::Iceberg {
                                                on_table_selected.call((catalog_name.clone(), table_clone.namespace.clone(), table_clone.name.clone()));
                                                on_close.call(());
                                            }
                                        }
                                    },
                                    class: format!(
                                        "w-full px-4 py-3 text-left hover:bg-gray-50 flex items-center justify-between {}",
                                        if table.table_type == catalog::TableType::Iceberg {
                                            "cursor-pointer"
                                        } else {
                                            "cursor-not-allowed opacity-50"
                                        }
                                    ),
                                    disabled: table.table_type != catalog::TableType::Iceberg,

                                    div {
                                        class: "flex items-center",
                                        span {
                                            class: "mr-3 text-lg",
                                            if table.table_type == catalog::TableType::Iceberg {
                                                "🧊"
                                            } else {
                                                "📄"
                                            }
                                        }
                                        div {
                                            div {
                                                class: "font-medium text-gray-900 text-sm",
                                                "{table.full_name}"
                                            }
                                            div {
                                                class: "text-gray-500 text-xs",
                                                "{table.namespace} • {table.name}"
                                            }
                                        }
                                    }

                                    if table.table_type == catalog::TableType::Iceberg {
                                        span {
                                            class: "text-gray-400 text-xs",
                                            "Press Enter"
                                        }
                                    } else {
                                        span {
                                            class: "text-gray-400 text-xs",
                                            "Not Iceberg"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Footer
                div {
                    class: "p-3 bg-gray-50 border-t border-gray-200 text-xs text-gray-500",
                    if !filtered_tables.is_empty() {
                        "Showing {filtered_tables.len().min(10)} of {filtered_tables.len()} tables"
                    } else {
                        "Use Ctrl+F to open this search anytime"
                    }
                }
            }
        }
    }
}
