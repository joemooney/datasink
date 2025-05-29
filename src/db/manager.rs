use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use super::{Database, SqliteDatabase, DatabaseError};

#[derive(Debug, Clone)]
pub struct DatabaseInfo {
    pub name: String,
    pub url: String,
    pub connected: bool,
    pub connection_time: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct DatabaseManager {
    databases: Arc<RwLock<HashMap<String, DatabaseConnection>>>,
}

struct DatabaseConnection {
    info: DatabaseInfo,
    db: Arc<RwLock<Box<dyn Database>>>,
    _handle: JoinHandle<()>,
}

impl DatabaseManager {
    pub fn new() -> Self {
        Self {
            databases: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add or connect to a database
    pub async fn add_database(&self, name: String, url: String) -> Result<(), DatabaseError> {
        let mut databases = self.databases.write().await;
        
        // Don't add if already exists
        if databases.contains_key(&name) {
            return Ok(());
        }

        // Create database connection
        let db = SqliteDatabase::connect(&url).await?;
        let db_arc = Arc::new(RwLock::new(Box::new(db) as Box<dyn Database>));
        
        // Create a background task for the database connection
        let _db_clone = db_arc.clone();
        let handle = tokio::spawn(async move {
            // Keep connection alive and handle any background tasks
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                // Could add health checks or cleanup here
            }
        });

        let info = DatabaseInfo {
            name: name.clone(),
            url: url.clone(),
            connected: true,
            connection_time: Some(chrono::Utc::now()),
        };

        let connection = DatabaseConnection {
            info,
            db: db_arc,
            _handle: handle,
        };

        databases.insert(name, connection);
        Ok(())
    }

    /// Get a database connection by name
    pub async fn get_database(&self, name: &str) -> Option<Arc<RwLock<Box<dyn Database>>>> {
        let databases = self.databases.read().await;
        databases.get(name).map(|conn| conn.db.clone())
    }
    
    /// Get a database by name or return the default if name is None/empty
    pub async fn get_database_or_default(&self, name: Option<&str>) -> Option<Arc<RwLock<Box<dyn Database>>>> {
        match name {
            Some(n) if !n.is_empty() => self.get_database(n).await,
            _ => self.get_default_database().await,
        }
    }

    /// Get the default database (first one added or "default")
    pub async fn get_default_database(&self) -> Option<Arc<RwLock<Box<dyn Database>>>> {
        let databases = self.databases.read().await;
        
        // Try "default" first
        if let Some(conn) = databases.get("default") {
            return Some(conn.db.clone());
        }
        
        // Otherwise return the first database
        databases.values().next().map(|conn| conn.db.clone())
    }

    /// List all databases and their status
    pub async fn list_databases(&self) -> Vec<DatabaseInfo> {
        let databases = self.databases.read().await;
        databases.values().map(|conn| conn.info.clone()).collect()
    }

    /// Remove a database connection
    pub async fn remove_database(&self, name: &str) -> bool {
        let mut databases = self.databases.write().await;
        databases.remove(name).is_some()
    }

    /// Get database count
    pub async fn database_count(&self) -> usize {
        let databases = self.databases.read().await;
        databases.len()
    }
}

impl Default for DatabaseManager {
    fn default() -> Self {
        Self::new()
    }
}