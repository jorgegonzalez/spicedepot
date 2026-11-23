use spicedepot_credentials::{entry, Error};
use uuid::Uuid;

#[test]
#[ignore = "requires an unlocked native credential store"]
fn saved_connection_survives_upgrade_and_rollback() {
    let id = Uuid::new_v4().to_string();
    // Reproduce the released app's lookup independently of the current code.
    let old = keyring_v3::Entry::new("dev.spicelens.app", &id).unwrap();
    assert!(matches!(
        old.get_password(),
        Err(keyring_v3::Error::NoEntry)
    ));

    struct Cleanup(keyring_v3::Entry);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = self.0.delete_credential();
        }
    }
    let old = Cleanup(old);
    old.0.set_password("old-🔑-café").unwrap();
    assert_eq!(entry(&id).unwrap().get_password().unwrap(), "old-🔑-café");

    entry(&id).unwrap().set_password("updated-密").unwrap();
    assert_eq!(old.0.get_password().unwrap(), "updated-密");
    entry(&id).unwrap().delete_credential().unwrap();
    assert!(matches!(
        old.0.get_password(),
        Err(keyring_v3::Error::NoEntry)
    ));

    entry(&id).unwrap().set_password("new-🔐").unwrap();
    assert_eq!(old.0.get_password().unwrap(), "new-🔐");
    old.0.delete_credential().unwrap();
    assert!(matches!(
        entry(&id).unwrap().get_password(),
        Err(Error::NoEntry)
    ));
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an unlocked Secret Service collection"]
fn other_secret_service_targets_are_isolated() {
    use dbus_secret_service::{EncryptionType, SecretService};
    use std::collections::HashMap;

    let id = Uuid::new_v4().to_string();
    let ss = SecretService::connect(EncryptionType::Dh).unwrap();
    let collection = ss.get_default_collection().unwrap();
    let other = collection
        .create_item(
            "SpiceDepot target isolation test",
            HashMap::from([
                ("service", "dev.spicelens.app"),
                ("username", id.as_str()),
                ("target", "other"),
            ]),
            b"another fixture",
            false,
            "text/plain",
        )
        .unwrap();

    let current = entry(&id).unwrap();
    assert!(matches!(current.get_password(), Err(Error::NoEntry)));
    current.set_password("own fixture").unwrap();
    assert_eq!(current.get_password().unwrap(), "own fixture");
    current.delete_credential().unwrap();
    assert_eq!(other.get_secret().unwrap(), b"another fixture");
    other.delete().unwrap();
}
