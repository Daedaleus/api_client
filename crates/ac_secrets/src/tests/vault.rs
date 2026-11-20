use std::path::{Path, PathBuf};

use ac_app::ports::{SecretError, SecretValues, SecretVault};
use age::secrecy::ExposeSecret;
use age::x25519;

use super::{AgeVault, CI_VARIABLE, KeyConfig};

/// A throwaway public key (its private key was never kept).
const SSH_PUBLIC_KEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOcrctpxeDVU0DQbIWlXheSUkd5TRgpltLt6mDZYjBzg test";

fn config(dir: &Path) -> KeyConfig {
    KeyConfig {
        identity_file: dir.join("config").join("identity.txt"),
        ci_identity: None,
        ssh_dir: None,
    }
}

fn keys_file(dir: &tempfile::TempDir) -> PathBuf {
    config(dir.path()).identity_file
}

fn values() -> SecretValues {
    SecretValues::from([
        ("token".to_owned(), "s3cr3t \"quoted\"".to_owned()),
        ("password".to_owned(), "hunter2".to_owned()),
    ])
}

#[test]
fn creates_the_own_key_once_and_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    let vault = AgeVault::new(config(dir.path()));
    let first = vault.own_recipient().unwrap();
    assert!(first.starts_with("age1"));
    assert_eq!(vault.own_recipient().unwrap(), first);
    let file = std::fs::read_to_string(dir.path().join("config/identity.txt")).unwrap();
    assert!(file.contains(&format!("# public key: {first}")));
    assert!(file.contains("AGE-SECRET-KEY-"));
}

#[test]
fn encrypts_armored_for_the_own_key_and_decrypts_again() {
    let dir = tempfile::tempdir().unwrap();
    let vault = AgeVault::new(config(dir.path()));
    let own = vault.own_recipient().unwrap();

    let data = vault
        .encrypt(&values(), &[own, SSH_PUBLIC_KEY.to_owned()])
        .unwrap();
    let text = String::from_utf8(data.clone()).unwrap();
    assert!(text.starts_with("-----BEGIN AGE ENCRYPTED FILE-----"));
    assert!(!text.contains("hunter2"));
    assert_eq!(vault.decrypt(&data).unwrap(), values());
}

#[test]
fn decrypts_with_a_ci_key_but_not_with_others() {
    let dir = tempfile::tempdir().unwrap();
    let ci = x25519::Identity::generate();
    let mut with_ci = config(dir.path());
    with_ci.ci_identity = Some(ci.to_string().expose_secret().to_owned());
    let ci_vault = AgeVault::new(with_ci);

    let other = tempfile::tempdir().unwrap();
    let other_vault = AgeVault::new(config(other.path()));
    let data = other_vault
        .encrypt(&values(), &[ci.to_public().to_string()])
        .unwrap();

    assert_eq!(ci_vault.decrypt(&data).unwrap(), values());
    other_vault.own_recipient().unwrap();
    assert_eq!(other_vault.decrypt(&data), Err(SecretError::NoKey));
}

#[test]
fn without_any_key_nothing_decrypts() {
    let dir = tempfile::tempdir().unwrap();
    let mut keys = config(dir.path());
    keys.ssh_dir = Some(dir.path().join("no-ssh"));
    let vault = AgeVault::new(keys);
    assert_eq!(vault.decrypt(b"anything"), Err(SecretError::NoKey));
}

#[test]
fn rejects_missing_and_invalid_recipients() {
    let dir = tempfile::tempdir().unwrap();
    let vault = AgeVault::new(config(dir.path()));
    assert_eq!(
        vault.encrypt(&values(), &[]),
        Err(SecretError::NoRecipients)
    );
    for invalid in ["age1nope", "ssh-ed25519 nope", "alice@example.com"] {
        assert_eq!(
            vault.encrypt(&values(), &[invalid.to_owned()]),
            Err(SecretError::InvalidRecipient(invalid.to_owned())),
        );
    }
}

#[test]
fn reports_damaged_data_and_key_files() {
    let dir = tempfile::tempdir().unwrap();
    let vault = AgeVault::new(config(dir.path()));
    vault.own_recipient().unwrap();
    assert!(matches!(
        vault.decrypt(b"not age"),
        Err(SecretError::Damaged(_))
    ));

    let broken = tempfile::tempdir().unwrap();
    let keys = config(broken.path());
    std::fs::create_dir_all(keys.identity_file.parent().unwrap()).unwrap();
    std::fs::write(&keys.identity_file, "# no key here\n").unwrap();
    let vault = AgeVault::new(keys);
    assert!(matches!(vault.own_recipient(), Err(SecretError::Key(_))));
    assert_eq!(vault.decrypt(b"x"), Err(SecretError::NoKey));
    std::fs::write(keys_file(&broken), "AGE-SECRET-KEY-broken\n").unwrap();
    assert!(matches!(vault.decrypt(b"x"), Err(SecretError::Key(_))));

    let mut bad_ci = config(dir.path());
    bad_ci.ci_identity = Some("not a key".to_owned());
    assert!(matches!(
        AgeVault::new(bad_ci).decrypt(b"x"),
        Err(SecretError::Key(detail)) if detail.contains(CI_VARIABLE)
    ));
}

#[test]
fn skips_ssh_keys_it_cannot_use() {
    let dir = tempfile::tempdir().unwrap();
    let ssh = dir.path().join("ssh");
    std::fs::create_dir_all(&ssh).unwrap();
    std::fs::write(ssh.join("id_ed25519"), "not a key").unwrap();
    let mut keys = config(dir.path());
    keys.ssh_dir = Some(ssh);
    let vault = AgeVault::new(keys);
    let own = vault.own_recipient().unwrap();
    let data = vault.encrypt(&values(), &[own]).unwrap();
    assert_eq!(vault.decrypt(&data).unwrap(), values());
}

#[test]
fn reads_keys_from_the_process_environment() {
    let file = PathBuf::from("identity.txt");
    let keys = KeyConfig::from_environment(file.clone());
    if std::env::var_os(super::IDENTITY_FILE_VARIABLE).is_none() {
        assert_eq!(keys.identity_file, file);
    }
    assert!(keys.ssh_dir.is_none_or(|dir| dir.ends_with(".ssh")));
}

#[test]
fn rejects_payloads_it_does_not_know() {
    use super::payload::decode;
    for text in [
        "not toml = = =",
        "format = \"other\"\nversion = 1",
        "format = \"api-client/secrets\"\nversion = 2",
        "format = \"api-client/secrets\"\nversion = \"1\"",
        "format = \"api-client/secrets\"\nversion = 1\nvalues = 3",
        "format = \"api-client/secrets\"\nversion = 1\n[values]\na = 1",
    ] {
        assert!(
            matches!(decode(text), Err(SecretError::Damaged(_))),
            "{text}"
        );
    }
    assert_eq!(
        decode("format = \"api-client/secrets\"\nversion = 1").unwrap(),
        SecretValues::new()
    );
}
