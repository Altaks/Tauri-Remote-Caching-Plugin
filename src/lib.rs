// Use of a URL safe base64 encoder to save the files urls as allowed file names, useful for decoding the file origin after.
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

/// The plugin state, which stores the cache registry, and is thread safe.
#[derive(Default)]
struct CacheState(Arc<RwLock<HashMap<String, String>>>);

/// Clears the files cache directory on the user's disk, registered as a tauri command, still usable from the rust side.
///
/// # Arguments
///
/// * `state`: the plugin current states, which stores the cache registry. Automagically provided by Tauri
/// * `app_handle`: the handle of the app, used to access the current app paths. Automagically provided by Tauri
///
/// returns: Result<(), String>
///
#[command]
async fn clear_cache<R: Runtime>(state: State<'_, CacheState>, app_handle: AppHandle<R>) -> Result<(), String> {

    // Locate the cache directory on the user's disk
    let cache_dir_path = app_handle.path_resolver().resolve_resource("cache/").unwrap();

    // Attempt to remove the file and all of its children files / subdirectories
    match std::fs::remove_dir_all(cache_dir_path) {
        Ok(_) => {
            // Everything went well, we clear the hashmap to avoid trying to load non-valid cache entries
            let mut registry_lock = state.0.write().await;
            (*registry_lock).clear(); // Clearing the cache
            Ok(())
        },
        Err(err) => Err(err.to_string()),
    }
}


/// Allows to cache remote url files on the user's disk, and is saved in the cache registry.
/// If an entry already exists in the cache state, the local file path is returned.
/// Used from a JS/TS side, the provided URL should be used though the `convertFileSrc` method from the tauri API.
///
/// # Arguments
///
/// * `url`: The URL where the file to cache is located.
/// * `state`: The plugin current states, which stores the cache registry. Automagically provided by Tauri
/// * `app_handle`: The handle of the app, used to access the current app paths. Automagically provided by Tauri
///
/// returns: Result<String, String>
///
#[command]
async fn cached<R: Runtime>(url: String, state: State<'_, CacheState>, app_handle: AppHandle<R>) -> Result<String, String> {
    let cloned_lock = state.0.clone();
    let cache_lock = state.0.read().await;

    let url_copy = url.clone();

    // Check if the file is already cached in the registry
    match (*cache_lock).get(&url) {
        Some(value) => {
            // The file is cached, we retrieve the local path
            let result = Ok(value.clone());

            // Releasing the read lock on the cache registry
            drop(cache_lock);

            result
        }
        None => {
            // No entry has been found, releasing the read lock
            drop(cache_lock);

            // Try to request the file from the provided URL, expecting the remote server to respond with the file and the "content-type" header
            if let Ok(response) = reqwest::get(url.clone()).await {

                // Convert the URL to a valid file name by storing it as base64 encoded string
                let url64 = Base64Encoder.encode(url.clone());

                // Get the file format from the "content-type" header, otherwise make the caching task fail.
                let mime_type = response.headers().get("content-type").unwrap_or_else(|| {
                    panic!("Failed to access content type on response body");
                }).to_str().unwrap();

                // Extract the file format from the mime-type
                // TODO : Maybe improve this part of the algorithm to make sure it matches the right file extension ?
                let file_format = mime_type.split('/').last().unwrap();

                // Create the path to which the file should be stored on the user's disk
                let mut save_path = app_handle.path_resolver().resolve_resource(format!("cache/{}", url64.clone())).unwrap();
                save_path.set_extension(file_format);

                // Make sure the "cache/" directory exists in the app working directory
                // Not making any checks because the OS already does it.
                std::fs::create_dir_all(save_path.clone().parent().unwrap()).expect("Failed to create cache directory");

                // Creat the file on the user's disk
                let mut file = tokio::fs::File::create(&save_path).await.expect("Failed to create file");

                // Unwrap the response bytes
                let temp = response.bytes().await.unwrap();
                let data = temp.as_ref();

                // Writing all the bytes to the file, truncating previous data if any
                if file.write_all(data).await.is_ok() {

                    // Flushing the file writing buffer
                    file.flush().await.unwrap();

                    // File has been written successfully, we can now store the file path in the registry
                    let mut cache_mtx_2 = cloned_lock.write().await;
                    (*cache_mtx_2).insert(url64, String::from(save_path.to_str().unwrap()));

                    // Releasing the write lock on the cache registry
                    drop(cache_mtx_2);

                    // Return the cached file path instead of the target URL to avoid double downloads
                    return Ok(String::from(save_path.to_str().unwrap())); // we use the currently saved file instead to avoid double download
                } else {
                    panic!("Failed to write file");
                }
            }

            // If the request to retrieve the file to cache failed, we return the provided URL.
            Ok(url_copy)
        }
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("remote-caching")
        .invoke_handler(tauri::generate_handler![cached, clear_cache])
        .setup(|app| {

            // /!\ IMPORTANT /!\
            // In order for the plugin to work properly, we must initialize it by making sure
            // the cache registry state matches the cache folder content.

            // Creation of the cache directory path.
            let cache_dir_path = app.path_resolver().resolve_resource("cache/").unwrap();

            // Making sure the cache path exists already
            std::fs::create_dir_all(cache_dir_path.clone()).expect("Failed to create cache directory");

            // Reading the cache directory content
            let mut cache_dir = std::fs::read_dir(&cache_dir_path).unwrap();

            // Initialization of the cache registry
            let mut cache_registry = HashMap::<String, String>::new();

            // Iterating over the cache directory content (files, directories, symlinks, etc. (never trust what people can place in app files))
            #[allow(clippy::while_let_on_iterator)]
            while let Some(entry) = cache_dir.next() {

                // If the entry is a valid entry, whatever Rust means by that ?
                if let Ok(entry) = entry {

                    // Checking the entry type
                    if let Ok(entry_type) = entry.file_type() {
                        if entry_type.is_file() {

                            // Get the file stem from the path
                            let path = entry.path();
                            let file_stem = path.file_stem().unwrap().to_str().unwrap();

                            // Attempt to decode the related cached URL from the file name
                            match Base64Encoder.decode(file_stem) {
                                Ok(url64) => {
                                    if let Ok(url) = String::from_utf8(url64) {

                                        // If we manage to decode the URL from which this file has been cached, we store the file path in
                                        // the cache registry using the URL as a key

                                        cache_registry.insert(url.clone(), path.to_str().unwrap().to_string());
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

            // Inject the cache registry state
            app.manage(CacheState(Arc::new(RwLock::new(cache_registry))));
            Ok(())
        })
        .build()
}
