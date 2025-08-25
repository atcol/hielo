use crate::catalog::{CatalogConfig, CatalogManager, CatalogType, TableReference};
use dioxus::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum CatalogFormType {
    Rest,
    Glue,
}

#[component]
pub fn CatalogConnectionScreen(
    catalog_manager: Signal<CatalogManager>,
    on_catalog_connected: EventHandler<()>,
    on_table_selected: EventHandler<(String, String, String)>, // (catalog_name, namespace, table_name)
) -> Element {
    let mut selected_catalog_type = use_signal(|| CatalogFormType::Rest);
    let connection_status = use_signal(|| ConnectionStatus::Disconnected);
    let selected_namespace = use_signal(|| Option::<String>::None);
    let selected_table = use_signal(|| Option::<TableReference>::None);
    let namespaces = use_signal(Vec::<String>::new);
    let tables = use_signal(Vec::<TableReference>::new);

    rsx! {
        div {
            class: "min-h-screen bg-gray-100",

            // Header
            header {
                class: "bg-white shadow-sm border-b",
                div {
                    class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                    div {
                        class: "flex justify-between items-center py-6",
                        h1 {
                            class: "text-3xl font-bold text-gray-900",
                            "🧊 Hielo - Connect to Catalog"
                        }
                    }
                }
            }

            main {
                class: "max-w-4xl mx-auto py-6 px-4 sm:px-6 lg:px-8 space-y-6",

                // Connection Form
                div {
                    class: "bg-white shadow rounded-lg",
                    div {
                        class: "px-4 py-5 sm:p-6",
                        h3 {
                            class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                            "Connect to Iceberg Catalog"
                        }

                        // Catalog Type Selection
                        div {
                            class: "mb-6",
                            label {
                                class: "block text-sm font-medium text-gray-700 mb-2",
                                "Catalog Type"
                            }
                            div {
                                class: "flex space-x-4",
                                label {
                                    class: "flex items-center",
                                    input {
                                        r#type: "radio",
                                        name: "catalog_type",
                                        checked: *selected_catalog_type.read() == CatalogFormType::Rest,
                                        onchange: move |_| selected_catalog_type.set(CatalogFormType::Rest),
                                        class: "mr-2"
                                    }
                                    "REST Catalog"
                                }
                                label {
                                    class: "flex items-center",
                                    input {
                                        r#type: "radio",
                                        name: "catalog_type",
                                        checked: *selected_catalog_type.read() == CatalogFormType::Glue,
                                        onchange: move |_| selected_catalog_type.set(CatalogFormType::Glue),
                                        class: "mr-2"
                                    }
                                    "AWS Glue Catalog"
                                }
                            }
                        }

                        // Form based on selected type
                        match *selected_catalog_type.read() {
                            CatalogFormType::Rest => rsx! {
                                RestCatalogForm {
                                    connection_status: connection_status,
                                    catalog_manager: catalog_manager,
                                    namespaces: namespaces,
                                    on_catalog_connected: on_catalog_connected,
                                }
                            },
                            CatalogFormType::Glue => rsx! {
                                GlueCatalogForm {
                                    connection_status: connection_status,
                                    catalog_manager: catalog_manager,
                                    namespaces: namespaces,
                                    on_catalog_connected: on_catalog_connected,
                                }
                            },
                        }
                    }
                }

                // Connection Status
                ConnectionStatusDisplay { status: connection_status() }

                // Namespace and Table Selection
                if matches!(connection_status(), ConnectionStatus::Connected) {
                    TableBrowser {
                        catalog_manager: catalog_manager,
                        namespaces: namespaces(),
                        selected_namespace: selected_namespace,
                        tables: tables,
                        on_table_selected,
                    }
                }
            }
        }
    }
}

#[component]
fn RestCatalogForm(
    connection_status: Signal<ConnectionStatus>,
    catalog_manager: Signal<CatalogManager>,
    namespaces: Signal<Vec<String>>,
    on_catalog_connected: EventHandler<()>,
) -> Element {
    let mut catalog_name = use_signal(|| "rest-catalog".to_string());
    let mut uri = use_signal(|| "".to_string());
    let mut warehouse = use_signal(|| "".to_string());
    let mut auth_token = use_signal(|| "".to_string());

    let connect = move |_| async move {
        connection_status.set(ConnectionStatus::Connecting);

        let mut config = HashMap::new();
        config.insert("uri".to_string(), uri());
        if !warehouse().is_empty() {
            config.insert("warehouse".to_string(), warehouse());
        }
        if !auth_token().is_empty() {
            config.insert("auth_token".to_string(), auth_token());
        }

        let catalog_config = CatalogConfig {
            catalog_type: CatalogType::Rest,
            name: catalog_name(),
            config,
        };

        let connection_result = catalog_manager
            .write()
            .connect_catalog(catalog_config.clone())
            .await;
        match connection_result {
            Ok(()) => {
                connection_status.set(ConnectionStatus::Connected);
                // Load namespaces
                match catalog_manager
                    .read()
                    .list_namespaces(&catalog_name())
                    .await
                {
                    Ok(ns) => {
                        namespaces.set(ns);
                        // Call the connected callback to switch to tabbed interface
                        on_catalog_connected.call(());
                    }
                    Err(e) => connection_status.set(ConnectionStatus::Error(e.to_string())),
                }
            }
            Err(e) => connection_status.set(ConnectionStatus::Error(e.to_string())),
        }
    };

    rsx! {
        div {
            class: "space-y-4",

            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "Catalog Name"
                }
                input {
                    r#type: "text",
                    value: "{catalog_name}",
                    oninput: move |evt| catalog_name.set(evt.value()),
                    class: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "my-rest-catalog"
                }
            }

            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "REST Endpoint URI *"
                }
                input {
                    r#type: "url",
                    value: "{uri}",
                    oninput: move |evt| uri.set(evt.value()),
                    class: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "http://localhost:8181"
                }
            }

            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "Warehouse Location (Optional)"
                }
                input {
                    r#type: "text",
                    value: "{warehouse}",
                    oninput: move |evt| warehouse.set(evt.value()),
                    class: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "s3://my-bucket/warehouse/"
                }
            }

            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "Auth Token (Optional)"
                }
                input {
                    r#type: "password",
                    value: "{auth_token}",
                    oninput: move |evt| auth_token.set(evt.value()),
                    class: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "Bearer token or API key"
                }
            }

            button {
                onclick: connect,
                disabled: uri().is_empty() || matches!(connection_status(), ConnectionStatus::Connecting),
                class: format!(
                    "w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white {}",
                    if matches!(connection_status(), ConnectionStatus::Connecting) {
                        "bg-gray-400 cursor-not-allowed"
                    } else {
                        "bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                    }
                ),
                if matches!(connection_status(), ConnectionStatus::Connecting) {
                    "Connecting..."
                } else {
                    "Connect to REST Catalog"
                }
            }
        }
    }
}

#[component]
fn GlueCatalogForm(
    connection_status: Signal<ConnectionStatus>,
    catalog_manager: Signal<CatalogManager>,
    namespaces: Signal<Vec<String>>,
    on_catalog_connected: EventHandler<()>,
) -> Element {
    let mut catalog_name = use_signal(|| "glue-catalog".to_string());
    let mut warehouse = use_signal(|| "".to_string());
    let mut region = use_signal(|| "us-east-1".to_string());
    let mut profile = use_signal(|| "".to_string());

    let connect = move |_| async move {
        connection_status.set(ConnectionStatus::Connecting);

        let mut config = HashMap::new();
        config.insert("warehouse".to_string(), warehouse());
        config.insert("region".to_string(), region());
        if !profile().is_empty() {
            config.insert("profile".to_string(), profile());
        }

        let catalog_config = CatalogConfig {
            catalog_type: CatalogType::Glue,
            name: catalog_name(),
            config,
        };

        let connection_result = catalog_manager
            .write()
            .connect_catalog(catalog_config.clone())
            .await;
        match connection_result {
            Ok(()) => {
                connection_status.set(ConnectionStatus::Connected);
                // Load namespaces
                match catalog_manager
                    .read()
                    .list_namespaces(&catalog_name())
                    .await
                {
                    Ok(ns) => {
                        namespaces.set(ns);
                        // Call the connected callback to switch to tabbed interface
                        on_catalog_connected.call(());
                    }
                    Err(e) => connection_status.set(ConnectionStatus::Error(e.to_string())),
                }
            }
            Err(e) => connection_status.set(ConnectionStatus::Error(e.to_string())),
        }
    };

    rsx! {
        div {
            class: "space-y-4",

            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "Catalog Name"
                }
                input {
                    r#type: "text",
                    value: "{catalog_name}",
                    oninput: move |evt| catalog_name.set(evt.value()),
                    class: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "my-glue-catalog"
                }
            }

            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "S3 Warehouse Location *"
                }
                input {
                    r#type: "text",
                    value: "{warehouse}",
                    oninput: move |evt| warehouse.set(evt.value()),
                    class: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "s3://my-bucket/warehouse/"
                }
            }

            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "AWS Region"
                }
                input {
                    r#type: "text",
                    value: "{region}",
                    oninput: move |evt| region.set(evt.value()),
                    class: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "us-east-1"
                }
            }

            div {
                label {
                    class: "block text-sm font-medium text-gray-700",
                    "AWS Profile (Optional)"
                }
                input {
                    r#type: "text",
                    value: "{profile}",
                    oninput: move |evt| profile.set(evt.value()),
                    class: "mt-1 block w-full border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500",
                    placeholder: "default"
                }
                p {
                    class: "mt-1 text-xs text-gray-500",
                    "Leave empty to use default AWS credentials"
                }
            }

            button {
                onclick: connect,
                disabled: warehouse().is_empty() || matches!(connection_status(), ConnectionStatus::Connecting),
                class: format!(
                    "w-full flex justify-center py-2 px-4 border border-transparent rounded-md shadow-sm text-sm font-medium text-white {}",
                    if matches!(connection_status(), ConnectionStatus::Connecting) {
                        "bg-gray-400 cursor-not-allowed"
                    } else {
                        "bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500"
                    }
                ),
                if matches!(connection_status(), ConnectionStatus::Connecting) {
                    "Connecting..."
                } else {
                    "Connect to Glue Catalog"
                }
            }
        }
    }
}

#[component]
fn ConnectionStatusDisplay(status: ConnectionStatus) -> Element {
    match status {
        ConnectionStatus::Disconnected => rsx! { div {} },
        ConnectionStatus::Connecting => rsx! {
            div {
                class: "bg-blue-50 border border-blue-200 rounded-md p-4",
                div {
                    class: "flex",
                    div {
                        class: "flex-shrink-0",
                        div {
                            class: "animate-spin rounded-full h-5 w-5 border-b-2 border-blue-600"
                        }
                    }
                    div {
                        class: "ml-3",
                        p {
                            class: "text-sm text-blue-700",
                            "Connecting to catalog..."
                        }
                    }
                }
            }
        },
        ConnectionStatus::Connected => rsx! {
            div {
                class: "bg-green-50 border border-green-200 rounded-md p-4",
                div {
                    class: "flex",
                    div {
                        class: "flex-shrink-0",
                        svg {
                            class: "h-5 w-5 text-green-400",
                            fill: "currentColor",
                            view_box: "0 0 20 20",
                            path {
                                fill_rule: "evenodd",
                                d: "M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z",
                                clip_rule: "evenodd"
                            }
                        }
                    }
                    div {
                        class: "ml-3",
                        p {
                            class: "text-sm font-medium text-green-800",
                            "✅ Connected successfully!"
                        }
                        p {
                            class: "text-sm text-green-700",
                            "Select a namespace and table below to explore."
                        }
                    }
                }
            }
        },
        ConnectionStatus::Error(error) => rsx! {
            div {
                class: "bg-red-50 border border-red-200 rounded-md p-4",
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
                            "Connection failed"
                        }
                        p {
                            class: "text-sm text-red-700",
                            "{error}"
                        }
                    }
                }
            }
        },
    }
}

#[component]
fn TableBrowser(
    catalog_manager: Signal<CatalogManager>,
    namespaces: Vec<String>,
    selected_namespace: Signal<Option<String>>,
    tables: Signal<Vec<TableReference>>,
    on_table_selected: EventHandler<(String, String, String)>,
) -> Element {
    let mut loading_tables = use_signal(|| false);
    let load_tables = move |namespace: String| async move {
        loading_tables.set(true);
        // Get the first catalog connection (assuming single connection for now)
        if let Some(connection) = catalog_manager.read().get_connections().first() {
            log::info!(
                "Loading tables for namespace: {} from catalog: {}",
                namespace,
                connection.config.name
            );
            match catalog_manager
                .read()
                .list_tables(&connection.config.name, &namespace)
                .await
            {
                Ok(table_list) => {
                    log::info!(
                        "Successfully loaded {} tables for namespace: {}",
                        table_list.len(),
                        namespace
                    );
                    tables.set(table_list);
                    selected_namespace.set(Some(namespace));
                }
                Err(e) => {
                    log::error!("Failed to load tables for namespace {}: {}", namespace, e);
                    eprintln!("Failed to load tables for namespace {}: {}", namespace, e);
                    // Clear tables on error
                    tables.set(Vec::new());
                }
            }
        } else {
            log::error!("No catalog connection found");
            eprintln!("No catalog connection found");
        }
        loading_tables.set(false);
    };

    rsx! {
        div {
            class: "bg-white shadow rounded-lg",
            div {
                class: "px-4 py-5 sm:p-6",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                    "Browse Tables"
                }

                // Namespace Selection
                div {
                    class: "mb-6",
                    label {
                        class: "block text-sm font-medium text-gray-700 mb-2",
                        "Select Namespace"
                    }
                    div {
                        class: "grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3",
                        for namespace in namespaces.clone() {
                            button {
                                onclick: {
                                    let ns = namespace.clone();
                                    move |_| {
                                        log::info!("Namespace button clicked: {}", ns);
                                        spawn(load_tables(ns.clone()));
                                    }
                                },
                                disabled: loading_tables(),
                                class: format!(
                                    "text-left px-4 py-2 border rounded-md transition-colors {}",
                                    if loading_tables() {
                                        "border-gray-300 bg-gray-100 text-gray-400 cursor-not-allowed"
                                    } else if selected_namespace().as_ref() == Some(&namespace.clone()) {
                                        "border-blue-500 bg-blue-50 text-blue-700"
                                    } else {
                                        "border-gray-300 hover:border-gray-400 hover:bg-gray-50"
                                    }
                                ),
                                "📁 {namespace}"
                            }
                        }
                    }
                }

                // Table Selection
                if let Some(current_namespace) = selected_namespace() {
                    div {
                        h4 {
                            class: "text-md font-medium text-gray-900 mb-3",
                            "Tables in {current_namespace}"
                        }
                        if loading_tables() {
                            div {
                                class: "flex items-center justify-center py-8",
                                div {
                                    class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mr-3"
                                }
                                span {
                                    class: "text-sm text-gray-600",
                                    "Loading tables..."
                                }
                            }
                        } else if tables().is_empty() {
                            p {
                                class: "text-sm text-gray-500 italic",
                                "No tables found in this namespace"
                            }
                        } else {
                            div {
                                class: "grid grid-cols-1 gap-2",
                                for table in tables() {
                                    button {
                                        onclick: move |_| {
                                            if let Some(connection) = catalog_manager.read().get_connections().first() {
                                                on_table_selected.call((
                                                    connection.config.name.clone(),
                                                    table.namespace.clone(),
                                                    table.name.clone()
                                                ));
                                            }
                                        },
                                        class: "text-left px-4 py-3 border border-gray-300 rounded-md hover:border-blue-400 hover:bg-blue-50 transition-colors",
                                        div {
                                            class: "flex items-center justify-between",
                                            div {
                                                p {
                                                    class: "text-sm font-medium text-gray-900",
                                                    "🗂️ {table.name}"
                                                }
                                                p {
                                                    class: "text-xs text-gray-500",
                                                    "{table.full_name}"
                                                }
                                            }
                                            svg {
                                                class: "h-5 w-5 text-gray-400",
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
pub fn CatalogBrowser(
    catalog_manager: Signal<CatalogManager>,
    on_table_selected: EventHandler<(String, String, String)>,
) -> Element {
    let selected_namespace = use_signal(|| Option::<String>::None);
    let mut namespaces = use_signal(Vec::<String>::new);
    let tables = use_signal(Vec::<TableReference>::new);

    // Load namespaces when component mounts or catalog changes
    use_effect(move || {
        spawn(async move {
            if let Some(connection) = catalog_manager.read().get_connections().first() {
                match catalog_manager
                    .read()
                    .list_namespaces(&connection.config.name)
                    .await
                {
                    Ok(ns) => namespaces.set(ns),
                    Err(e) => log::error!("Failed to load namespaces: {}", e),
                }
            }
        });
    });

    rsx! {
        div {
            class: "space-y-6",

            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-5 sm:p-6",
                    h3 {
                        class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                        "Browse Catalog"
                    }
                    p {
                        class: "text-sm text-gray-500 mb-6",
                        "Select a namespace to explore tables in your Iceberg catalog."
                    }

                    TableBrowser {
                        catalog_manager: catalog_manager,
                        namespaces: namespaces(),
                        selected_namespace: selected_namespace,
                        tables: tables,
                        on_table_selected: on_table_selected
                    }
                }
            }
        }
    }
}
