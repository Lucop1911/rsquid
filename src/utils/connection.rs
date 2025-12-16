use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub name: String,
    pub db_type: String, // postgres, mysql, mariadb, sqlite
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
}

impl Connection {
    pub fn to_connection_string(&self) -> String {
        match self.db_type.as_str() {
            "postgres" => {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    self.username, self.password, self.host, self.port, self.database
                )
            }
            "mysql" | "mariadb" => {
                if self.username.is_empty() {
                    format!("mysql://{}:{}/{}", self.host, self.port, self.database)
                } else if self.password.is_empty() {
                    format!("mysql://{}@{}:{}/{}", self.username, self.host, self.port, self.database)
                } else {
                    format!(
                        "mysql://{}:{}@{}:{}/{}",
                        self.username, self.password, self.host, self.port, self.database
                    )
                }
            }
            "sqlite" => {
                format!("sqlite://{}", self.database)
            }
            _ => {
                eprintln!("Unsupported database type: {}", self.db_type);
                String::new()
            }
        }
    }
}

pub struct ConnectionManager {
    config_path: PathBuf,
}

impl ConnectionManager {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("Could not find config directory")?
            .join("rsquid");
        
        fs::create_dir_all(&config_dir)?;
        
        let config_path = config_dir.join("connections.json");
        
        Ok(Self { config_path })
    }

    pub fn load_connections(&self) -> Result<Vec<Connection>> {
        if !self.config_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.config_path)?;
        let connections: Vec<Connection> = serde_json::from_str(&content)?;
        Ok(connections)
    }

    pub fn save_connection(&self, connection: Connection) -> Result<()> {
        let mut connections = self.load_connections().unwrap_or_default();
        connections.push(connection);
        
        let content = serde_json::to_string_pretty(&connections)?;
        fs::write(&self.config_path, content)?;
        
        Ok(())
    }

    pub fn delete_connection(&self, index: usize) -> Result<()> {
        let mut connections = self.load_connections()?;
        
        if index < connections.len() {
            connections.remove(index);
            let content = serde_json::to_string_pretty(&connections)?;
            fs::write(&self.config_path, content)?;
        }
        
        Ok(())
    }

    pub fn update_connection(&self, index: usize, connection: Connection) -> Result<()> {
        let mut connections = self.load_connections()?;
        
        if index < connections.len() {
            connections[index] = connection;
            let content = serde_json::to_string_pretty(&connections)?;
            fs::write(&self.config_path, content)?;
        }
        
        Ok(())
    }
}