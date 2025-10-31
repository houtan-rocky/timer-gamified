// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! OOGA VOOGA!", name)
}

use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::Manager;
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize, Debug)]
struct LicenseData {
    key: String,
    activated_at: u64,
}

// Get license file path
fn get_license_path(app_handle: &tauri::AppHandle) -> PathBuf {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to get app data directory");
    std::fs::create_dir_all(&app_data_dir).expect("failed to create app data directory");
    app_data_dir.join("license.json")
}

// Validate license key format and checksum
fn validate_license_key(key: &str) -> bool {
    // Format validation: XXXXX-XXXXX-XXXXX-XXXXX-XXXXX
    if key.len() != 29 {
        return false;
    }
    
    let segments: Vec<&str> = key.split('-').collect();
    if segments.len() != 5 {
        return false;
    }
    
    for segment in &segments {
        if segment.len() != 5 {
            return false;
        }
        if !segment.chars().all(|c| c.is_ascii_alphanumeric()) {
            return false;
        }
    }
    
    // Checksum validation
    let mut checksum: u32 = 0;
    for segment in &segments {
        for ch in segment.chars() {
            checksum = (checksum + ch as u32) % 256;
        }
    }
    
    // Checksum should be between 100-255 for valid keys
    checksum >= 100 && checksum <= 255
}

#[tauri::command]
fn check_license(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let license_path = get_license_path(&app_handle);
    
    if !license_path.exists() {
        return Ok(false);
    }
    
    match fs::read_to_string(&license_path) {
        Ok(content) => {
            match serde_json::from_str::<LicenseData>(&content) {
                Ok(data) => {
                    // Validate the stored key
                    if validate_license_key(&data.key) {
                        Ok(true)
                    } else {
                        // Invalid key, remove the file
                        let _ = fs::remove_file(&license_path);
                        Ok(false)
                    }
                }
                Err(_) => {
                    // Corrupted file, remove it
                    let _ = fs::remove_file(&license_path);
                    Ok(false)
                }
            }
        }
        Err(_) => Ok(false),
    }
}

#[tauri::command]
fn activate_license(app_handle: tauri::AppHandle, key: String) -> Result<bool, String> {
    let key_upper = key.trim().to_uppercase();
    
    if key_upper.is_empty() {
        return Err("License key cannot be empty".to_string());
    }
    
    if !validate_license_key(&key_upper) {
        return Err("Invalid license key format. Expected format: XXXXX-XXXXX-XXXXX-XXXXX-XXXXX".to_string());
    }
    
    let license_data = LicenseData {
        key: key_upper,
        activated_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    };
    
    let license_path = get_license_path(&app_handle);
    
    match serde_json::to_string_pretty(&license_data) {
        Ok(json) => {
            match fs::write(&license_path, json) {
                Ok(_) => Ok(true),
                Err(e) => Err(format!("Failed to save license: {}", e)),
            }
        }
        Err(e) => Err(format!("Failed to serialize license: {}", e)),
    }
}

#[tauri::command]
fn remove_license(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let license_path = get_license_path(&app_handle);
    
    if license_path.exists() {
        match fs::remove_file(&license_path) {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to remove license: {}", e)),
        }
    } else {
        Ok(true) // Already removed
    }
}

// Get media storage directory
fn get_media_dir(app_handle: &tauri::AppHandle) -> PathBuf {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to get app data directory");
    let media_dir = app_data_dir.join("media");
    std::fs::create_dir_all(&media_dir).expect("failed to create media directory");
    std::fs::create_dir_all(media_dir.join("images")).expect("failed to create images directory");
    std::fs::create_dir_all(media_dir.join("sounds")).expect("failed to create sounds directory");
    media_dir
}

// Hash data using SHA256
fn hash_data(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[tauri::command]
fn upload_image(app_handle: tauri::AppHandle, data: Vec<u8>) -> Result<String, String> {
    let hash = hash_data(&data);
    let media_dir = get_media_dir(&app_handle);
    let image_path = media_dir.join("images").join(&hash);
    
    match fs::write(&image_path, data) {
        Ok(_) => Ok(hash),
        Err(e) => Err(format!("Failed to save image: {}", e)),
    }
}

#[tauri::command]
fn upload_sound(app_handle: tauri::AppHandle, data: Vec<u8>) -> Result<String, String> {
    let hash = hash_data(&data);
    let media_dir = get_media_dir(&app_handle);
    let sound_path = media_dir.join("sounds").join(&hash);
    
    match fs::write(&sound_path, data) {
        Ok(_) => Ok(hash),
        Err(e) => Err(format!("Failed to save sound: {}", e)),
    }
}

#[tauri::command]
fn get_image(app_handle: tauri::AppHandle, hash: String) -> Result<Vec<u8>, String> {
    let media_dir = get_media_dir(&app_handle);
    let image_path = media_dir.join("images").join(&hash);
    
    match fs::read(&image_path) {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("Failed to read image: {}", e)),
    }
}

#[tauri::command]
fn get_sound(app_handle: tauri::AppHandle, hash: String) -> Result<Vec<u8>, String> {
    let media_dir = get_media_dir(&app_handle);
    let sound_path = media_dir.join("sounds").join(&hash);
    
    match fs::read(&sound_path) {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("Failed to read sound: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![greet, check_license, activate_license, remove_license, upload_image, upload_sound, get_image, get_sound])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
