//! Credential addresses shared by every saved-connection operation.

use keyring_core::api::CredentialStoreApi;
pub use keyring_core::{Entry, Error};

#[cfg(target_os = "macos")]
use apple_native_keyring_store::keychain::Store;
#[cfg(target_os = "linux")]
use dbus_secret_service_keyring_store::Store;
#[cfg(windows)]
use windows_native_keyring_store::Store;

// This name predates the SpiceDepot branding and identifies existing tokens.
const SERVICE: &str = "dev.spicelens.app";

pub fn entry(connection_id: &str) -> Result<Entry, Error> {
    // Opening the store per operation lets a later call recover if the Linux
    // session bus or Secret Service was unavailable on the first attempt.
    let store = Store::new()?;
    #[cfg(target_os = "linux")]
    {
        // The released v3 builder wrote this target on every Secret Service item.
        let modifiers = std::collections::HashMap::from([("target", "default")]);
        store.build(SERVICE, connection_id, Some(&modifiers))
    }
    #[cfg(not(target_os = "linux"))]
    store.build(SERVICE, connection_id, None)
}
