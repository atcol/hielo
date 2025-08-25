use anyhow::Result;
use iceberg::table::Table;
use iceberg::{Catalog, NamespaceIdent, TableIdent};
use iceberg_catalog_glue::{GlueCatalog, GlueCatalogConfig};
use iceberg_catalog_rest::{RestCatalog, RestCatalogConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use url::Url;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableReference {
    pub namespace: String,
    pub name: String,
    pub full_name: String,
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
}

impl CatalogManager {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
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

        if let Some(region) = config.config.get("region") {
            props.insert("region".to_string(), region.clone());
        }

        if let Some(profile) = config.config.get("profile") {
            props.insert("profile".to_string(), profile.clone());
        }

        if let Some(endpoint) = config.config.get("endpoint_url") {
            props.insert("endpoint_url".to_string(), endpoint.clone());
        }

        let glue_config = GlueCatalogConfig::builder()
            .warehouse(warehouse.clone())
            .props(props)
            .build();

        let catalog = GlueCatalog::new(glue_config).await.map_err(|e| {
            CatalogError::ConnectionFailed(format!("Failed to create Glue catalog: {}", e))
        })?;

        Ok(Arc::new(catalog))
    }

    pub async fn list_namespaces(&self, catalog_name: &str) -> Result<Vec<String>, CatalogError> {
        let connection = self
            .connections
            .iter()
            .find(|conn| conn.config.name == catalog_name)
            .ok_or_else(|| {
                CatalogError::ConnectionFailed(format!("Catalog '{}' not found", catalog_name))
            })?;

        let namespaces = connection
            .catalog
            .list_namespaces(None)
            .await
            .map_err(|e| CatalogError::NetworkError(format!("Failed to list namespaces: {}", e)))?;

        Ok(namespaces.into_iter().map(|ns| ns.to_string()).collect())
    }

    pub async fn list_tables(
        &self,
        catalog_name: &str,
        namespace: &str,
    ) -> Result<Vec<TableReference>, CatalogError> {
        let connection = self
            .connections
            .iter()
            .find(|conn| conn.config.name == catalog_name)
            .ok_or_else(|| {
                CatalogError::ConnectionFailed(format!("Catalog '{}' not found", catalog_name))
            })?;

        let namespace_ident = NamespaceIdent::from_vec(vec![namespace.to_string()])
            .map_err(|e| CatalogError::InvalidConfig(format!("Invalid namespace: {}", e)))?;

        let table_idents = connection
            .catalog
            .list_tables(&namespace_ident)
            .await
            .map_err(|e| CatalogError::NetworkError(format!("Failed to list tables: {}", e)))?;

        Ok(table_idents
            .into_iter()
            .map(|ident| TableReference {
                namespace: namespace.to_string(),
                name: ident.name().to_string(),
                full_name: format!("{}.{}", namespace, ident.name()),
            })
            .collect())
    }

    pub async fn load_table(
        &self,
        catalog_name: &str,
        namespace: &str,
        table_name: &str,
    ) -> Result<Table, CatalogError> {
        let connection = self
            .connections
            .iter()
            .find(|conn| conn.config.name == catalog_name)
            .ok_or_else(|| {
                CatalogError::ConnectionFailed(format!("Catalog '{}' not found", catalog_name))
            })?;

        let table_ident = TableIdent::from_strs(vec![namespace, table_name])
            .map_err(|e| CatalogError::InvalidConfig(format!("Invalid table identifier: {}", e)))?;

        let table = connection
            .catalog
            .load_table(&table_ident)
            .await
            .map_err(|e| CatalogError::TableNotFound(format!("Failed to load table: {}", e)))?;

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
