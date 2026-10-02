//! Canonical command working directories for capability construction.

use nono::{NonoError, Result};
use std::path::{Path, PathBuf};

/// A command cwd resolved at the launch boundary.
///
/// The private representation ensures callers must use the fallible constructor
/// before passing a cwd to policy write checks. This records canonicalization at
/// construction time; it does not pin filesystem objects or prevent later races.
pub(crate) struct CanonicalCwd(PathBuf);

impl CanonicalCwd {
    /// Resolve symlinks and path components, returning an error on failure.
    pub(crate) fn new(path: &Path) -> Result<Self> {
        path.canonicalize()
            .map(Self)
            .map_err(|source| NonoError::PathCanonicalization {
                path: path.to_path_buf(),
                source,
            })
    }

    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::CanonicalCwd;
    use nono::{NonoError, Result};
    use std::os::unix::fs::symlink;
    use std::path::Path;

    fn tempdir() -> Result<tempfile::TempDir> {
        tempfile::tempdir().map_err(|source| NonoError::ConfigWrite {
            path: Path::new("/tmp").to_path_buf(),
            source,
        })
    }

    #[test]
    fn resolves_path_components() -> Result<()> {
        let temp = tempdir()?;
        let child = temp.path().join("child");
        std::fs::create_dir(&child).map_err(|source| NonoError::ConfigWrite {
            path: child.clone(),
            source,
        })?;
        let expected = CanonicalCwd::new(temp.path())?;
        let cwd = CanonicalCwd::new(&child.join("../."))?;
        assert_eq!(cwd.as_path(), expected.as_path());
        assert!(cwd.as_path().is_absolute());
        Ok(())
    }

    #[test]
    fn resolves_symlinked_cwd() -> Result<()> {
        let temp = tempdir()?;
        let alias = temp.path().join("alias");
        symlink(temp.path(), &alias).map_err(|source| NonoError::ConfigWrite {
            path: alias.clone(),
            source,
        })?;
        let expected = CanonicalCwd::new(temp.path())?;
        let cwd = CanonicalCwd::new(&alias)?;
        assert_eq!(cwd.as_path(), expected.as_path());
        Ok(())
    }

    #[test]
    fn missing_cwd_returns_canonicalization_error() -> Result<()> {
        let temp = tempdir()?;
        let missing = temp.path().join("missing");
        assert!(matches!(
            CanonicalCwd::new(&missing),
            Err(NonoError::PathCanonicalization { path, source })
                if path == missing && source.kind() == std::io::ErrorKind::NotFound
        ));
        Ok(())
    }

    #[test]
    fn dangling_symlink_returns_canonicalization_error() -> Result<()> {
        let temp = tempdir()?;
        let alias = temp.path().join("alias");
        symlink(temp.path().join("missing"), &alias).map_err(|source| NonoError::ConfigWrite {
            path: alias.clone(),
            source,
        })?;
        assert!(matches!(
            CanonicalCwd::new(&alias),
            Err(NonoError::PathCanonicalization { path, source })
                if path == alias && source.kind() == std::io::ErrorKind::NotFound
        ));
        Ok(())
    }
}
