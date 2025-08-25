use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::catalog::CatalogConfig;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub catalogs: Vec<CatalogConfig>,
}

impl AppConfig {
    /// Get the path to the config file in the user's home directory
    pub fn config_path() -> Result<PathBuf> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

        let config_dir = home_dir.join(".hielo");

        // Create config directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir.join("config.json"))
    }

    /// Load configuration from file, creating default if file doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let config: AppConfig = serde_json::from_str(&contents)
                .map_err(|e| anyhow::anyhow!("Failed to parse config file: {}", e))?;
            Ok(config)
        } else {
            // Create default config
            let default_config = AppConfig::default();
            default_config.save()?; // Save the default config
            Ok(default_config)
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, contents)?;
        Ok(())
    }

    /// Add a catalog configuration, ensuring unique names
    pub fn add_catalog(&mut self, catalog: CatalogConfig) -> Result<()> {
        // Check if name already exists
        if self.catalogs.iter().any(|c| c.name == catalog.name) {
            return Err(anyhow::anyhow!(
                "Catalog name '{}' already exists",
                catalog.name
            ));
        }

        self.catalogs.push(catalog);
        self.save()?;
        Ok(())
    }

    /// Update an existing catalog configuration
    pub fn update_catalog(&mut self, catalog: CatalogConfig) -> Result<()> {
        if let Some(existing) = self.catalogs.iter_mut().find(|c| c.name == catalog.name) {
            *existing = catalog;
            self.save()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Catalog '{}' not found", catalog.name))
        }
    }

    /// Remove a catalog configuration
    pub fn remove_catalog(&mut self, name: &str) -> Result<()> {
        let initial_len = self.catalogs.len();
        self.catalogs.retain(|c| c.name != name);

        if self.catalogs.len() < initial_len {
            self.save()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Catalog '{}' not found", name))
        }
    }

    /// Get a catalog by name
    pub fn get_catalog(&self, name: &str) -> Option<&CatalogConfig> {
        self.catalogs.iter().find(|c| c.name == name)
    }

    /// Check if a catalog name is unique
    pub fn is_name_unique(&self, name: &str) -> bool {
        !self.catalogs.iter().any(|c| c.name == name)
    }
}

/// Sanitize credentials in catalog config for display purposes
pub fn sanitize_config_for_display(config: &CatalogConfig) -> CatalogConfig {
    let mut display_config = config.clone();

    // Sanitize sensitive fields
    if let Some(token) = display_config.config.get_mut("auth_token") {
        if !token.is_empty() {
            *token = "***HIDDEN***".to_string();
        }
    }

    if let Some(profile) = display_config.config.get_mut("profile") {
        if !profile.is_empty() {
            // Don't hide profile as it's not sensitive, just identifying
        }
    }

    display_config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::CatalogType;
    use std::collections::HashMap;
    

    fn create_test_catalog() -> CatalogConfig {
        let mut config = HashMap::new();
        config.insert("uri".to_string(), "http://localhost:8181".to_string());

        CatalogConfig {
            catalog_type: CatalogType::Rest,
            name: "test-catalog".to_string(),
            config,
        }
    }

    #[test]
    fn test_add_catalog() {
        let mut app_config = AppConfig::default();
        let catalog = create_test_catalog();

        assert!(app_config.add_catalog(catalog.clone()).is_ok());
        assert_eq!(app_config.catalogs.len(), 1);

        // Test duplicate name
        assert!(app_config.add_catalog(catalog).is_err());
        assert_eq!(app_config.catalogs.len(), 1);
    }

    #[test]
    fn test_name_uniqueness() {
        let mut app_config = AppConfig::default();
        let catalog = create_test_catalog();

        assert!(app_config.is_name_unique("test-catalog"));
        assert!(app_config.add_catalog(catalog).is_ok());
        assert!(!app_config.is_name_unique("test-catalog"));
        assert!(app_config.is_name_unique("other-catalog"));
    }

    #[test]
    fn test_sanitize_config() {
        let mut config = HashMap::new();
        config.insert("uri".to_string(), "http://localhost:8181".to_string());
        config.insert("auth_token".to_string(), "secret-token".to_string());

        let catalog_config = CatalogConfig {
            catalog_type: CatalogType::Rest,
            name: "test".to_string(),
            config,
        };

        let sanitized = sanitize_config_for_display(&catalog_config);
        assert_eq!(sanitized.config.get("auth_token").unwrap(), "***HIDDEN***");
        assert_eq!(
            sanitized.config.get("uri").unwrap(),
            "http://localhost:8181"
        );
    }
}
