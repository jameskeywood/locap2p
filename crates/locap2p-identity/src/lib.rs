//! Persistent libp2p identities for framework nodes and services.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use libp2p::identity::Keypair;
use thiserror::Error;

/// Loads an identity from `path`, creating a new Ed25519 identity if it does
/// not exist yet.
pub fn load_or_generate(path: impl AsRef<Path>) -> Result<Keypair, IdentityError> {
    let path = path.as_ref();

    if path.exists() {
        return load(path);
    }

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(IdentityError::Io)?;
    }

    let keypair = Keypair::generate_ed25519();
    let encoded = keypair
        .to_protobuf_encoding()
        .map_err(IdentityError::Encoding)?;

    let temporary = temporary_path(path);
    write_private_file(&temporary, &encoded)?;

    match fs::rename(&temporary, path) {
        Ok(()) => Ok(keypair),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(&temporary);
            load(path)
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(IdentityError::Io(error))
        }
    }
}

/// Loads an existing identity from `path`.
pub fn load(path: impl AsRef<Path>) -> Result<Keypair, IdentityError> {
    let bytes = fs::read(path.as_ref()).map_err(IdentityError::Io)?;
    Keypair::from_protobuf_encoding(&bytes).map_err(IdentityError::Encoding)
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".tmp");
    PathBuf::from(temporary)
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), IdentityError> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(IdentityError::Io)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(IdentityError::Io)?;
    }

    file.write_all(bytes).map_err(IdentityError::Io)?;
    file.sync_all().map_err(IdentityError::Io)?;
    Ok(())
}

/// Errors returned while loading or creating an identity.
#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("identity file I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid identity encoding: {0}")]
    Encoding(#[from] libp2p::identity::DecodingError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_reloads_identity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("node.key");

        let first = load_or_generate(&path).unwrap();
        let second = load_or_generate(&path).unwrap();

        assert_eq!(first.public().to_peer_id(), second.public().to_peer_id());
    }

    #[test]
    fn loads_existing_identity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("node.key");
        let keypair = load_or_generate(&path).unwrap();

        let loaded = load(&path).unwrap();

        assert_eq!(keypair.public().to_peer_id(), loaded.public().to_peer_id());
    }
}
