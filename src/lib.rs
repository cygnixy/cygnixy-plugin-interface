use libloading::{Library, Symbol};
use mlua::Lua;
use std::collections::HashMap;
use std::error::Error;
use tracing::{error, trace};

/// Trait that all Lua plugins must implement.
///
/// Provides the required methods for plugin lifecycle management and Lua function registration.
pub trait PluginLua: Send + Sync {
    /// Returns the name of the plugin.
    fn name(&self) -> &str;

    /// Called when the plugin is loaded.
    ///
    /// This method is used for initializing resources or performing setup tasks.
    fn on_load(&mut self) -> Result<(), Box<dyn Error>>;

    /// Called when the plugin is unloaded.
    ///
    /// This method allows the plugin to clean up resources before it is removed.
    fn on_unload(&mut self) -> Result<(), Box<dyn Error>>;

    /// Returns a map of Lua functions to be registered with the Lua state.
    ///
    /// Each function is associated with a name, allowing it to be called from Lua scripts.
    fn get_lua_functions(&self, lua: &Lua) -> HashMap<String, mlua::Function>;
}

/// Manages loading, unloading, and interacting with Lua plugins.
///
/// This structure is responsible for:
/// - Loading dynamic libraries that contain Lua plugins.
/// - Managing plugin instances and ensuring they are correctly initialized and cleaned up.
/// - Automatically registering Lua functions provided by plugins.
pub struct PluginManager {
    /// A map of plugin names to plugin instances.
    plugins: HashMap<String, Box<dyn PluginLua>>,
    /// Keeps track of loaded libraries to prevent premature unloading.
    libraries: Vec<Library>,
}

impl Default for PluginManager {
    /// Creates a default instance of `PluginManager`.
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    /// Creates a new `PluginManager`.
    pub fn new() -> Self {
        PluginManager {
            plugins: HashMap::new(),
            libraries: Vec::new(),
        }
    }

    /// Loads a plugin from a dynamic library.
    ///
    /// # Parameters
    /// - `path`: Path to the dynamic library containing the plugin.
    ///
    /// # Returns
    /// - `Ok(())` if the plugin is successfully loaded.
    /// - `Err` if there is an error during loading or initialization.
    ///
    /// # Safety
    /// This method uses unsafe code to interact with the dynamic library and call the plugin's
    /// exported `create_plugin` function.
    pub fn load_plugin(&mut self, path: &str) -> Result<(), Box<dyn Error>> {
        type PluginCreate = unsafe fn() -> *mut dyn PluginLua;

        unsafe {
            // Load the dynamic library.
            let lib = Library::new(path)?;
            trace!("Library loaded from path: {}", path);

            // Locate and invoke the plugin's create function.
            let create_plugin: Symbol<PluginCreate> = lib.get(b"create_plugin")?;
            let mut boxed_raw_plugin = Box::from_raw(create_plugin());

            // Initialize the plugin by calling its `on_load` method.
            boxed_raw_plugin.on_load()?;
            trace!("Plugin '{}' loaded successfully.", boxed_raw_plugin.name());

            self.plugins
                .insert(boxed_raw_plugin.name().to_string(), boxed_raw_plugin);
            self.libraries.push(lib);
        }

        Ok(())
    }

    /// Unloads a plugin by its name.
    ///
    /// # Parameters
    /// - `name`: Name of the plugin to be unloaded.
    ///
    /// # Returns
    /// - `Ok(())` if the plugin is successfully unloaded.
    /// - `Err` if the plugin fails to clean up resources or is not found.
    pub fn unload_plugin(&mut self, name: &str) -> Result<(), Box<dyn Error>> {
        if let Some(mut plugin) = self.plugins.remove(name) {
            // Call `on_unload` to allow the plugin to clean up resources.
            plugin.on_unload()?;
            trace!("Plugin '{}' unloaded successfully.", name);
        } else {
            trace!("Plugin '{}' not found during unload.", name);
        }

        Ok(())
    }

    /// Retrieves a reference to a loaded plugin by its name.
    ///
    /// # Parameters
    /// - `name`: Name of the plugin.
    ///
    /// # Returns
    /// - `Some(&dyn PluginLua)` if the plugin is found.
    /// - `None` if the plugin is not loaded.
    pub fn get_plugin(&self, name: &str) -> Option<&dyn PluginLua> {
        self.plugins.get(name).map(|plugin| plugin.as_ref())
    }

    /// Registers a plugin instance directly, bypassing file loading.
    ///
    /// # Parameters
    /// - `plugin`: A boxed instance of a plugin implementing the `PluginLua` trait.
    ///
    /// # Returns
    /// - `Ok(())` if the plugin was successfully registered.
    /// - `Err(Box<dyn Error>)` if an error occurs during plugin initialization.
    ///
    /// # Example
    /// ```rust
    /// let plugin: Box<dyn PluginLua> = Box::new(MyPlugin::new());
    /// plugin_manager.register_plugin_instance(plugin)?;
    /// ```
    ///
    /// # Notes
    /// - The plugin's `on_load` method is called during this process to initialize the plugin.
    /// - The plugin is stored in the internal plugin map for future reference.
    pub fn register_plugin_instance(
        &mut self,
        mut plugin: Box<dyn PluginLua>,
    ) -> Result<(), Box<dyn Error>> {
        let plugin_name = plugin.name().to_string();
        plugin.on_load()?; // Initialize the plugin
        self.plugins.insert(plugin_name, plugin);
        Ok(())
    }

    /// Registers all Lua functions from all loaded plugins with the given Lua state.
    ///
    /// # Parameters
    /// - `lua`: The Lua state where the functions should be registered.
    ///
    /// # Returns
    /// - `Ok(())` if all functions are registered successfully.
    /// - `Err` if there is an error during registration.
    pub fn register_all_plugins(&self, lua: &Lua) -> Result<(), Box<dyn Error>> {
        for plugin in self.plugins.values() {
            trace!("Registering functions for plugin '{}'.", plugin.name());
            let plugin_table = lua.create_table()?;
            for (name, function) in plugin.get_lua_functions(lua) {
                plugin_table.set(name, function)?;
            }
            lua.globals().set(plugin.name(), plugin_table)?;
            trace!(
                "Functions for plugin '{}' registered successfully.",
                plugin.name()
            );
        }
        Ok(())
    }
}

impl Drop for PluginManager {
    /// Ensures that all plugins are unloaded and cleaned up when the `PluginManager` is dropped.
    fn drop(&mut self) {
        for (_, mut plugin) in self.plugins.drain() {
            // Call `on_unload` for proper cleanup before unloading.
            if let Err(e) = plugin.on_unload() {
                error!("Error unloading plugin: {}", e);
            }
        }
        trace!("Plugins unloaded.");
    }
}

/// Macro to export the plugin's create function.
///
/// This macro defines the `create_plugin` function that is used to instantiate the plugin
/// from a dynamic library.
///
/// # Example
/// ```rust
/// export_plugin!(MyPlugin);
/// ```
#[macro_export]
macro_rules! export_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn create_plugin() -> *mut dyn PluginLua {
            let plugin = <$plugin_type>::new();
            Box::into_raw(Box::new(plugin))
        }
    };
}
