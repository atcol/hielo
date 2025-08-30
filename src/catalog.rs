use anyhow::Result;
use iceberg::table::Table;
use iceberg::{Catalog, NamespaceIdent, TableIdent};
use iceberg_catalog_glue::{GlueCatalog, GlueCatalogConfig};
use iceberg_catalog_rest::{RestCatalog, RestCatalogConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

use crate::config::AppConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CatalogType {
    Rest,
    Glue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogConfig {
    pub catalog_type: CatalogType,
    pub name: String,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct CatalogConnection {
    pub config: CatalogConfig,
    pub catalog: Arc<dyn Catalog>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TableType {
    Iceberg,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TableReference {
    pub namespace: String,
    pub name: String,
    pub full_name: String,
    pub table_type: TableType,
}

#[derive(Debug, Clone)]
pub enum CatalogError {
    ConnectionFailed(String),
    InvalidConfig(String),
    TableNotFound(String),
    NamespaceNotFound(String),
    AuthenticationFailed(String),
    NetworkError(String),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CatalogError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            CatalogError::InvalidConfig(msg) => write!(f, "Invalid configuration: {}", msg),
            CatalogError::TableNotFound(msg) => write!(f, "Table not found: {}", msg),
            CatalogError::NamespaceNotFound(msg) => write!(f, "Namespace not found: {}", msg),
            CatalogError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            CatalogError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for CatalogError {}

impl CatalogConfig {
    pub fn new_rest(name: String, uri: String) -> Self {
        let mut config = HashMap::new();
        config.insert("uri".to_string(), uri);

        Self {
            catalog_type: CatalogType::Rest,
            name,
            config,
        }
    }

    pub fn new_glue(name: String, warehouse: String, region: Option<String>) -> Self {
        let mut config = HashMap::new();
        config.insert("warehouse".to_string(), warehouse);
        if let Some(region) = region {
            config.insert("region".to_string(), region);
        }

        Self {
            catalog_type: CatalogType::Glue,
            name,
            config,
        }
    }
}

pub struct CatalogManager {
    connections: Vec<CatalogConnection>,
    config: AppConfig,
}

impl CatalogManager {
    pub fn new() -> Self {
        let config = AppConfig::load().unwrap_or_default();
        Self {
            connections: Vec::new(),
            config,
        }
    }

    /// Get saved catalog configurations
    pub fn get_saved_catalogs(&self) -> &[CatalogConfig] {
        &self.config.catalogs
    }

    /// Check if a catalog name is unique
    pub fn is_catalog_name_unique(&self, name: &str) -> bool {
        self.config.is_name_unique(name)
    }

    pub async fn connect_catalog(&mut self, config: CatalogConfig) -> Result<(), CatalogError> {
        let catalog: Arc<dyn Catalog> = match config.catalog_type {
            CatalogType::Rest => self.create_rest_catalog(&config).await?,
            CatalogType::Glue => self.create_glue_catalog(&config).await?,
        };

        let connection = CatalogConnection {
            config: config.clone(),
            catalog,
        };

        // Remove existing connection with same name
        self.connections
            .retain(|conn| conn.config.name != config.name);
        self.connections.push(connection);

        // Save catalog configuration to persistent config
        if let Err(e) = self.config.add_catalog(config) {
            // If it's a duplicate name error, update instead of add
            if e.to_string().contains("already exists") {
                // For now, we'll just log this - in practice, the UI should prevent duplicates
                log::warn!("Catalog name already exists in config: {}", e);
            } else {
                log::error!("Failed to save catalog configuration: {}", e);
            }
        } else {
            log::info!("Catalog configuration saved successfully");
        }

        Ok(())
    }

    async fn create_rest_catalog(
        &self,
        config: &CatalogConfig,
    ) -> Result<Arc<dyn Catalog>, CatalogError> {
        let uri = config.config.get("uri").ok_or_else(|| {
            CatalogError::InvalidConfig("URI is required for REST catalog".to_string())
        })?;

        let url = Url::parse(uri)
            .map_err(|e| CatalogError::InvalidConfig(format!("Invalid URI: {}", e)))?;

        let mut props = HashMap::new();
        props.insert("uri".to_string(), uri.clone());

        // Add optional warehouse
        if let Some(warehouse) = config.config.get("warehouse") {
            props.insert("warehouse".to_string(), warehouse.clone());
        }

        // Add optional auth token
        if let Some(token) = config.config.get("auth_token") {
            props.insert("token".to_string(), token.clone());
        }

        let rest_config = RestCatalogConfig::builder()
            .uri(uri.clone())
            .props(props)
            .build();

        let catalog = RestCatalog::new(rest_config);

        Ok(Arc::new(catalog))
    }

    async fn create_glue_catalog(
        &self,
        config: &CatalogConfig,
    ) -> Result<Arc<dyn Catalog>, CatalogError> {
        let warehouse = config.config.get("warehouse").ok_or_else(|| {
            CatalogError::InvalidConfig("Warehouse is required for Glue catalog".to_string())
        })?;

        let mut props = HashMap::new();
        props.insert("warehouse".to_string(), warehouse.clone());

        // Region is required for Glue catalog - ensure it's always present
        let region = config
            .config
            .get("region")
            .cloned()
            .unwrap_or_else(|| "us-east-1".to_string());
        props.insert("region".to_string(), region);

        if let Some(profile) = config.config.get("profile") {
            props.insert("profile".to_string(), profile.clone());
        }

        if let Some(endpoint) = config.config.get("endpoint_url") {
            props.insert("endpoint_url".to_string(), endpoint.clone());
        }

        let glue_config = GlueCatalogConfig::builder()
            .warehouse(warehouse.clone())
            .props(props.clone())
            .build();

        log::info!(
            "Creating Glue catalog with config - warehouse: '{}', region: '{}', props: {:?}",
            warehouse,
            props.get("region").unwrap_or(&"N/A".to_string()),
            props.keys().collect::<Vec<_>>()
        );

        // Ensure region is available for AWS SDK - try multiple methods
        if let Some(region) = props.get("region") {
            // Set environment variables as fallback
            unsafe {
                std::env::set_var("AWS_DEFAULT_REGION", region);
                std::env::set_var("AWS_REGION", region);
            }
            log::info!("Set AWS region environment variables to: {}", region);
        } else {
            log::warn!("No region found in Glue catalog configuration!");
        }

        let catalog = GlueCatalog::new(glue_config).await.map_err(|e| {
            let error = format!("Failed to create Glue catalog: {}", e);
            log::error!("{}", error);
            CatalogError::ConnectionFailed(error)
        })?;

        Ok(Arc::new(catalog))
    }

    pub async fn list_namespaces(&self, catalog_name: &str) -> Result<Vec<String>, CatalogError> {
        log::info!("Listing namespaces for catalog: '{}'", catalog_name);

        let connection = self
            .connections
            .iter()
            .find(|conn| conn.config.name == catalog_name)
            .ok_or_else(|| {
                let error = format!("Catalog '{}' not found", catalog_name);
                log::error!("{}", error);
                CatalogError::ConnectionFailed(error)
            })?;

        log::info!(
            "Found catalog connection, catalog type: {:?}",
            connection.config.catalog_type
        );

        let namespaces = connection
            .catalog
            .list_namespaces(None)
            .await
            .map_err(|e| {
                let error = format!("Failed to list namespaces: {}", e);
                log::error!("{}", error);
                CatalogError::NetworkError(error)
            })?;

        let namespace_strings: Vec<String> = namespaces
            .into_iter()
            .map(|ns| {
                let ns_string = ns.to_string();
                log::info!("Found namespace: '{}'", ns_string);
                ns_string
            })
            .collect();

        log::info!("Returning {} namespaces", namespace_strings.len());
        Ok(namespace_strings)
    }

    pub async fn list_tables(
        &self,
        catalog_name: &str,
        namespace: &str,
    ) -> Result<Vec<TableReference>, CatalogError> {
        log::info!(
            "Listing tables for catalog: '{}', namespace: '{}'",
            catalog_name,
            namespace
        );

        let connection = self
            .connections
            .iter()
            .find(|conn| conn.config.name == catalog_name)
            .ok_or_else(|| {
                let error = format!("Catalog '{}' not found", catalog_name);
                log::error!("{}", error);
                CatalogError::ConnectionFailed(error)
            })?;

        log::info!(
            "Found catalog connection, catalog type: {:?}",
            connection.config.catalog_type
        );

        let namespace_ident =
            NamespaceIdent::from_vec(vec![namespace.to_string()]).map_err(|e| {
                let error = format!("Invalid namespace '{}': {}", namespace, e);
                log::error!("{}", error);
                CatalogError::InvalidConfig(error)
            })?;

        log::info!("Created namespace identifier: {:?}", namespace_ident);

        let table_idents = connection
            .catalog
            .list_tables(&namespace_ident)
            .await
            .map_err(|e| {
                let error = format!("Failed to list tables in namespace '{}': {}", namespace, e);
                log::error!("{}", error);
                CatalogError::NetworkError(error)
            })?;

        log::info!(
            "Found {} table identifiers in namespace '{}'",
            table_idents.len(),
            namespace
        );

        let mut table_refs: Vec<TableReference> = Vec::new();

        for ident in table_idents {
            let table_name = ident.name().to_string();
            let full_name = format!("{}.{}", namespace, table_name);

            // Try to load the table to determine if it's an Iceberg table
            let table_type = match connection.catalog.load_table(&ident).await {
                Ok(_) => {
                    log::info!("✅ Iceberg table detected: {}", full_name);
                    TableType::Iceberg
                }
                Err(e) => {
                    log::info!(
                        "❓ Non-Iceberg or inaccessible table: {} ({})",
                        full_name,
                        e
                    );
                    TableType::Unknown
                }
            };

            let table_ref = TableReference {
                namespace: namespace.to_string(),
                name: table_name,
                full_name: full_name.clone(),
                table_type,
            };

            log::info!(
                "Table: {} [Type: {:?}]",
                table_ref.full_name,
                table_ref.table_type
            );
            table_refs.push(table_ref);
        }

        log::info!("Returning {} table references", table_refs.len());
        Ok(table_refs)
    }

    pub async fn load_table(
        &self,
        catalog_name: &str,
        namespace: &str,
        table_name: &str,
    ) -> Result<Table, CatalogError> {
        log::info!(
            "Loading table: catalog='{}', namespace='{}', table='{}'",
            catalog_name,
            namespace,
            table_name
        );

        let connection = self
            .connections
            .iter()
            .find(|conn| conn.config.name == catalog_name)
            .ok_or_else(|| {
                let error = format!("Catalog '{}' not found", catalog_name);
                log::error!("{}", error);
                CatalogError::ConnectionFailed(error)
            })?;

        log::info!(
            "Found catalog connection, type: {:?}, config: {:?}",
            connection.config.catalog_type,
            connection.config.config.keys().collect::<Vec<_>>()
        );

        let table_ident = TableIdent::from_strs(vec![namespace, table_name]).map_err(|e| {
            let error = format!("Invalid table identifier: {}", e);
            log::error!("{}", error);
            CatalogError::InvalidConfig(error)
        })?;

        log::info!("Table identifier created: {:?}", table_ident);

        let table = connection
            .catalog
            .load_table(&table_ident)
            .await
            .map_err(|e| {
                let error = format!("Failed to load table '{}': {}", table_ident, e);
                log::error!("{}", error);
                CatalogError::TableNotFound(error)
            })?;

        log::info!("Table loaded successfully: {}", table_ident);
        Ok(table)
    }

    pub fn get_connections(&self) -> &[CatalogConnection] {
        &self.connections
    }

    pub fn remove_connection(&mut self, catalog_name: &str) -> bool {
        let initial_len = self.connections.len();
        self.connections
            .retain(|conn| conn.config.name != catalog_name);
        self.connections.len() < initial_len
    }

    /// Delete a catalog - removes both the connection and the saved configuration
    pub fn delete_catalog(&mut self, catalog_name: &str) -> Result<(), CatalogError> {
        // Remove from active connections
        self.remove_connection(catalog_name);

        // Remove from saved configuration
        if let Err(e) = self.config.remove_catalog(catalog_name) {
            log::error!("Failed to remove catalog from config: {}", e);
            return Err(CatalogError::InvalidConfig(format!(
                "Failed to remove catalog from config: {}",
                e
            )));
        }

        log::info!("Successfully deleted catalog: {}", catalog_name);
        Ok(())
    }
}

// Test connection function
pub async fn test_catalog_connection(config: &CatalogConfig) -> Result<String, CatalogError> {
    let mut manager = CatalogManager::new();
    manager.connect_catalog(config.clone()).await?;

    // Try to list namespaces as a connection test
    let namespaces = manager.list_namespaces(&config.name).await?;

    Ok(format!(
        "Connection successful! Found {} namespace(s)",
        namespaces.len()
    ))
}
