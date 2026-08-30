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

use crate::types::BoxError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                url: "Not specified".to_string(),
            },
        }
    }
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, BoxError> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn load_or_default_with_status<P: AsRef<Path>>(path: P) -> (Self, bool) {
        match Self::load_from_file(&path) {
            Ok(config) => (config, true),
            Err(_) => (Self::default(), false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct TempConfigFile {
        path: PathBuf,
    }

    impl TempConfigFile {
        fn new(name: &str, content: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("wbroker-rs-{name}-{}.toml", std::process::id()));
            fs::write(&path, content).unwrap();
            Self { path }
        }
    }

    impl Drop for TempConfigFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.database.url, "Not specified");
    }

    #[test]
    fn test_load_from_valid_file() {
        let file = TempConfigFile::new(
            "valid",
            r#"
[database]
url = "sqlite:./test.db"
"#,
        );

        let config = Config::load_from_file(&file.path).unwrap();

        assert_eq!(config.database.url, "sqlite:./test.db");
    }

    #[test]
    fn test_load_or_default_for_missing_file() {
        let path =
            std::env::temp_dir().join(format!("wbroker-rs-missing-{}.toml", std::process::id()));
        let _ = fs::remove_file(&path);

        let (config, loaded) = Config::load_or_default_with_status(&path);

        assert!(!loaded);
        assert_eq!(config.database.url, "Not specified");
    }

    #[test]
    fn test_load_or_default_for_invalid_file() {
        let file = TempConfigFile::new("invalid", "invalid toml content [[[");

        assert!(Config::load_from_file(&file.path).is_err());

        let (config, loaded) = Config::load_or_default_with_status(&file.path);
        assert!(!loaded);
        assert_eq!(config.database.url, "Not specified");
    }
}
