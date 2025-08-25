use dioxus::prelude::*;

mod catalog;
mod catalog_ui;
mod components;
mod data;
mod iceberg_adapter;

use catalog::CatalogManager;
use catalog_ui::{CatalogBrowser, CatalogConnectionScreen};
use components::{SnapshotTimelineTab, TableInfoTab};
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
    TableInfo,
    SnapshotHistory,
}

fn main() {
    dioxus_logger::init(log::LevelFilter::Info).expect("failed to init logger");
    launch(App);
}

fn App() -> Element {
    let mut app_state = use_signal(|| AppState::CatalogConnection);
    let mut open_tabs = use_signal(|| vec![AppTab::Catalog]);
    let mut active_tab_index = use_signal(|| 0usize);
    let mut table_view_tab = use_signal(|| TableViewTab::TableInfo);
    let catalog_manager = use_signal(CatalogManager::new);
    let mut loading_table = use_signal(|| false);
    let mut error_message = use_signal(|| Option::<String>::None);

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
                    match iceberg_adapter::convert_iceberg_table(&iceberg_table, namespace.clone())
                    {
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
                                h1 {
                                    class: "text-3xl font-bold text-gray-900",
                                    "🧊 Hielo"
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
                                        on_table_selected: load_table
                                    }
                                },
                                AppTab::Table { table, .. } => rsx! {
                                    // Table sub-tabs
                                    div {
                                        class: "mb-6",
                                        nav {
                                            class: "flex space-x-8 border-b border-gray-200",
                                            button {
                                                class: format!(
                                                    "py-2 px-1 border-b-2 font-medium text-sm transition-colors {}",
                                                    if *table_view_tab.read() == TableViewTab::TableInfo {
                                                        "border-blue-500 text-blue-600"
                                                    } else {
                                                        "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300"
                                                    }
                                                ),
                                                onclick: move |_| table_view_tab.set(TableViewTab::TableInfo),
                                                "📋 Table Information"
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
                                    }

                                    // Table content
                                    match *table_view_tab.read() {
                                        TableViewTab::TableInfo => rsx! {
                                            TableInfoTab { table: table.clone() }
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
