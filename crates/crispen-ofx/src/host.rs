//! OpenFX host implementation for plugin discovery and basic binary loading.
//!
//! This module discovers OFX plugin binaries on disk, loads each dynamic
//! library, and queries exported plug-in descriptors via:
//! - `OfxSetHost` (optional)
//! - `OfxGetNumberOfPlugins` (required)
//! - `OfxGetPlugin` (required)

use std::collections::BTreeSet;
use std::ffi::{CStr, c_char, c_int, c_uint, c_void};
use std::fs;
use std::path::{Path, PathBuf};

use libloading::{Library, Symbol};
use thiserror::Error;

type OfxPropertySetHandle = *mut c_void;
type OfxStatus = c_int;
type FetchSuiteFn =
    unsafe extern "C" fn(OfxPropertySetHandle, *const c_char, c_int) -> *const c_void;
type OfxSetHostFn = unsafe extern "C" fn(*const OfxHostRaw) -> OfxStatus;
type OfxGetNumberOfPluginsFn = unsafe extern "C" fn() -> c_int;
type OfxGetPluginFn = unsafe extern "C" fn(c_int) -> *mut OfxPluginRaw;

#[repr(C)]
struct OfxHostRaw {
    host: OfxPropertySetHandle,
    fetch_suite: Option<FetchSuiteFn>,
}

#[repr(C)]
struct OfxPluginRaw {
    plugin_api: *const c_char,
    api_version: c_int,
    plugin_identifier: *const c_char,
    plugin_version_major: c_uint,
    plugin_version_minor: c_uint,
    set_host: Option<unsafe extern "C" fn(*mut OfxHostRaw)>,
    main_entry: Option<
        unsafe extern "C" fn(
            *const c_char,
            *const c_void,
            OfxPropertySetHandle,
            OfxPropertySetHandle,
        ) -> OfxStatus,
    >,
}

/// Information about an OFX plugin descriptor exported from a binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfxPluginDescriptor {
    pub binary_path: PathBuf,
    pub plugin_index: i32,
    pub plugin_api: String,
    pub api_version: i32,
    pub plugin_identifier: String,
    pub plugin_version_major: u32,
    pub plugin_version_minor: u32,
}

/// Non-fatal plugin binary load failure captured during refresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfxLoadFailure {
    pub binary_path: PathBuf,
    pub message: String,
}

#[derive(Debug, Error)]
enum BinaryLoadError {
    #[error("failed to load library: {0}")]
    LoadLibrary(#[source] libloading::Error),
    #[error("missing required symbol `{symbol}`: {source}")]
    MissingSymbol {
        symbol: &'static str,
        #[source]
        source: libloading::Error,
    },
    #[error("OfxGetNumberOfPlugins returned invalid negative value {0}")]
    InvalidPluginCount(i32),
}

struct LoadedBinary {
    _library: Library,
    _binary_path: PathBuf,
}

/// Manages OpenFX plugin discovery and descriptor loading.
pub struct OfxHost {
    search_paths: Vec<PathBuf>,
    loaded_binaries: Vec<LoadedBinary>,
    plugins: Vec<OfxPluginDescriptor>,
    failures: Vec<OfxLoadFailure>,
}

impl OfxHost {
    /// Create a new OpenFX host and scan for available plugins.
    pub fn new() -> Self {
        let mut host = Self::with_search_paths(Self::default_search_paths());
        host.refresh();
        host
    }

    /// Create an OpenFX host with explicit search roots.
    pub fn with_search_paths(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            loaded_binaries: Vec::new(),
            plugins: Vec::new(),
            failures: Vec::new(),
        }
    }

    /// Compute default OFX plugin search paths for this platform.
    pub fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        if let Some(var) = std::env::var_os("OFX_PLUGIN_PATH") {
            paths.extend(std::env::split_paths(&var));
        }

        #[cfg(target_os = "linux")]
        {
            paths.push(PathBuf::from("/usr/OFX/Plugins"));
            paths.push(PathBuf::from("/usr/local/OFX/Plugins"));
            if let Some(home) = std::env::var_os("HOME") {
                paths.push(PathBuf::from(home).join(".ofx/Plugins"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            paths.push(PathBuf::from("/Library/OFX/Plugins"));
            if let Some(home) = std::env::var_os("HOME") {
                paths.push(PathBuf::from(home).join("Library/OFX/Plugins"));
            }
        }

        #[cfg(target_os = "windows")]
        {
            paths.push(PathBuf::from(r"C:\Program Files\Common Files\OFX\Plugins"));
            if let Some(program_files_x86) = std::env::var_os("ProgramFiles(x86)") {
                paths.push(PathBuf::from(program_files_x86).join(r"Common Files\OFX\Plugins"));
            }
        }

        dedup_paths(paths)
    }

    /// Return current plugin search paths.
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Replace search paths (does not trigger refresh).
    pub fn set_search_paths(&mut self, search_paths: Vec<PathBuf>) {
        self.search_paths = dedup_paths(search_paths);
    }

    /// Discover candidate OFX plugin binaries under current search roots.
    pub fn discover_plugin_binaries(&self) -> Vec<PathBuf> {
        let mut discovered = BTreeSet::new();
        for root in &self.search_paths {
            collect_plugin_binaries(root, &mut discovered);
        }
        discovered.into_iter().collect()
    }

    /// Rescan filesystem and reload all discovered plugin binaries.
    ///
    /// Failures in individual binaries are recorded in [`failures`](Self::failures)
    /// and do not abort the refresh.
    pub fn refresh(&mut self) {
        self.loaded_binaries.clear();
        self.plugins.clear();
        self.failures.clear();

        let binaries = self.discover_plugin_binaries();
        for binary_path in binaries {
            match unsafe { load_binary(&binary_path) } {
                Ok((loaded_binary, mut descriptors)) => {
                    self.loaded_binaries.push(loaded_binary);
                    self.plugins.append(&mut descriptors);
                }
                Err(error) => {
                    tracing::debug!(
                        "Failed to load OFX binary {}: {error}",
                        binary_path.display()
                    );
                    self.failures.push(OfxLoadFailure {
                        binary_path,
                        message: error.to_string(),
                    });
                }
            }
        }
    }

    /// Loaded plugin descriptors.
    pub fn plugins(&self) -> &[OfxPluginDescriptor] {
        &self.plugins
    }

    /// Non-fatal binary load failures from the most recent refresh.
    pub fn failures(&self) -> &[OfxLoadFailure] {
        &self.failures
    }
}

impl Default for OfxHost {
    fn default() -> Self {
        Self::new()
    }
}

fn dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = BTreeSet::new();
    deduped.extend(paths.into_iter().filter(|p| !p.as_os_str().is_empty()));
    deduped.into_iter().collect()
}

fn collect_plugin_binaries(root: &Path, out: &mut BTreeSet<PathBuf>) {
    if !root.exists() {
        return;
    }

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let read_dir = match fs::read_dir(&dir) {
            Ok(iter) => iter,
            Err(error) => {
                tracing::debug!("Skipping unreadable OFX dir {}: {error}", dir.display());
                continue;
            }
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(kind) => kind,
                Err(error) => {
                    tracing::debug!("Skipping OFX entry {}: {error}", path.display());
                    continue;
                }
            };

            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() && is_plugin_binary(&path) {
                out.insert(path);
            }
        }
    }
}

fn is_plugin_binary(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    let ext = ext.to_ascii_lowercase();

    if ext == "ofx" {
        return true;
    }

    #[cfg(target_os = "linux")]
    {
        return ext == "so";
    }
    #[cfg(target_os = "macos")]
    {
        return ext == "dylib";
    }
    #[cfg(target_os = "windows")]
    {
        return ext == "dll";
    }
    #[allow(unreachable_code)]
    false
}

unsafe extern "C" fn fetch_suite_stub(
    _host: OfxPropertySetHandle,
    _suite_name: *const c_char,
    _suite_version: c_int,
) -> *const c_void {
    std::ptr::null()
}

unsafe fn load_binary(
    path: &Path,
) -> Result<(LoadedBinary, Vec<OfxPluginDescriptor>), BinaryLoadError> {
    let library = unsafe { Library::new(path) }.map_err(BinaryLoadError::LoadLibrary)?;

    let get_number_of_plugins: OfxGetNumberOfPluginsFn = {
        let symbol: Symbol<'_, OfxGetNumberOfPluginsFn> = unsafe {
            library.get(b"OfxGetNumberOfPlugins\0").map_err(|source| {
                BinaryLoadError::MissingSymbol {
                    symbol: "OfxGetNumberOfPlugins",
                    source,
                }
            })?
        };
        *symbol
    };

    let get_plugin: OfxGetPluginFn = {
        let symbol: Symbol<'_, OfxGetPluginFn> = unsafe {
            library
                .get(b"OfxGetPlugin\0")
                .map_err(|source| BinaryLoadError::MissingSymbol {
                    symbol: "OfxGetPlugin",
                    source,
                })?
        };
        *symbol
    };

    let set_host = unsafe {
        library
            .get::<OfxSetHostFn>(b"OfxSetHost\0")
            .ok()
            .map(|symbol| *symbol)
    };

    let host = OfxHostRaw {
        host: std::ptr::null_mut(),
        fetch_suite: Some(fetch_suite_stub),
    };
    if let Some(set_host_fn) = set_host {
        let status = unsafe { set_host_fn(&host as *const OfxHostRaw) };
        if status != 0 {
            tracing::debug!(
                "OfxSetHost returned non-zero status {} for {}",
                status,
                path.display()
            );
        }
    }

    let count = unsafe { get_number_of_plugins() };
    if count < 0 {
        return Err(BinaryLoadError::InvalidPluginCount(count));
    }

    let mut descriptors = Vec::new();
    for index in 0..count {
        let ptr = unsafe { get_plugin(index) };
        if ptr.is_null() {
            tracing::debug!(
                "OfxGetPlugin returned null for index {} in {}",
                index,
                path.display()
            );
            continue;
        }

        let raw = unsafe { &*ptr };
        descriptors.push(OfxPluginDescriptor {
            binary_path: path.to_path_buf(),
            plugin_index: index,
            plugin_api: c_string_or_empty(raw.plugin_api),
            api_version: raw.api_version,
            plugin_identifier: c_string_or_empty(raw.plugin_identifier),
            plugin_version_major: raw.plugin_version_major,
            plugin_version_minor: raw.plugin_version_minor,
        });
    }

    Ok((
        LoadedBinary {
            _library: library,
            _binary_path: path.to_path_buf(),
        },
        descriptors,
    ))
}

fn c_string_or_empty(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "crispen_ofx_{}_{}_{}",
            name,
            std::process::id(),
            nanos
        ));
        fs::create_dir_all(&dir).expect("temp test dir should be created");
        dir
    }

    #[test]
    fn discover_plugin_binaries_finds_ofx_files() {
        let root = unique_temp_dir("discover");
        let bundle_dir = root.join("TestPlugin.ofx.bundle/Contents/Linux-x86-64");
        fs::create_dir_all(&bundle_dir).expect("bundle dir should be created");
        let plugin_file = bundle_dir.join("test_plugin.ofx");
        let mut file = fs::File::create(&plugin_file).expect("plugin file should be created");
        writeln!(file, "not-a-real-plugin").expect("write should succeed");

        let host = OfxHost::with_search_paths(vec![root.clone()]);
        let binaries = host.discover_plugin_binaries();
        assert!(
            binaries.contains(&plugin_file),
            "discovered binaries should include {}",
            plugin_file.display()
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discover_plugin_binaries_skips_non_plugin_files() {
        let root = unique_temp_dir("non_plugin");
        let plain_dir = root.join("plain");
        fs::create_dir_all(&plain_dir).expect("plain dir should be created");
        let not_plugin = plain_dir.join("notes.txt");
        let mut file = fs::File::create(&not_plugin).expect("notes file should be created");
        writeln!(file, "hello").expect("write should succeed");

        let host = OfxHost::with_search_paths(vec![root.clone()]);
        let binaries = host.discover_plugin_binaries();
        assert!(
            !binaries.contains(&not_plugin),
            "non plugin file should not be discovered"
        );

        let _ = fs::remove_dir_all(root);
    }
}
