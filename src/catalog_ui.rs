use crate::catalog::{CatalogConfig, CatalogManager, CatalogType, TableReference, TableType};
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

#[derive(Debug, Clone, PartialEq)]
pub enum NavigationView {
    Namespaces,
    Tables { namespace: String },
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

                // Show saved catalogs if any exist
                if !catalog_manager.read().get_saved_catalogs().is_empty() {
                    SavedCatalogsSection {
                        catalog_manager: catalog_manager,
                        connection_status: connection_status,
                        namespaces: namespaces,
                        on_catalog_connected: on_catalog_connected,
                    }
                }

                // Connection Form
                div {
                    class: "bg-white shadow rounded-lg",
                    div {
                        class: "px-4 py-5 sm:p-6",
                        h3 {
                            class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                            if !catalog_manager.read().get_saved_catalogs().is_empty() {
                                "Add New Catalog"
                            } else {
                                "Connect to Iceberg Catalog"
                            }
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
                        loading_namespaces: false, // Connection screen doesn't show namespace loading
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

        // Check if name is unique
        if !catalog_manager
            .read()
            .is_catalog_name_unique(&catalog_name())
        {
            connection_status.set(ConnectionStatus::Error(format!(
                "Catalog name '{}' already exists. Please choose a different name.",
                catalog_name()
            )));
            return;
        }

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

        // Check if name is unique
        if !catalog_manager
            .read()
            .is_catalog_name_unique(&catalog_name())
        {
            connection_status.set(ConnectionStatus::Error(format!(
                "Catalog name '{}' already exists. Please choose a different name.",
                catalog_name()
            )));
            return;
        }

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
    loading_namespaces: bool,
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
                    if loading_namespaces {
                        div {
                            class: "flex items-center justify-center py-8",
                            div {
                                class: "animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600 mr-3"
                            }
                            span {
                                class: "text-sm text-gray-600",
                                "Loading namespaces..."
                            }
                        }
                    } else if namespaces.is_empty() {
                        div {
                            class: "text-center py-8",
                            p {
                                class: "text-sm text-gray-500 italic",
                                "No namespaces found in this catalog"
                            }
                        }
                    } else {
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
                                            // Only allow clicking on Iceberg tables
                                            if table.table_type == TableType::Iceberg {
                                                if let Some(connection) = catalog_manager.read().get_connections().first() {
                                                    on_table_selected.call((
                                                        connection.config.name.clone(),
                                                        table.namespace.clone(),
                                                        table.name.clone()
                                                    ));
                                                }
                                            }
                                        },
                                        disabled: table.table_type != TableType::Iceberg,
                                        class: format!(
                                            "text-left px-4 py-3 border rounded-md transition-colors {}",
                                            match table.table_type {
                                                TableType::Iceberg => "border-gray-300 hover:border-blue-400 hover:bg-blue-50 cursor-pointer",
                                                TableType::Unknown => "border-gray-200 bg-gray-50 cursor-not-allowed opacity-60"
                                            }
                                        ),
                                        div {
                                            class: "flex items-center justify-between",
                                            div {
                                                p {
                                                    class: format!(
                                                        "text-sm font-medium {}",
                                                        match table.table_type {
                                                            TableType::Iceberg => "text-gray-900",
                                                            TableType::Unknown => "text-gray-500"
                                                        }
                                                    ),
                                                    match table.table_type {
                                                        TableType::Iceberg => format!("🧊 {}", table.name),
                                                        TableType::Unknown => format!("📄 {}", table.name)
                                                    }
                                                }
                                                p {
                                                    class: "text-xs text-gray-500",
                                                    "{table.full_name}"
                                                }
                                                if table.table_type != TableType::Iceberg {
                                                    p {
                                                        class: "text-xs text-gray-400 italic mt-1",
                                                        "Not an Iceberg table"
                                                    }
                                                }
                                            }
                                            if table.table_type == TableType::Iceberg {
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
}

#[component]
fn SavedCatalogsSection(
    catalog_manager: Signal<CatalogManager>,
    connection_status: Signal<ConnectionStatus>,
    namespaces: Signal<Vec<String>>,
    on_catalog_connected: EventHandler<()>,
) -> Element {
    let connect_to_saved_catalog = move |catalog_config: crate::catalog::CatalogConfig| async move {
        connection_status.set(ConnectionStatus::Connecting);

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
                    .list_namespaces(&catalog_config.name)
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
            class: "bg-white shadow rounded-lg",
            div {
                class: "px-4 py-5 sm:p-6",
                h3 {
                    class: "text-lg leading-6 font-medium text-gray-900 mb-4",
                    "Saved Catalogs"
                }
                p {
                    class: "text-sm text-gray-500 mb-4",
                    "Connect to a previously saved catalog or add a new one below."
                }

                div {
                    class: "grid grid-cols-1 gap-3 sm:grid-cols-2",
                    for catalog_config in catalog_manager.read().get_saved_catalogs() {
                        div {
                            class: "border border-gray-200 rounded-lg p-4 hover:border-blue-300 hover:bg-blue-50 transition-colors",
                            div {
                                class: "flex items-center justify-between mb-2",
                                h4 {
                                    class: "text-sm font-medium text-gray-900",
                                    "{catalog_config.name}"
                                }
                                span {
                                    class: match catalog_config.catalog_type {
                                        CatalogType::Rest => "inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-blue-100 text-blue-800",
                                        CatalogType::Glue => "inline-flex items-center px-2 py-1 rounded text-xs font-medium bg-orange-100 text-orange-800",
                                    },
                                    match catalog_config.catalog_type {
                                        CatalogType::Rest => "REST",
                                        CatalogType::Glue => "Glue",
                                    }
                                }
                            }
                            p {
                                class: "text-xs text-gray-500 mb-3",
                                match catalog_config.catalog_type {
                                    CatalogType::Rest => format!("URI: {}", catalog_config.config.get("uri").unwrap_or(&"N/A".to_string())),
                                    CatalogType::Glue => format!("Warehouse: {}", catalog_config.config.get("warehouse").unwrap_or(&"N/A".to_string())),
                                }
                            }
                            button {
                                onclick: {
                                    let config = catalog_config.clone();
                                    move |_| {
                                        spawn(connect_to_saved_catalog(config.clone()));
                                    }
                                },
                                disabled: matches!(connection_status(), ConnectionStatus::Connecting),
                                class: format!(
                                    "w-full flex justify-center py-2 px-3 border border-transparent rounded-md shadow-sm text-sm font-medium text-white {}",
                                    if matches!(connection_status(), ConnectionStatus::Connecting) {
                                        "bg-gray-400 cursor-not-allowed"
                                    } else {
                                        "bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"
                                    }
                                ),
                                if matches!(connection_status(), ConnectionStatus::Connecting) {
                                    "Connecting..."
                                } else {
                                    "Connect"
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
    let mut current_view = use_signal(|| NavigationView::Namespaces);
    let mut namespaces = use_signal(Vec::<String>::new);
    let mut tables = use_signal(Vec::<TableReference>::new);
    let mut loading = use_signal(|| true);

    // Load namespaces when component mounts or catalog changes
    use_effect(move || {
        spawn(async move {
            loading.set(true);
            if let Some(connection) = catalog_manager.read().get_connections().first() {
                match catalog_manager
                    .read()
                    .list_namespaces(&connection.config.name)
                    .await
                {
                    Ok(ns) => {
                        namespaces.set(ns);
                        loading.set(false);
                    }
                    Err(e) => {
                        log::error!("Failed to load namespaces: {}", e);
                        loading.set(false);
                    }
                }
            } else {
                loading.set(false);
            }
        });
    });

    // Function to navigate to a namespace
    let navigate_to_namespace = move |namespace: String| {
        current_view.set(NavigationView::Tables { namespace: namespace.clone() });
        spawn(async move {
            loading.set(true);
            if let Some(connection) = catalog_manager.read().get_connections().first() {
                match catalog_manager
                    .read()
                    .list_tables(&connection.config.name, &namespace)
                    .await
                {
                    Ok(table_list) => {
                        tables.set(table_list);
                        loading.set(false);
                    }
                    Err(e) => {
                        log::error!("Failed to load tables for namespace {}: {}", namespace, e);
                        loading.set(false);
                    }
                }
            } else {
                loading.set(false);
            }
        });
    };

    // Function to navigate back to namespaces
    let navigate_back = move |_| {
        current_view.set(NavigationView::Namespaces);
        tables.set(Vec::new());
    };

    rsx! {
        div {
            class: "space-y-4",
            
            // File Explorer Header
            div {
                class: "bg-white shadow rounded-lg",
                div {
                    class: "px-4 py-3 border-b border-gray-200",
                    FileBrowserHeader {
                        current_view: current_view(),
                        catalog_manager: catalog_manager,
                        on_navigate_back: navigate_back
                    }
                }
                div {
                    class: "px-4 py-4",
                    if loading() {
                        LoadingView {}
                    } else {
                        match current_view() {
                            NavigationView::Namespaces => rsx! {
                                NamespaceExplorerView {
                                    namespaces: namespaces(),
                                    on_namespace_selected: navigate_to_namespace
                                }
                            },
                            NavigationView::Tables { namespace } => rsx! {
                                TableExplorerView {
                                    namespace: namespace,
                                    tables: tables(),
                                    catalog_manager: catalog_manager,
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
fn FileBrowserHeader(
    current_view: NavigationView,
    catalog_manager: Signal<CatalogManager>,
    on_navigate_back: EventHandler<()>,
) -> Element {
    let catalog_name = if let Some(connection) = catalog_manager.read().get_connections().first() {
        connection.config.name.clone()
    } else {
        "Unknown Catalog".to_string()
    };

    rsx! {
        div {
            class: "flex items-center space-x-3",
            
            // Back button (only show when viewing tables)
            if let NavigationView::Tables { .. } = current_view {
                button {
                    onclick: move |_| on_navigate_back.call(()),
                    class: "flex items-center text-gray-600 hover:text-gray-900 transition-colors",
                    svg {
                        class: "h-5 w-5 mr-1",
                        fill: "none",
                        stroke: "currentColor",
                        view_box: "0 0 24 24",
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            stroke_width: "2",
                            d: "M15 19l-7-7 7-7"
                        }
                    }
                    "Back"
                }
            }

            // Breadcrumb navigation
            nav {
                class: "flex items-center space-x-2 text-sm",
                div {
                    class: "flex items-center space-x-2 text-gray-600",
                    span {
                        class: "font-medium",
                        "🗂️ {catalog_name}"
                    }
                    
                    match current_view {
                        NavigationView::Namespaces => rsx! {
                            span { class: "text-gray-400", " > " }
                            span { class: "text-gray-900 font-medium", "Namespaces" }
                        },
                        NavigationView::Tables { namespace } => rsx! {
                            span { class: "text-gray-400", " > " }
                            button {
                                onclick: move |_| on_navigate_back.call(()),
                                class: "text-blue-600 hover:text-blue-800 underline",
                                "Namespaces"
                            }
                            span { class: "text-gray-400", " > " }
                            span { class: "text-gray-900 font-medium", "📁 {namespace}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LoadingView() -> Element {
    rsx! {
        div {
            class: "flex items-center justify-center py-12",
            div {
                class: "text-center",
                div {
                    class: "animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"
                }
                p {
                    class: "text-sm text-gray-600",
                    "Loading..."
                }
            }
        }
    }
}

#[component]
fn NamespaceExplorerView(
    namespaces: Vec<String>,
    on_namespace_selected: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: "space-y-3",
            
            div {
                class: "flex items-center mb-4",
                h3 {
                    class: "text-lg font-medium text-gray-900",
                    "Namespaces"
                }
                span {
                    class: "ml-2 text-sm text-gray-500",
                    "({namespaces.len()} items)"
                }
            }

            if namespaces.is_empty() {
                div {
                    class: "text-center py-12",
                    div {
                        class: "text-gray-400 mb-2",
                        "📂"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "No namespaces found in this catalog"
                    }
                }
            } else {
                div {
                    class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3",
                    for namespace in namespaces {
                        div {
                            class: "group relative",
                            button {
                                onclick: {
                                    let ns = namespace.clone();
                                    move |_| on_namespace_selected.call(ns.clone())
                                },
                                class: "w-full p-4 text-left border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                                div {
                                    class: "flex flex-col items-center space-y-2",
                                    div {
                                        class: "text-3xl text-blue-500 group-hover:text-blue-600",
                                        "📁"
                                    }
                                    div {
                                        class: "text-center",
                                        p {
                                            class: "text-sm font-medium text-gray-900 truncate",
                                            title: "{namespace}",
                                            "{namespace}"
                                        }
                                        p {
                                            class: "text-xs text-gray-500 mt-1",
                                            "Namespace"
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
fn TableExplorerView(
    namespace: String,
    tables: Vec<TableReference>,
    catalog_manager: Signal<CatalogManager>,
    on_table_selected: EventHandler<(String, String, String)>,
) -> Element {
    let iceberg_tables: Vec<_> = tables.iter().filter(|t| t.table_type == TableType::Iceberg).collect();
    let other_tables: Vec<_> = tables.iter().filter(|t| t.table_type != TableType::Iceberg).collect();

    rsx! {
        div {
            class: "space-y-4",
            
            div {
                class: "flex items-center mb-4",
                h3 {
                    class: "text-lg font-medium text-gray-900",
                    "Tables in {namespace}"
                }
                span {
                    class: "ml-2 text-sm text-gray-500",
                    "({tables.len()} items)"
                }
            }

            if tables.is_empty() {
                div {
                    class: "text-center py-12",
                    div {
                        class: "text-gray-400 mb-2",
                        "📄"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "No tables found in this namespace"
                    }
                }
            } else {
                div {
                    class: "space-y-6",
                    
                    // Iceberg Tables Section
                    if !iceberg_tables.is_empty() {
                        div {
                            h4 {
                                class: "text-md font-medium text-gray-900 mb-3 flex items-center",
                                "🧊 Iceberg Tables"
                                span {
                                    class: "ml-2 text-sm text-gray-500 font-normal",
                                    "({iceberg_tables.len()})"
                                }
                            }
                            div {
                                class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3",
                                for table in iceberg_tables {
                                    div {
                                        class: "group relative",
                                        button {
                                            onclick: {
                                                let table_clone = table.clone();
                                                move |_| {
                                                    if let Some(connection) = catalog_manager.read().get_connections().first() {
                                                        on_table_selected.call((
                                                            connection.config.name.clone(),
                                                            table_clone.namespace.clone(),
                                                            table_clone.name.clone()
                                                        ));
                                                    }
                                                }
                                            },
                                            class: "w-full p-4 text-left border border-gray-200 rounded-lg hover:border-blue-300 hover:bg-blue-50 transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent",
                                            div {
                                                class: "flex flex-col items-center space-y-2",
                                                div {
                                                    class: "text-3xl text-blue-500 group-hover:text-blue-600",
                                                    "🧊"
                                                }
                                                div {
                                                    class: "text-center",
                                                    p {
                                                        class: "text-sm font-medium text-gray-900 truncate",
                                                        title: "{table.name}",
                                                        "{table.name}"
                                                    }
                                                    p {
                                                        class: "text-xs text-gray-500 mt-1",
                                                        "Iceberg Table"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Other Tables Section
                    if !other_tables.is_empty() {
                        div {
                            h4 {
                                class: "text-md font-medium text-gray-500 mb-3 flex items-center",
                                "📄 Other Tables"
                                span {
                                    class: "ml-2 text-sm text-gray-400 font-normal",
                                    "({other_tables.len()})"
                                }
                            }
                            div {
                                class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3",
                                for table in other_tables {
                                    div {
                                        class: "group relative opacity-60",
                                        div {
                                            class: "w-full p-4 text-left border border-gray-200 rounded-lg bg-gray-50 cursor-not-allowed",
                                            div {
                                                class: "flex flex-col items-center space-y-2",
                                                div {
                                                    class: "text-3xl text-gray-400",
                                                    "📄"
                                                }
                                                div {
                                                    class: "text-center",
                                                    p {
                                                        class: "text-sm font-medium text-gray-500 truncate",
                                                        title: "{table.name}",
                                                        "{table.name}"
                                                    }
                                                    p {
                                                        class: "text-xs text-gray-400 mt-1",
                                                        "Not accessible"
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