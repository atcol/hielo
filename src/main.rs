#![allow(non_snake_case)]

use dioxus::prelude::*;

mod catalog;
mod catalog_ui;
mod components;
mod config;
mod data;
mod iceberg_adapter;
mod left_nav;

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
    let table_view_tab = use_signal(|| TableViewTab::Overview);
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

    let load_table = move |(catalog_name, namespace, table_name): (String, String, String)| {
        log::info!("Loading table: {} from namespace: {} in catalog: {}", table_name, namespace, catalog_name);
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
                        left_nav::LeftNavigationPane {
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
                                            button {
                                                onclick: move |_| active_tab_index.set(index),
                                                class: format!("px-4 py-2 text-sm font-medium border-r border-gray-200 {}",
                                                    if index == active_tab_index() {
                                                        "bg-white text-blue-600 border-b-2 border-blue-600"
                                                    } else {
                                                        "text-gray-500 hover:text-gray-700 hover:bg-gray-100"
                                                    }
                                                ),
                                                {
                                                    match tab {
                                                        AppTab::Catalog => "📁 Catalogs".to_string(),
                                                        AppTab::Table { table, .. } => format!("📊 {}", table.name),
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
                                                    class: "h-full p-6",
                                                    components::TableOverviewTab {
                                                        table: table.clone()
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
