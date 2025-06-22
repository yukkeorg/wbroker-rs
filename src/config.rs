// MIT License
// Copyright (c) 2025 Yukke.org
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub connection_string: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                connection_string: "sqlite:./sensor_data.db".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default_with_status<P: AsRef<Path>>(path: P) -> (Self, bool) {
        match Self::load_from_file(&path) {
            Ok(config) => (config, true),
            Err(_) => {
                let default_config = Self::default();
                if let Err(e) = default_config.save_to_file(&path) {
                    eprintln!("Warning: Failed to create default config file: {}", e);
                }
                (default_config, false)
            }
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.database.connection_string, "sqlite:./sensor_data.db");
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_string = toml::to_string(&config).unwrap();
        assert!(toml_string.contains("connection_string"));
        assert!(toml_string.contains("sqlite:./sensor_data.db"));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
[database]
connection_string = "sqlite:./test.db"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.database.connection_string, "sqlite:./test.db");
    }

    #[test]
    fn test_load_from_file_success() {
        let temp_file = "test_config.toml";
        let config = Config::default();
        config.save_to_file(temp_file).unwrap();

        let loaded_config = Config::load_from_file(temp_file).unwrap();
        assert_eq!(
            loaded_config.database.connection_string,
            config.database.connection_string
        );

        fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_load_from_file_not_found() {
        let result = Config::load_from_file("nonexistent_config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_to_file() {
        let temp_file = "test_save_config.toml";
        let config = Config {
            database: DatabaseConfig {
                connection_string: "sqlite:./saved_test.db".to_string(),
            },
        };

        let result = config.save_to_file(temp_file);
        assert!(result.is_ok());
        assert!(Path::new(temp_file).exists());

        let content = fs::read_to_string(temp_file).unwrap();
        assert!(content.contains("sqlite:./saved_test.db"));

        fs::remove_file(temp_file).ok();
    }

    #[test]
    fn test_config_debug_format() {
        let config = Config::default();
        let debug_string = format!("{:?}", config);
        assert!(debug_string.contains("Config"));
        assert!(debug_string.contains("DatabaseConfig"));
        assert!(debug_string.contains("sqlite:./sensor_data.db"));
    }

    #[test]
    fn test_database_config_debug_format() {
        let db_config = DatabaseConfig {
            connection_string: "sqlite:./test.db".to_string(),
        };
        let debug_string = format!("{:?}", db_config);
        assert!(debug_string.contains("DatabaseConfig"));
        assert!(debug_string.contains("sqlite:./test.db"));
    }

    #[test]
    fn test_invalid_toml_handling() {
        let invalid_toml = "invalid toml content [[[";
        let result: Result<Config, _> = toml::from_str(invalid_toml);
        assert!(result.is_err());
    }
}
