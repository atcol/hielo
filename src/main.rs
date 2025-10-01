#![allow(non_snake_case)]

use dioxus::events::Key;
use dioxus::prelude::*;

mod analytics;
mod catalog;
mod catalog_ui;
mod components;
mod config;
mod data;
mod iceberg_adapter;

use catalog::CatalogManager;
use catalog_ui::CatalogConnectionScreen;
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
    dioxus_logger::init(tracing::Level::INFO).expect("failed to init logger");

    LaunchBuilder::desktop()
        .with_cfg(dioxus::desktop::Config::new().with_window(
            dioxus::desktop::WindowBuilder::new().with_title("Hielo - Apache Iceberg Table Viewer"),
        ))
        .launch(App);
}

fn App() -> Element {
    let mut app_state = use_signal(|| {
        // Start in Connected state if there are saved catalogs, otherwise CatalogConnection
        let catalog_manager = CatalogManager::new();
        if catalog_manager.get_saved_catalogs().is_empty() {
            AppState::CatalogConnection
        } else {
            AppState::Connected
        }
    });
    let mut open_tabs = use_signal(|| vec![AppTab::Catalog]);
    let mut active_tab_index = use_signal(|| 0usize);
    let mut table_view_tab = use_signal(|| TableViewTab::Overview);
    let mut catalog_manager = use_signal(CatalogManager::new);
    let mut loading_table = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);
    let mut show_global_search = use_signal(|| false);
    let mut global_search_query = use_signal(String::new);
    let mut nav_pane_collapsed = use_signal(|| false);
    let mut show_delete_confirmation = use_signal(|| false);
    let mut delete_catalog_name = use_signal(String::new);
    let expanded_catalogs = use_signal(std::collections::HashSet::<String>::new);
    let expanded_namespaces = use_signal(std::collections::HashSet::<String>::new);

    let mut close_tab = {
        let mut open_tabs = open_tabs.clone();
        let mut active_tab_index = active_tab_index.clone();
        move |index: usize| {
            let mut tabs = open_tabs.write();
            if index < tabs.len() && index != 0 {
                // Don't close the catalog tab (index 0)
                tabs.remove(index);

                // Adjust active tab index if necessary
                let current_active = active_tab_index();
                if current_active >= tabs.len() {
                    active_tab_index.set(tabs.len() - 1);
                } else if current_active > index {
                    active_tab_index.set(current_active - 1);
                }
            }
        }
    };

    let load_table = move |(catalog_name, namespace, table_name): (String, String, String)| {
        log::info!(
            "Loading table: {} from namespace: {} in catalog: {}",
            table_name,
            namespace,
            catalog_name
        );
        spawn(async move {
            loading_table.set(true);
            error_message.set(None);

            match catalog_manager
                .read()
                .load_table(&catalog_name, &namespace, &table_name)
                .await
            {
                Ok(iceberg_table) => {
                    log::info!("Successfully loaded iceberg table, converting...");
                    match iceberg_adapter::convert_iceberg_table(
                        &iceberg_table,
                        namespace.clone(),
                        catalog_name.clone(),
                    ) {
                        Ok(hielo_table) => {
                            log::info!("Table converted successfully, creating tab...");
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
                                log::info!("Switching to existing tab at index: {}", index);
                                active_tab_index.set(index);
                            } else {
                                // Add new tab and switch to it
                                let mut tabs = open_tabs.read().clone();
                                tabs.push(new_tab);
                                let new_index = tabs.len() - 1;
                                log::info!("Adding new tab and switching to index: {}", new_index);
                                open_tabs.set(tabs);
                                active_tab_index.set(new_index);
                            }

                            // Ensure we're in connected state
                            log::info!("Setting app state to Connected");
                            app_state.set(AppState::Connected);
                        }
                        Err(e) => {
                            log::error!("Failed to convert table: {}", e);
                            error_message.set(Some(format!("Failed to convert table: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to load table: {}", e);
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
                // Handle CTRL+K to open global search (only when connected)
                let key_str = format!("{:?}", event.key());
                if event.modifiers().ctrl() && key_str.contains("\"k\"") && matches!(app_state(), AppState::Connected) {
                    show_global_search.set(true);
                    global_search_query.set(String::new());
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
                    // Layout with sidebar and main content
                    div {
                        class: "flex h-screen bg-gray-100",

                        // Left Navigation Pane
                        LeftNavigationPane {
                            collapsed: nav_pane_collapsed(),
                            catalog_manager: catalog_manager,
                            expanded_catalogs: expanded_catalogs,
                            expanded_namespaces: expanded_namespaces,
                            on_toggle_collapse: move |_| nav_pane_collapsed.set(!nav_pane_collapsed()),
                            on_catalog_delete_requested: move |catalog_name: String| {
                                delete_catalog_name.set(catalog_name);
                                show_delete_confirmation.set(true);
                            },
                            on_table_selected: load_table,
                            on_add_catalog: move |_| app_state.set(AppState::CatalogConnection)
                        }

                        // Main Content Area
                        div {
                            class: "flex-1 flex flex-col bg-white",

                            // Header
                            header {
                                class: "bg-white shadow-sm border-b flex-shrink-0",
                                div {
                                    class: "px-4 sm:px-6 lg:px-8",
                                    div {
                                        class: "flex justify-between items-center py-6",
                                        div {
                                            class: "flex items-center space-x-4",
                                            h1 {
                                                class: "text-3xl font-bold text-gray-900",
                                                "🧊 Hielo"
                                            }
                                        }
                                    }
                                }
                            }

                            // Main content
                            main {
                                class: "flex-1 flex flex-col overflow-hidden",

                                // Tab bar
                                if open_tabs.read().len() > 1 {
                                    div {
                                        class: "flex border-b border-gray-200 bg-gray-50",
                                        for (index, tab) in open_tabs.read().iter().enumerate() {
                                            div {
                                                class: format!("flex items-center border-r border-gray-200 {}",
                                                    if index == active_tab_index() {
                                                        "bg-white text-blue-600 border-b-2 border-blue-600"
                                                    } else {
                                                        "text-gray-500 hover:text-gray-700 hover:bg-gray-100"
                                                    }
                                                ),
                                                button {
                                                    onclick: move |_| active_tab_index.set(index),
                                                    class: "px-4 py-2 text-sm font-medium flex-1 text-left",
                                                    {
                                                        match tab {
                                                            AppTab::Catalog => "📁 Catalogs".to_string(),
                                                            AppTab::Table { table, .. } => format!("📊 {}", table.name),
                                                        }
                                                    }
                                                }
                                                if matches!(tab, AppTab::Table { .. }) {
                                                    button {
                                                        onclick: move |e| {
                                                            e.stop_propagation();
                                                            close_tab(index);
                                                        },
                                                        class: "px-2 py-1 hover:bg-gray-200 rounded-full mr-1 text-gray-400 hover:text-gray-600",
                                                        title: "Close tab",
                                                        "×"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                // Tab content
                                div {
                                    class: "flex-1 overflow-y-auto",
                                    if let Some(current_tab) = open_tabs.read().get(active_tab_index()) {
                                        match current_tab {
                                            AppTab::Catalog => rsx! {
                                                div {
                                                    class: "p-6",
                                                    div {
                                                        class: "text-center py-12",
                                                        h2 {
                                                            class: "text-2xl font-semibold text-gray-900 mb-4",
                                                            "Welcome to Hielo! 🧊"
                                                        }
                                                        p {
                                                            class: "text-gray-600 mb-6",
                                                            "Get started by adding a catalog connection, then browse your tables using the left navigation pane."
                                                        }
                                                        div {
                                                            class: "text-sm text-gray-500 space-y-2",
                                                            p { "➕ Click 'Add' in the left panel to connect to a catalog" }
                                                            p { "💡 Press Ctrl+K to search for tables globally" }
                                                            p { "🌳 Click catalog names to expand namespaces" }
                                                            p { "🧊 Click Iceberg tables to open them" }
                                                        }
                                                    }
                                                }
                                            },
                                            AppTab::Table { table, .. } => rsx! {
                                                div {
                                                    class: "h-full flex flex-col",

                                                    // Table sub-tabs
                                                    div {
                                                        class: "flex justify-between border-b border-gray-200 bg-gray-50 px-6",
                                                        div {
                                                            class: "flex",
                                                            button {
                                                                onclick: move |_| table_view_tab.set(TableViewTab::Overview),
                                                                class: format!("px-4 py-2 text-sm font-medium {}",
                                                                    if matches!(table_view_tab(), TableViewTab::Overview) {
                                                                        "text-blue-600 border-b-2 border-blue-600 bg-white"
                                                                    } else {
                                                                        "text-gray-500 hover:text-gray-700"
                                                                    }
                                                                ),
                                                                "Overview"
                                                            }
                                                            button {
                                                                onclick: move |_| table_view_tab.set(TableViewTab::Schema),
                                                                class: format!("px-4 py-2 text-sm font-medium {}",
                                                                    if matches!(table_view_tab(), TableViewTab::Schema) {
                                                                        "text-blue-600 border-b-2 border-blue-600 bg-white"
                                                                    } else {
                                                                        "text-gray-500 hover:text-gray-700"
                                                                    }
                                                                ),
                                                                "Schema"
                                                            }
                                                            button {
                                                                onclick: move |_| table_view_tab.set(TableViewTab::Partitions),
                                                                class: format!("px-4 py-2 text-sm font-medium {}",
                                                                    if matches!(table_view_tab(), TableViewTab::Partitions) {
                                                                        "text-blue-600 border-b-2 border-blue-600 bg-white"
                                                                    } else {
                                                                        "text-gray-500 hover:text-gray-700"
                                                                    }
                                                                ),
                                                                "Partitions"
                                                            }
                                                            button {
                                                                onclick: move |_| table_view_tab.set(TableViewTab::SnapshotHistory),
                                                                class: format!("px-4 py-2 text-sm font-medium {}",
                                                                    if matches!(table_view_tab(), TableViewTab::SnapshotHistory) {
                                                                        "text-blue-600 border-b-2 border-blue-600 bg-white"
                                                                    } else {
                                                                        "text-gray-500 hover:text-gray-700"
                                                                    }
                                                                ),
                                                                "Snapshots"
                                                            }
                                                        }

                                                        // Refresh button
                                                        div {
                                                            class: "flex items-center",
                                                            button {
                                                                onclick: {
                                                                    let table_clone = table.clone();
                                                                    move |_| {
                                                                        log::info!("Refreshing table: {}.{}", table_clone.namespace, table_clone.name);
                                                                        load_table((table_clone.catalog_name.clone(), table_clone.namespace.clone(), table_clone.name.clone()));
                                                                    }
                                                                },
                                                                disabled: loading_table(),
                                                                class: format!("flex items-center px-3 py-2 text-sm font-medium rounded-md {}",
                                                                    if loading_table() {
                                                                        "bg-gray-100 text-gray-400 cursor-not-allowed"
                                                                    } else {
                                                                        "bg-blue-600 text-white hover:bg-blue-700 transition-colors"
                                                                    }
                                                                ),
                                                                title: "Refresh table data",

                                                                if loading_table() {
                                                                    div {
                                                                        class: "animate-spin rounded-full h-4 w-4 border-b-2 border-gray-400 mr-2"
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

                                                    // Table sub-tab content
                                                    div {
                                                        class: "flex-1 overflow-y-auto p-6",
                                                        match table_view_tab() {
                                                            TableViewTab::Overview => rsx! {
                                                                components::TableOverviewTab {
                                                                    table: table.clone()
                                                                }
                                                            },
                                                            TableViewTab::Schema => rsx! {
                                                                components::TableSchemaTab {
                                                                    table: table.clone()
                                                                }
                                                            },
                                                            TableViewTab::Partitions => rsx! {
                                                                components::TablePartitionsTab {
                                                                    table: table.clone()
                                                                }
                                                            },
                                                            TableViewTab::SnapshotHistory => rsx! {
                                                                components::SnapshotTimelineTab {
                                                                    table: table.clone()
                                                                }
                                                            },
                                                        }
                                                    }
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }

                        // Delete confirmation dialog
                        if show_delete_confirmation() {
                            DeleteConfirmationDialog {
                                catalog_name: delete_catalog_name(),
                                on_confirm: move |_| {
                                    let catalog_name_to_delete = delete_catalog_name();
                                    if let Err(e) = catalog_manager.with_mut(|manager| {
                                        manager.delete_catalog(&catalog_name_to_delete)
                                    }) {
                                        error_message.set(Some(format!("Failed to delete catalog: {}", e)));
                                    }
                                    show_delete_confirmation.set(false);
                                    delete_catalog_name.set(String::new());
                                },
                                on_cancel: move |_| {
                                    show_delete_confirmation.set(false);
                                    delete_catalog_name.set(String::new());
                                }
                            }
                        }
                    }
                },
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
    let mut all_tables = use_signal(Vec::<catalog::TableReference>::new);
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
                        "🔍 Find Table (Ctrl+K)"
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
                        "Use Ctrl+K to open this search anytime"
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FilteredTreeNode {
    catalog_name: String,
    catalog_type: catalog::CatalogType,
    show_catalog: bool,
    force_expand_catalog: bool,
    filtered_namespaces: Vec<FilteredNamespaceNode>,
}

#[derive(Clone, Debug, PartialEq)]
struct FilteredNamespaceNode {
    namespace_name: String,
    namespace_key: String,
    show_namespace: bool,
    force_expand_namespace: bool,
    filtered_tables: Vec<catalog::TableReference>,
}

fn compute_filtered_tree(
    saved_catalogs: &[catalog::CatalogConfig],
    catalog_namespaces: &std::collections::HashMap<String, Vec<String>>,
    namespace_tables: &std::collections::HashMap<String, Vec<catalog::TableReference>>,
    filter_query: &str,
) -> Vec<FilteredTreeNode> {
    if filter_query.is_empty() {
        // No filter - return structure indicating all items should be shown normally
        return saved_catalogs
            .iter()
            .map(|catalog| FilteredTreeNode {
                catalog_name: catalog.name.clone(),
                catalog_type: catalog.catalog_type.clone(),
                show_catalog: true,
                force_expand_catalog: false,
                filtered_namespaces: vec![], // Will use normal rendering
            })
            .collect();
    }

    let query_lower = filter_query.to_lowercase();
    let mut result = Vec::new();

    for catalog_config in saved_catalogs {
        let catalog_name = &catalog_config.name;
        let catalog_matches = catalog_name.to_lowercase().contains(&query_lower);

        // Get namespaces for this catalog
        let namespaces = catalog_namespaces
            .get(catalog_name)
            .cloned()
            .unwrap_or_default();

        let mut filtered_namespaces = Vec::new();
        let mut has_matching_children = false;

        for namespace_name in &namespaces {
            let namespace_key = format!("{}::{}", catalog_name, namespace_name);
            let namespace_matches = namespace_name.to_lowercase().contains(&query_lower);

            // Get tables for this namespace
            let tables = namespace_tables
                .get(&namespace_key)
                .cloned()
                .unwrap_or_default();

            let filtered_tables: Vec<catalog::TableReference> = tables
                .into_iter()
                .filter(|table| table.name.to_lowercase().contains(&query_lower))
                .collect();

            let has_matching_tables = !filtered_tables.is_empty();
            let table_matches = has_matching_tables;

            // Show namespace if it matches OR has matching tables
            if namespace_matches || table_matches {
                filtered_namespaces.push(FilteredNamespaceNode {
                    namespace_name: namespace_name.clone(),
                    namespace_key: namespace_key.clone(),
                    show_namespace: true,
                    force_expand_namespace: table_matches, // Auto-expand if contains matching tables
                    filtered_tables,
                });
                has_matching_children = true;
            }
        }

        // Show catalog if it matches OR has matching children
        if catalog_matches || has_matching_children {
            result.push(FilteredTreeNode {
                catalog_name: catalog_name.clone(),
                catalog_type: catalog_config.catalog_type.clone(),
                show_catalog: true,
                force_expand_catalog: has_matching_children, // Auto-expand if contains matches
                filtered_namespaces,
            });
        }
    }

    result
}

fn compute_filter_counts(filtered_tree: &[FilteredTreeNode]) -> (usize, usize, usize) {
    let mut catalog_count = 0;
    let mut namespace_count = 0;
    let mut table_count = 0;

    for catalog in filtered_tree {
        if catalog.show_catalog {
            catalog_count += 1;
        }

        for namespace in &catalog.filtered_namespaces {
            if namespace.show_namespace {
                namespace_count += 1;
            }
            table_count += namespace.filtered_tables.len();
        }
    }

    (catalog_count, namespace_count, table_count)
}

#[component]
fn LeftNavigationPane(
    collapsed: bool,
    catalog_manager: Signal<CatalogManager>,
    expanded_catalogs: Signal<std::collections::HashSet<String>>,
    expanded_namespaces: Signal<std::collections::HashSet<String>>,
    on_toggle_collapse: EventHandler<()>,
    on_catalog_delete_requested: EventHandler<String>,
    on_table_selected: EventHandler<(String, String, String)>,
    on_add_catalog: EventHandler<()>,
) -> Element {
    let mut namespace_tables =
        use_signal(std::collections::HashMap::<String, Vec<catalog::TableReference>>::new);
    let mut loading_namespaces = use_signal(std::collections::HashSet::<String>::new);
    let mut catalog_namespaces = use_signal(std::collections::HashMap::<String, Vec<String>>::new);
    let mut nav_filter_query = use_signal(String::new);
    let mut debounced_filter_query = use_signal(String::new);

    // Debounce filter input
    use_effect(move || {
        let query = nav_filter_query();
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            if nav_filter_query() == query {
                debounced_filter_query.set(query);
            }
        });
    });

    let load_catalog_namespaces = move |catalog_name: String| {
        log::info!("Loading namespaces for catalog: {}", catalog_name);
        spawn(async move {
            // First, ensure the catalog is connected
            let catalog_config = {
                let manager = catalog_manager.read();
                manager
                    .get_saved_catalogs()
                    .iter()
                    .find(|c| c.name == catalog_name)
                    .cloned()
            };

            if let Some(config) = catalog_config {
                // Try to connect the catalog if not already connected
                let is_connected = catalog_manager
                    .read()
                    .get_connections()
                    .iter()
                    .any(|conn| conn.config.name == catalog_name);

                if !is_connected {
                    log::info!("Catalog not connected, connecting: {}", catalog_name);
                    let connect_result = {
                        let mut manager_guard = catalog_manager.write();
                        manager_guard.connect_catalog(config).await
                    };

                    match connect_result {
                        Ok(_) => {
                            log::info!("Successfully connected to catalog: {}", catalog_name);
                        }
                        Err(e) => {
                            log::error!("Failed to connect catalog {}: {}", catalog_name, e);
                            return;
                        }
                    }
                }

                // Now try to list namespaces
                log::info!("Loading namespaces for connected catalog: {}", catalog_name);
                match catalog_manager.read().list_namespaces(&catalog_name).await {
                    Ok(ns_list) => {
                        log::info!(
                            "Loaded {} namespaces for catalog {}",
                            ns_list.len(),
                            catalog_name
                        );
                        catalog_namespaces.with_mut(|namespaces| {
                            namespaces.insert(catalog_name.clone(), ns_list);
                        });
                    }
                    Err(e) => {
                        log::error!(
                            "Failed to load namespaces for catalog {}: {}",
                            catalog_name,
                            e
                        );
                    }
                }
            } else {
                log::error!("Catalog configuration not found for: {}", catalog_name);
            }
        });
    };

    let mut toggle_catalog_expansion = move |catalog_name: String| {
        log::info!("Toggling catalog expansion for: {}", catalog_name);
        let should_expand = !expanded_catalogs.read().contains(&catalog_name);

        expanded_catalogs.with_mut(|expanded| {
            if expanded.contains(&catalog_name) {
                log::info!("Collapsing catalog: {}", catalog_name);
                expanded.remove(&catalog_name);
            } else {
                log::info!("Expanding catalog: {}", catalog_name);
                expanded.insert(catalog_name.clone());
            }
        });

        // If expanding, also trigger namespace loading
        if should_expand {
            load_catalog_namespaces(catalog_name);
        }
    };

    let mut toggle_namespace_expansion = move |namespace_key: String| {
        let namespace_parts: Vec<&str> = namespace_key.split("::").collect();
        if namespace_parts.len() == 2 {
            let catalog_name = namespace_parts[0];
            let namespace_name = namespace_parts[1];

            let should_expand = !expanded_namespaces.read().contains(&namespace_key);

            expanded_namespaces.with_mut(|expanded| {
                if expanded.contains(&namespace_key) {
                    expanded.remove(&namespace_key);
                } else {
                    expanded.insert(namespace_key.clone());
                }
            });

            if should_expand {
                // Load tables for this namespace
                let catalog_name = catalog_name.to_string();
                let namespace_name = namespace_name.to_string();
                let namespace_key_clone = namespace_key.clone();

                spawn(async move {
                    loading_namespaces.with_mut(|loading| {
                        loading.insert(namespace_key_clone.clone());
                    });

                    match catalog_manager
                        .read()
                        .list_tables(&catalog_name, &namespace_name)
                        .await
                    {
                        Ok(tables) => {
                            namespace_tables.with_mut(|map| {
                                map.insert(namespace_key_clone.clone(), tables);
                            });
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to load tables for namespace {}: {}",
                                namespace_key_clone,
                                e
                            );
                        }
                    }

                    loading_namespaces.with_mut(|loading| {
                        loading.remove(&namespace_key_clone);
                    });
                });
            }
        }
    };

    let saved_catalogs = catalog_manager.read().get_saved_catalogs().to_vec();

    // Add filtered tree computation using debounced query
    let filtered_tree = {
        let query = debounced_filter_query();
        let namespaces = catalog_namespaces();
        let tables = namespace_tables();
        compute_filtered_tree(&saved_catalogs, &namespaces, &tables, &query)
    };

    // Compute filter result counts
    let (catalog_count, namespace_count, table_count) = if !debounced_filter_query().is_empty() {
        compute_filter_counts(&filtered_tree)
    } else {
        (0, 0, 0)
    };

    rsx! {
        div {
            class: format!("bg-white border-r border-gray-200 flex flex-col transition-all duration-300 {}",
                if collapsed { "w-12" } else { "w-80" }
            ),

            // Header with Toolbar
            div {
                class: "p-4 border-b border-gray-200",
                if !collapsed {
                    // Search input section
                    div {
                        class: "mb-3",
                        div {
                            class: "relative",
                            div {
                                class: "absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none",
                                svg {
                                    class: "h-4 w-4 text-gray-400",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                                    }
                                }
                            }
                            input {
                                r#type: "text",
                                value: "{nav_filter_query()}",
                                oninput: move |evt| nav_filter_query.set(evt.value()),
                                onkeydown: move |evt| {
                                    if evt.key() == Key::Escape {
                                        nav_filter_query.set(String::new());
                                    }
                                },
                                class: "block w-full pl-9 pr-8 py-1.5 text-sm border border-gray-300 rounded-md bg-white placeholder-gray-500 focus:outline-none focus:ring-1 focus:ring-blue-500 focus:border-blue-500",
                                placeholder: "Filter catalogs, namespaces, tables..."
                            }
                            // Clear button
                            if !nav_filter_query().is_empty() {
                                button {
                                    onclick: move |_| nav_filter_query.set(String::new()),
                                    class: "absolute inset-y-0 right-0 pr-2 flex items-center text-gray-400 hover:text-gray-600",
                                    svg {
                                        class: "h-4 w-4",
                                        fill: "none",
                                        stroke: "currentColor",
                                        view_box: "0 0 24 24",
                                        path {
                                            stroke_linecap: "round",
                                            stroke_linejoin: "round",
                                            stroke_width: "2",
                                            d: "M6 18L18 6M6 6l12 12"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Filter result counts
                if !nav_filter_query().is_empty() {
                    div {
                        class: "px-3 py-1 text-xs text-gray-600 bg-gray-50 border-b border-gray-200",
                        if catalog_count > 0 || namespace_count > 0 || table_count > 0 {
                            span {
                                "Showing "
                                span { class: "font-medium", "{catalog_count}" }
                                " catalogs, "
                                span { class: "font-medium", "{namespace_count}" }
                                " namespaces, "
                                span { class: "font-medium", "{table_count}" }
                                " tables"
                            }
                        } else if !debounced_filter_query().is_empty() {
                            span {
                                class: "text-gray-500 italic",
                                "No matching results found"
                            }
                        }
                    }
                }

                // Title and controls
                div {
                    class: "flex items-center justify-between",
                    if !collapsed {
                        div {
                            class: "flex items-center gap-2 flex-1",
                            h2 {
                                class: "text-lg font-semibold text-gray-900",
                                "📚 Catalogs"
                            }
                            // Add Catalog Button
                            button {
                                onclick: move |_| on_add_catalog.call(()),
                                class: "ml-auto px-3 py-1 bg-blue-600 text-white text-sm rounded-md hover:bg-blue-700 transition-colors flex items-center gap-1",
                                title: "Add New Catalog",
                                span { "+" }
                                span { "Add" }
                            }
                        }
                    }
                    button {
                        onclick: move |_| on_toggle_collapse.call(()),
                        class: "p-1 rounded hover:bg-gray-100 transition-colors",
                        svg {
                            class: "h-5 w-5 text-gray-500",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: if collapsed { "M9 5l7 7-7 7" } else { "M15 19l-7-7 7-7" }
                            }
                        }
                    }
                }
            }

            if !collapsed {
                // Catalog list
                div {
                    class: "flex-1 overflow-y-auto p-2",
                    if saved_catalogs.is_empty() {
                        div {
                            class: "text-center py-8 text-gray-500",
                            div { "📚" }
                            div { class: "text-sm mt-2", "No catalogs configured" }
                            div { class: "text-xs mt-1", "Click 'Add' to get started" }
                        }
                    } else {
                        div {
                            class: "space-y-1",
                            for filtered_catalog in filtered_tree.iter() {
                                CatalogTreeNode {
                                    catalog_name: filtered_catalog.catalog_name.clone(),
                                    catalog_type: filtered_catalog.catalog_type.clone(),
                                    expanded: expanded_catalogs.read().contains(&filtered_catalog.catalog_name) || filtered_catalog.force_expand_catalog,
                                    filtered_namespaces: if nav_filter_query().is_empty() {
                                        None
                                    } else {
                                        Some(filtered_catalog.filtered_namespaces.clone())
                                    },
                                    expanded_namespaces: expanded_namespaces,
                                    namespace_tables: namespace_tables,
                                    loading_namespaces: loading_namespaces,
                                    catalog_namespaces: catalog_namespaces,
                                    catalog_manager: catalog_manager,
                                    on_toggle_catalog: move |name: String| toggle_catalog_expansion(name),
                                    on_toggle_namespace: move |key: String| toggle_namespace_expansion(key),
                                    on_delete_catalog: on_catalog_delete_requested,
                                    on_table_selected: on_table_selected
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
fn CatalogTreeNode(
    catalog_name: String,
    catalog_type: catalog::CatalogType,
    expanded: bool,
    filtered_namespaces: Option<Vec<FilteredNamespaceNode>>, // New parameter
    expanded_namespaces: Signal<std::collections::HashSet<String>>,
    namespace_tables: Signal<std::collections::HashMap<String, Vec<catalog::TableReference>>>,
    loading_namespaces: Signal<std::collections::HashSet<String>>,
    catalog_namespaces: Signal<std::collections::HashMap<String, Vec<String>>>,
    catalog_manager: Signal<CatalogManager>,
    on_toggle_catalog: EventHandler<String>,
    on_toggle_namespace: EventHandler<String>,
    on_delete_catalog: EventHandler<String>,
    on_table_selected: EventHandler<(String, String, String)>,
) -> Element {
    // Use filtered namespaces if provided, otherwise use normal logic
    let namespaces_to_render = if let Some(filtered) = &filtered_namespaces {
        filtered
            .iter()
            .map(|fn_node| fn_node.namespace_name.clone())
            .collect::<Vec<_>>()
    } else {
        catalog_namespaces
            .read()
            .get(&catalog_name)
            .cloned()
            .unwrap_or_default()
    };

    // Simple loading check: if expanded but no namespaces loaded yet
    let loading_catalog = expanded && namespaces_to_render.is_empty();

    let catalog_icon = match catalog_type {
        catalog::CatalogType::Rest => "🌐",
        catalog::CatalogType::Glue => "🔗",
    };

    rsx! {
        div {
            class: "select-none",

            // Catalog header
            div {
                class: "flex items-center justify-between group hover:bg-gray-50 rounded px-2 py-1",

                // Expand button and catalog name
                div {
                    class: "flex items-center flex-1 cursor-pointer",
                    onclick: {
                        let catalog_name_toggle = catalog_name.clone();
                        move |_| {
                            log::info!("Catalog clicked: {}", catalog_name_toggle);
                            on_toggle_catalog.call(catalog_name_toggle.clone())
                        }
                    },

                    // Expand/collapse icon
                    div {
                        class: "w-4 h-4 mr-1 flex items-center justify-center",
                        if loading_catalog {
                            div {
                                class: "animate-spin rounded-full h-3 w-3 border border-gray-300 border-t-blue-600"
                            }
                        } else {
                            svg {
                                class: format!("h-3 w-3 text-gray-500 transition-transform {}",
                                    if expanded { "rotate-90" } else { "" }
                                ),
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M9 5l7 7-7 7"
                                }
                            }
                        }
                    }

                    // Catalog icon and name
                    span { class: "text-sm mr-2", "{catalog_icon}" }
                    span { class: "text-sm font-medium text-gray-900 truncate", "{catalog_name}" }
                }

                // Delete button
                button {
                    onclick: {
                        let catalog_name_delete = catalog_name.clone();
                        move |e: dioxus::prelude::Event<dioxus::html::MouseData>| {
                            e.stop_propagation();
                            on_delete_catalog.call(catalog_name_delete.clone());
                        }
                    },
                    class: "opacity-0 group-hover:opacity-100 p-1 hover:bg-red-100 rounded transition-all",
                    title: "Delete catalog",
                    svg {
                        class: "h-3 w-3 text-red-500",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16"
                        }
                    }
                }
            }

            // Namespaces (when expanded)
            if expanded {
                div {
                    class: "ml-4 mt-1 space-y-1",
                    for namespace_name in namespaces_to_render.iter() {
                        {
                            let filtered_ns_data = filtered_namespaces
                                .as_ref()
                                .and_then(|filtered| filtered.iter().find(|fn_node| &fn_node.namespace_name == namespace_name));

                            let namespace_key = format!("{}::{}", catalog_name, namespace_name);
                            let should_expand = expanded_namespaces.read().contains(&namespace_key)
                                || filtered_ns_data.map(|ns| ns.force_expand_namespace).unwrap_or(false);

                            rsx! {
                                NamespaceTreeNode {
                                    catalog_name: catalog_name.clone(),
                                    namespace_name: namespace_name.clone(),
                                    namespace_key: namespace_key.clone(),
                                    expanded: should_expand,
                                    filtered_tables: filtered_ns_data.map(|ns| ns.filtered_tables.clone()),
                                    namespace_tables: namespace_tables,
                                    loading_namespaces: loading_namespaces,
                                    on_toggle_namespace: on_toggle_namespace,
                                    on_table_selected: on_table_selected
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
fn NamespaceTreeNode(
    catalog_name: String,
    namespace_name: String,
    namespace_key: String,
    expanded: bool,
    filtered_tables: Option<Vec<catalog::TableReference>>, // New parameter
    namespace_tables: Signal<std::collections::HashMap<String, Vec<catalog::TableReference>>>,
    loading_namespaces: Signal<std::collections::HashSet<String>>,
    on_toggle_namespace: EventHandler<String>,
    on_table_selected: EventHandler<(String, String, String)>,
) -> Element {
    let is_loading = loading_namespaces.read().contains(&namespace_key);

    // Use filtered tables if provided, otherwise use normal logic
    let tables_to_render = if let Some(filtered) = &filtered_tables {
        filtered.clone()
    } else {
        namespace_tables
            .read()
            .get(&namespace_key)
            .cloned()
            .unwrap_or_default()
    };

    rsx! {
        div {
            class: "select-none",

            // Namespace header
            div {
                class: "flex items-center hover:bg-gray-50 rounded px-2 py-1 cursor-pointer",
                onclick: move |_| on_toggle_namespace.call(namespace_key.clone()),

                // Expand/collapse icon
                div {
                    class: "w-4 h-4 mr-1 flex items-center justify-center",
                    if is_loading {
                        div {
                            class: "animate-spin rounded-full h-3 w-3 border border-gray-300 border-t-blue-600"
                        }
                    } else {
                        svg {
                            class: format!("h-3 w-3 text-gray-400 transition-transform {}",
                                if expanded { "rotate-90" } else { "" }
                            ),
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M9 5l7 7-7 7"
                            }
                        }
                    }
                }

                // Namespace icon and name
                span { class: "text-sm mr-2", "📁" }
                span {
                    class: "text-sm text-gray-700 truncate",
                    "{namespace_name}"
                }
            }

            // Tables (when expanded)
            if expanded && !is_loading {
                div {
                    class: "ml-4 mt-1 space-y-1",
                    if tables_to_render.is_empty() {
                        div {
                            class: "px-2 py-1 text-xs text-gray-500 italic",
                            "No tables found"
                        }
                    } else {
                        for table in tables_to_render.iter() {
                            div {
                                class: format!("flex items-center px-2 py-1 rounded transition-colors {}",
                                    if table.table_type == catalog::TableType::Iceberg {
                                        "hover:bg-blue-50 cursor-pointer"
                                    } else {
                                        "cursor-not-allowed opacity-50"
                                    }
                                ),
                                onclick: {
                                    let catalog_name = catalog_name.clone();
                                    let namespace_name = namespace_name.clone();
                                    let table_name = table.name.clone();
                                    let table_type = table.table_type;
                                    move |_| {
                                        if table_type == catalog::TableType::Iceberg {
                                            on_table_selected.call((catalog_name.clone(), namespace_name.clone(), table_name.clone()));
                                        }
                                    }
                                },

                                span {
                                    class: "text-sm mr-2",
                                    if table.table_type == catalog::TableType::Iceberg { "🧊" } else { "📄" }
                                }
                                span {
                                    class: format!("text-xs truncate {}",
                                        if table.table_type == catalog::TableType::Iceberg {
                                            "text-gray-800"
                                        } else {
                                            "text-gray-500"
                                        }
                                    ),
                                    "{table.name}"
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
fn DeleteConfirmationDialog(
    catalog_name: String,
    on_confirm: EventHandler<()>,
    on_cancel: EventHandler<()>,
) -> Element {
    rsx! {
        // Modal overlay
        div {
            class: "fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center",
            onclick: move |_| on_cancel.call(()),

            // Modal content
            div {
                class: "bg-white rounded-lg shadow-xl max-w-md w-full mx-4",
                onclick: |e| e.stop_propagation(),

                // Header
                div {
                    class: "flex items-center justify-between p-4 border-b border-gray-200",
                    h3 {
                        class: "text-lg font-medium text-gray-900",
                        "🗑️ Delete Catalog"
                    }
                    button {
                        onclick: move |_| on_cancel.call(()),
                        class: "text-gray-400 hover:text-gray-600",
                        "✕"
                    }
                }

                // Content
                div {
                    class: "p-4",
                    p {
                        class: "text-sm text-gray-600 mb-4",
                        "Are you sure you want to delete the catalog \""
                        span { class: "font-medium", "{catalog_name}" }
                        "\"? This action cannot be undone."
                    }
                    p {
                        class: "text-xs text-gray-500",
                        "Note: This will only remove the catalog from Hielo's saved connections. It will not affect the actual catalog or its data."
                    }
                }

                // Actions
                div {
                    class: "flex justify-end space-x-3 p-4 border-t border-gray-200",
                    button {
                        onclick: move |_| on_cancel.call(()),
                        class: "px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 hover:bg-gray-200 rounded-md transition-colors",
                        "Cancel"
                    }
                    button {
                        onclick: move |_| on_confirm.call(()),
                        class: "px-4 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 rounded-md transition-colors",
                        "Delete"
                    }
                }
            }
        }
    }
}
