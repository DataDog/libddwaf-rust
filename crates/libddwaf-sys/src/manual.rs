use std::ffi::CStr;
use std::fmt;
use std::path::Path;
use std::sync::OnceLock;

use crate::ddwaf;

// Never unload: handles, allocator callbacks and version strings can outlive callers.
pub(crate) static LIBRARY: OnceLock<ddwaf> = OnceLock::new();

/// Failure to initialize the manually loaded libddwaf library.
#[derive(Debug)]
#[non_exhaustive]
pub enum LoadError {
    /// A library has already been installed. Replacing it is not supported.
    AlreadyLoaded,
    /// The OS could not open the library or one of its dependencies.
    Library(libloading::Error),
    /// A required exported function is missing.
    Symbol(libloading::Error),
    /// The native library version does not match the generated bindings.
    Version {
        expected: &'static str,
        found: Option<String>,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyLoaded => f.write_str("libddwaf is already loaded"),
            Self::Library(error) => write!(f, "could not load libddwaf: {error}"),
            Self::Symbol(error) => write!(f, "missing libddwaf symbol: {error}"),
            Self::Version { expected, found } => write!(
                f,
                "expected libddwaf {expected}, found {}",
                found.as_deref().unwrap_or("a null version string")
            ),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Library(error) | Self::Symbol(error) => Some(error),
            _ => None,
        }
    }
}

/// Loads libddwaf once, before using any other libddwaf API.
///
/// With `Some(path)`, opens that file. With `None`, asks the OS loader to resolve
/// `libddwaf.so`, `libddwaf.dylib` or `ddwaf.dll` using its usual search rules.
/// This mode does not embed a library or fall back to another one.
///
/// The version must match this crate exactly, and every required symbol must be
/// present before the library is published. An error leaves initialization
/// retryable. After success the library stays loaded for the process lifetime;
/// subsequent calls return [`LoadError::AlreadyLoaded`]. Concurrent calls can
/// open libraries, but only one can publish its bindings.
///
/// On failure, callers can disable their product without terminating the host
/// process. They must not proceed to create WAF resources after an error.
///
/// # Errors
/// Returns an error for OS loading failures, missing symbols, a mismatched
/// version, or an already initialized library.
///
/// # Safety
/// The selected library and its dependencies must be trusted, with initializers
/// and finalizers safe to execute. Its exports must implement the libddwaf ABI
/// for this crate's version, including `ddwaf_get_version` returning a valid
/// null-terminated string (or null). A version check cannot prove ABI safety.
/// The OS loader search path must not be changed concurrently with loading.
///
/// ```no_run
/// # #[cfg(feature = "dynamic-manual")]
/// # fn main() -> Result<(), libddwaf_sys::LoadError> {
/// // This path must identify a trusted, ABI-compatible native library.
/// unsafe { libddwaf_sys::load_library(Some(std::path::Path::new("/opt/app/libddwaf.so"))) }?;
/// # Ok(())
/// # }
/// # #[cfg(not(feature = "dynamic-manual"))]
/// # fn main() {}
/// ```
pub unsafe fn load_library(path: Option<&Path>) -> Result<(), LoadError> {
    if LIBRARY.get().is_some() {
        return Err(LoadError::AlreadyLoaded);
    }

    let default_name = libloading::library_filename("ddwaf");
    let path = path.unwrap_or_else(|| Path::new(&default_name));
    let library = unsafe { libloading::Library::new(path) }.map_err(LoadError::Library)?;
    // This function's ABI predates the current object layouts. Check it before
    // resolving or calling the rest of the API.
    let get_version = unsafe {
        library.get::<unsafe extern "C" fn() -> *const std::os::raw::c_char>(b"ddwaf_get_version\0")
    }
    .map_err(LoadError::Symbol)?;
    let version = unsafe { get_version() };
    let version = if version.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(version) })
    };
    let expected = env!("CARGO_PKG_VERSION");
    if version.map(CStr::to_bytes) != Some(expected.as_bytes()) {
        return Err(LoadError::Version {
            expected,
            found: version.map(|version| version.to_string_lossy().into_owned()),
        });
    }

    let bindings = unsafe { ddwaf::from_library(library) }.map_err(LoadError::Symbol)?;
    LIBRARY.set(bindings).map_err(|_| LoadError::AlreadyLoaded)
}
