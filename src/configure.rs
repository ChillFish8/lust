use std::fs::read_to_string;
use std::sync::Arc;

use gotham_derive::StateData;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};

use crate::image::ImageFormat;
use crate::storage::DatabaseBackend;

/// The size of the pages when listing indexes via the admin panel.
pub const PAGE_SIZE: i64 = 50;

/// A cheaply cloneable version of the given configuration
/// for shared state middleware.
#[derive(Clone, StateData)]
pub struct StateConfig(pub Arc<Config>);

#[derive(Copy, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Off,
    Info,
    Debug,
    Error,
}

/// A given size of a preset.
/// Any uploaded images will be automatically duplicated and resized in this
/// preset.
#[derive(Deserialize)]
pub struct SizingPreset {
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize)]
pub struct Config {
    pub log_level: LogLevel,
    pub host: String,
    pub port: u16,
    pub base_data_path: String,
    pub formats: HashMap<ImageFormat, bool>,
    pub database_backend: DatabaseBackend,
    pub size_presets: HashMap<String, SizingPreset>,
    pub default_serving_preset: String,
    pub default_serving_format: ImageFormat,
    pub webp_quality: Option<f32>,
    pub webp_compression: Option<f32>,
    pub webp_method: Option<u8>,
    pub webp_threads: Option<u32>,
    pub cache_size: usize,
}

impl Config {
    pub fn from_file(file: &str) -> anyhow::Result<Self> {
        let data = read_to_string(file)?;
        Ok(serde_json::from_str::<Self>(&data)?)
    }

    pub fn template(backend: &str) -> anyhow::Result<serde_json::Value> {
        let config = match backend.to_lowercase().as_str() {
            "redis" => json!({
                "type": "redis",
                "config": {
                    "connection_uri": "redis://user:pass@localhost/0",
                    "pool_size": 12,
                }
            }),
            "cassandra" => json!({
                "type": "cassandra",
                "config": {
                    "clusters": [
                        "ip:port",
                        "ip:port",
                        "ip:port",
                    ],
                    "keyspace": {
                        "strategy": "SimpleStrategy",
                        "spec": {
                            "replication_factor": 3
                        }
                    },
                    "user": "",
                    "password": "",
                }
            }),
            "postgres" => json!({
                "type": "postgres",
                "config": {
                    "connection_uri": "postgres://user:pass@localhost/foo",
                    "pool_size": 10,
                }
            }),
            "mysql" => json!({
                "type": "mysql",
                "config": {
                    "connection_uri": "mysql://user:pass@localhost/foo",
                    "pool_size": 10,
                }
            }),
            "sqlite" => json!({
                "type": "sqlite",
                "config": {
                    "connection_uri": "sqlite://database.db",
                    "pool_size": 10,
                }
            }),
            _ => return Err(anyhow::Error::msg("invalid database backend given")),
        };

        Ok(json!({
            "log_level": LogLevel::Info,
            "host": "127.0.0.1",
            "port": 7070,
            "base_data_path": "/images",
            "formats": {
                "png": true,
                "jpeg": true,
                "gif": false,
                "webp": true,
            },
            "database_backend": config,
            "size_presets": {
                "small": {
                    "width": 32,
                    "height": 32,
                },
                "medium": {
                    "width": 64,
                    "height": 64,
                },
                "large": {
                    "width": 128,
                    "height": 128,
                },
            },
            "default_serving_preset": "original",
            "default_serving_format": "webp",
            "webp_quality": None::<f32>,
            "webp_compression": Some(50),
            "webp_method": Some(4),
            "webp_threads": None::<u32>,
            "cache_size": 500,
        }))
    }
}
