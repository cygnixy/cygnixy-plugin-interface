# Cygnixy Plugin Interface for Dynamic Lua Integration

The Cygnixy Plugin Interface provides a robust system for managing Lua plugins within the **[Cygnixy framework](https://cygnixy.com)**. It enables seamless dynamic loading, unloading, and registration of Lua functions using Rust. Built with the mlua crate, this interface supports dynamic libraries for modular and extensible plugin development.

## Features

* **Dynamic Plugin Loading**:
  Load plugins from shared libraries ( `.dll` , `.so` , `.dylib` ) at runtime.
  
* **Plugin Interface**:
  Define and implement plugins with a common trait ( `PluginLua` ) to ensure consistent behavior.

* **Lua Function Registration**:
  Automatically register Lua functions exposed by plugins into the Lua runtime.

* **Lifecycle Management**:
  Manage plugin initialization ( `on_load` ) and cleanup ( `on_unload` ) seamlessly.

* **Logging**:
  Integrated with `tracing` for structured logs during plugin operations.

## How It Works

1. **Trait Definition**:
   Each plugin implements the `PluginLua` trait to define its behavior and expose Lua functions.

2. **Plugin Manager**:
   The `PluginManager` structure handles loading plugins, maintaining references to them, and ensuring they are correctly initialized and cleaned up.

3. **Dynamic Libraries**:
   Plugins are compiled as shared libraries and loaded dynamically at runtime using the `libloading` crate.

4. **Lua Integration**:
   Functions provided by plugins are registered into the Lua runtime and made accessible for scripting.

## Usage

### Define a Plugin

To create a plugin, implement the `PluginLua` trait and define Lua functions:

```rust
use mlua::{Function, Lua};
use plugin_interface::{export_plugin, PluginLua};
use std::collections::HashMap;

pub struct MyPlugin;

impl PluginLua for MyPlugin {
    fn name(&self) -> &str {
        "my_plugin"
    }

    fn on_load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("MyPlugin loaded!");
        Ok(())
    }

    fn on_unload(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("MyPlugin unloaded!");
        Ok(())
    }

    fn get_lua_functions(&self, lua: &Lua) -> HashMap<String, Function> {
        let mut functions = HashMap::new();
        functions.insert(
            "hello".to_string(),
            lua.create_function(|_, name: String| {
                println!("Hello, {}!", name);
                Ok(())
            }).unwrap(),
        );
        functions
    }
}

export_plugin!(MyPlugin);
```

### Load a Plugin

Use the `PluginManager` to load, unload, and manage plugins:

```rust
use plugin_manager::PluginManager;
use mlua::Lua;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    let mut manager = PluginManager::new();

    // Load a plugin from a shared library
    manager.load_plugin("path/to/plugin.dll")?;

    // Register plugin functions with Lua
    manager.register_all_plugins(&lua)?;

    // Unload the plugin
    manager.unload_plugin("my_plugin")?;

    Ok(())
}
```

## Safety Considerations

* **Dynamic Loading**:
  Uses `unsafe` blocks to load and interact with shared libraries. Ensure plugins are trusted and well-tested.

* **Thread Safety**:
  Plugins must be thread-safe ( `Send + Sync` ) to avoid undefined behavior in multithreaded contexts.

## Dependencies

* [`mlua`](https://crates.io/crates/mlua): For Lua integration.
* [`libloading`](https://crates.io/crates/libloading): For dynamic library loading.
* [`tracing`](https://crates.io/crates/tracing): For structured logging.

## License

This project is licensed under the MIT License.
