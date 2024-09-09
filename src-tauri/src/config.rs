use crate::cpu_test::CpuTestMethod;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use strum::IntoEnumIterator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub test_duration_per_core: String,
    pub cores_to_test: String,
    pub active_test_methods: Vec<CpuTestMethod>,
    pub offset_per_core: HashMap<usize, i32>,
}

lazy_static! {
    pub static ref CONFIG_PATH: PathBuf = dirs::config_dir()
        .unwrap()
        .join("pbo-assistant")
        .join("config.json");
}

/// Load configuration from file
pub fn load_config(config_wirte_lock: &Arc<RwLock<bool>>) -> AppConfig {
    if !CONFIG_PATH.exists() {
        let new_config = AppConfig {
            test_duration_per_core: "10m".to_string(),
            cores_to_test: "".to_string(),
            active_test_methods: CpuTestMethod::iter().collect(),
            offset_per_core: HashMap::new(),
        };

        save_config(&new_config, config_wirte_lock);

        return new_config;
    }

    let config_str = std::fs::read_to_string(CONFIG_PATH.as_path()).unwrap();
    let config: AppConfig = serde_json::from_str(&config_str).unwrap();
    config
}

/// Save configuration to file
pub fn save_config(config: &AppConfig, config_write_lock: &Arc<RwLock<bool>>) {
    let is_someone_already_writing = *config_write_lock.read().unwrap();

    if is_someone_already_writing {
        return;
    } else {
        *config_write_lock.write().unwrap() = true;
    };

    // Check if config file exists, if not create folder structure
    if !CONFIG_PATH.exists() {
        std::fs::create_dir_all(CONFIG_PATH.parent().unwrap()).unwrap();
    }

    let config_str = serde_json::to_string(config).unwrap();
    std::fs::write(CONFIG_PATH.as_path(), config_str).unwrap();

    *config_write_lock.write().unwrap() = false;
}
