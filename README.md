# Tauri Plugin | Remote caching

Caches remote files in the user's application folder for a better bandwidth management.

## Installation

*This plugin requires a Rust version of at least 1.60*

There are three general methods of installation that we can recommend.

- Use crates.io and npm (easiest, and requires you to trust that our publishing pipeline worked)
- Pull sources directly from Github using git tags / revision hashes (most secure)
- Git submodule install this repo in your tauri project and then use file protocol to ingest the source (most secure, but inconvenient to use)
 
Install the Core plugin by adding the following to your `Cargo.toml` file :

```toml
[dependencies]
tauri-plugin-remote-caching = { git = "https://github.com/Altaks/Tauri-Remote-Caching-Plugin/"}
```

You can install the JavaScript/TypeScript guest bindings using your preferred JavaScript package manager : 

> Note: If your JavaScript package manager cannot install packages from git monorepos, you can still use the code by manually copying the Guest bindings into your source files

```sh
pnpm add https://github.com/Altaks/Tauri-Remote-Caching-Plugin

# or 
npm add https://github.com/Altaks/Tauri-Remote-Caching-Plugin

# or
yarn add https://github.com/Altaks/Tauri-Remote-Caching-Plugin
```

## Usage

First of all you need to register the plugin in the Rust part of you application : 

`src-tauri/src/main.rs` : 

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_remote_caching::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Afterward the whole plugin's API is available through the JavaScript Guest bindings : 

> Warning : You must add the following inside your application config (`tauri.conf.json`) inside the `tauri` configuration part : 
> 
> ```json
> "protocol": {
>    "asset": true,
>    "assetScope": ["**"]
> }
> ```
> Furthermore, you must enable these features on your app's `tauri` dependency : 
> 
> ```toml
> tauri = { version = "1", features = [ "protocol-asset", "fs-all", "shell-open", "path-all"] }
> ```

```tsx
export const CachedImage = ({src, className}: {src: string, className?: string}) => {
    
    // We set a default state of no-url to display nothing
    const [url, setUrl] = useState("");

    // Loads the requested image
    const loadImage = async () => {
        console.log(`Searching cached image`)
        const cachedImage = await cached(src)
        console.log(`Found cached image : ${cachedImage}`)
        setUrl(convertFileSrc(cachedImage));
    }

    // Start the image caching / retrieving
    useEffect(() => {
        loadImage().catch(console.error)
    }, [src]);

    return (
        <>
            <img src={url} alt={url} className={className} decoding={"async"}/>
        </>
    )
}
```

## Contributing

PR accepted. I might take time to make PR's reviews, feel free to contact me `altair61.dev@gmail.com`.
