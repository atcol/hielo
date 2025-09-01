use crate::catalog::{CatalogManager, CatalogType};
use dioxus::prelude::*;
use std::collections::{HashMap, HashSet};

#[component]
pub fn LeftNavigationPane(
    collapsed: bool,
    catalog_manager: Signal<CatalogManager>,
    expanded_catalogs: Signal<HashSet<String>>,
    expanded_namespaces: Signal<HashSet<String>>,
    on_toggle_collapse: EventHandler<()>,
    on_catalog_delete_requested: EventHandler<String>,
    on_table_selected: EventHandler<(String, String, String)>,
    on_add_catalog: EventHandler<()>,
) -> Element {
    let mut namespace_tables =
        use_signal(HashMap::<String, Vec<crate::catalog::TableReference>>::new);
    let mut loading_namespaces = use_signal(HashSet::<String>::new);
    let mut catalog_namespaces = use_signal(HashMap::<String, Vec<String>>::new);

    let load_catalog_namespaces = move |catalog_name: String| {
        log::info!("Loading namespaces for catalog: {}", catalog_name);
        spawn(async move {
            // First, ensure the catalog is connected
            let catalog_config = {
                let manager = catalog_manager.read();
                manager.get_saved_catalogs()
                    .iter()
                    .find(|c| c.name == catalog_name)
                    .cloned()
            };
            
            if let Some(config) = catalog_config {
                // Try to connect the catalog if not already connected
                let is_connected = catalog_manager.read()
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
                        log::info!("Loaded {} namespaces for catalog {}", ns_list.len(), catalog_name);
                        catalog_namespaces.with_mut(|namespaces| {
                            namespaces.insert(catalog_name.clone(), ns_list);
                        });
                    }
                    Err(e) => {
                        log::error!("Failed to load namespaces for catalog {}: {}", catalog_name, e);
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

    rsx! {
        div {
            class: format!("bg-white border-r border-gray-200 flex flex-col transition-all duration-300 {}",
                if collapsed { "w-12" } else { "w-80" }
            ),

            // Header with Toolbar
            div {
                class: "p-4 border-b border-gray-200 flex items-center justify-between",
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
                            for catalog_config in saved_catalogs.iter() {
                                CatalogTreeNode {
                                    catalog_name: catalog_config.name.clone(),
                                    catalog_type: catalog_config.catalog_type.clone(),
                                    expanded: expanded_catalogs.read().contains(&catalog_config.name),
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
    catalog_type: CatalogType,
    expanded: bool,
    expanded_namespaces: Signal<HashSet<String>>,
    namespace_tables: Signal<HashMap<String, Vec<crate::catalog::TableReference>>>,
    loading_namespaces: Signal<HashSet<String>>,
    catalog_namespaces: Signal<HashMap<String, Vec<String>>>,
    catalog_manager: Signal<CatalogManager>,
    on_toggle_catalog: EventHandler<String>,
    on_toggle_namespace: EventHandler<String>,
    on_delete_catalog: EventHandler<String>,
    on_table_selected: EventHandler<(String, String, String)>,
) -> Element {
    // Get namespaces for this catalog from the shared state
    let namespaces = catalog_namespaces.read().get(&catalog_name).cloned().unwrap_or_default();
    
    // Simple loading check: if expanded but no namespaces loaded yet
    let loading_catalog = expanded && namespaces.is_empty();

    let catalog_icon = match catalog_type {
        CatalogType::Rest => "🌐",
        CatalogType::Glue => "🔗",
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
                    for namespace in namespaces.iter() {
                        NamespaceTreeNode {
                            catalog_name: catalog_name.clone(),
                            namespace_name: namespace.clone(),
                            namespace_key: format!("{}::{}", catalog_name, namespace),
                            expanded: expanded_namespaces.read().contains(&format!("{}::{}", catalog_name, namespace)),
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

#[component]
fn NamespaceTreeNode(
    catalog_name: String,
    namespace_name: String,
    namespace_key: String,
    expanded: bool,
    namespace_tables: Signal<HashMap<String, Vec<crate::catalog::TableReference>>>,
    loading_namespaces: Signal<HashSet<String>>,
    on_toggle_namespace: EventHandler<String>,
    on_table_selected: EventHandler<(String, String, String)>,
) -> Element {
    let is_loading = loading_namespaces.read().contains(&namespace_key);
    let tables = namespace_tables
        .read()
        .get(&namespace_key)
        .cloned()
        .unwrap_or_default();

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
                    if tables.is_empty() {
                        div {
                            class: "px-2 py-1 text-xs text-gray-500 italic",
                            "No tables found"
                        }
                    } else {
                        for table in tables.iter() {
                            div {
                                class: format!("flex items-center px-2 py-1 rounded transition-colors {}",
                                    if table.table_type == crate::catalog::TableType::Iceberg {
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
                                        if table_type == crate::catalog::TableType::Iceberg {
                                            on_table_selected.call((catalog_name.clone(), namespace_name.clone(), table_name.clone()));
                                        }
                                    }
                                },

                                span {
                                    class: "text-sm mr-2",
                                    if table.table_type == crate::catalog::TableType::Iceberg { "🧊" } else { "📄" }
                                }
                                span {
                                    class: format!("text-xs truncate {}",
                                        if table.table_type == crate::catalog::TableType::Iceberg {
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