use std::fs::read_to_string;
use std::sync::Arc;

use gotham_derive::StateData;
use hashbrown::HashMap;
use serde::Deserialize;

use crate::context::{CompressionMode, ImageFormat};
use crate::storage::DatabaseBackend;

/// A cheaply cloneable version of the given configuration
/// for shared state middleware.
#[derive(Clone, StateData)]
pub struct StateConfig(pub Arc<Config>);

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
    pub host: String,
    pub port: u16,
    pub base_data_path: String,
    pub formats: HashMap<ImageFormat, bool>,
    pub database_backend: DatabaseBackend,
    pub size_presets: HashMap<String, SizingPreset>,
    pub default_serving_preset: String,
    pub default_serving_format: ImageFormat,
    pub serve_compression_mode: CompressionMode,
}

impl Config {
    pub fn from_file(file: &str) -> anyhow::Result<Self> {
        let data = read_to_string(file)?;
        Ok(serde_json::from_str::<Self>(&data)?)
    }

    pub fn template(backend: &str) -> anyhow::Result<serde_json::Value> {
        let config = match backend.to_lowercase().as_str() {
            "cassandra" => json!({
                "type": "cassandra",
                "config": {
                    "clusters": [
                        "ip:port",
                        "ip:port",
                        "ip:port",
                    ],
                    "replication_factor": 1,
                    "replication_class": "SimpleStrategy",
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
            "serve_compression_mode": CompressionMode::Auto,
        }))
    }
}