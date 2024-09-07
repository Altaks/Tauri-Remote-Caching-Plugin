#![feature(async_closure)]

use base64::{engine::general_purpose::URL_SAFE_NO_PAD as Base64Encoder, Engine};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{
    command,
    plugin::{Builder, TauriPlugin},
    AppHandle, Manager, Runtime, State,
};
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

#[derive(Default)]
struct CacheState(Arc<RwLock<HashMap<String, String>>>);

#[command]
async fn clear_cache<R: Runtime>(state: State<'_, CacheState>, app_handle: AppHandle<R>) -> Result<(), String> {
    let cache_dir_path = app_handle.path_resolver().resolve_resource("cache/").unwrap();
    
    match std::fs::remove_dir_all(cache_dir_path) {
        Ok(_) => {
            let mut registry_lock = state.0.write().await;
            (*registry_lock).clear();
            println!("Cache cleared");
            Ok(())
        },
        Err(err) => Err(err.to_string()),
    }
}

#[command]
async fn cached<'a, R: Runtime>(url: String, state: State<'a, CacheState>, app_handle: AppHandle<R>) -> Result<String, String> {
    let cloned_lock = state.0.clone();
    let cache_lock = state.0.read().await;

    let url_copy = url.clone();

    match (*cache_lock).get(&url) {
        Some(value) => {
            println!("Cache hit");
            let result = Ok(value.clone());
            drop(cache_lock); // Releasing the read lock
            result
        }
        None => {
            println!("Cache miss");
            drop(cache_lock); // Releasing the read lock

            if let Ok(response) = reqwest::get(url.clone()).await {
                let url64 = Base64Encoder.encode(url.clone());

                let mime_type = response.headers().get("content-type").unwrap_or_else(|| {
                    println!("Failed to access content type on response body");
                    panic!();
                }).to_str().unwrap();

                let file_format = mime_type.split('/').last().unwrap();
                let mut save_path = app_handle.path_resolver().resolve_resource(format!("cache/{}", url64.clone())).unwrap();
                save_path.set_extension(file_format);
                println!("Checking for cache folder existence");
                std::fs::create_dir_all(save_path.clone().parent().unwrap()).expect("Failed to create cache directory");
                let mut file = tokio::fs::File::create(&save_path).await.expect("Failed to create file");

                println!("Unwrapping response bytes");
                let _data = response.bytes().await.unwrap();
                let data = _data.as_ref();

                println!("Writing data to file");
                if file.write_all(data).await.is_ok() {
                    println!("File written successfully");
                    file.flush().await.unwrap();
                    println!("File flushed successfully");
                    // file has been cached ig
                    println!("Injecting file into registry");
                    let mut cache_mtx_2 = cloned_lock.write().await;
                    (&mut *cache_mtx_2).insert(url64, String::from(save_path.to_str().unwrap()));
                    drop(cache_mtx_2);
                    println!("Cached file: {}", save_path.to_str().unwrap());
                    return Ok(String::from(save_path.to_str().unwrap())); // we use the currently saved file instead to avoid double download
                } else {
                    println!("Failed to write file");
                }
            }

            Ok(url_copy)
        }
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("remote-caching")
        .invoke_handler(tauri::generate_handler![cached, clear_cache])
        .setup(|app| {

            let cache_dir_path = app.path_resolver().resolve_resource("cache/").unwrap();

            std::fs::create_dir_all(cache_dir_path.clone()).expect("Failed to create cache directory");
            let mut cache_dir = std::fs::read_dir(&cache_dir_path).unwrap();

            let mut cache_registry = HashMap::<String, String>::new();

            while let Some(entry) = cache_dir.next() {
                if let Ok(entry) = entry {
                    if let Ok(entry_type) = entry.file_type() {
                        if entry_type.is_file() {
                            let path = entry.path();

                            let file_stem = path.file_stem().unwrap().to_str().unwrap();
                            println!("Found file : {} with extension {:?}", file_stem, path.extension().unwrap().to_str());

                            match Base64Encoder.decode(file_stem) {
                                Ok(url64) => {
                                    if let Ok(url) = String::from_utf8(url64) {
                                        cache_registry.insert(url.clone(), path.to_str().unwrap().to_string());
                                        println!("Decoded url : {}", url);
                                    }
                                }
                                Err(e) => {
                                    println!("Failed to decode URL: {}", e);
                                }
                            }
                        }
                    }
                }
            }

            // Inject cache_registry
            app.manage(CacheState(Arc::new(RwLock::new(cache_registry))));
            Ok(())
        })
        .build()
}
