#[cfg(feature = "download-to-file")]
mod download_to_file;
#[cfg(feature = "download-to-file")]
pub use self::download_to_file::download_to_file;

#[cfg(feature = "drop-remove-path")]
mod drop_remove_path;
#[cfg(feature = "drop-remove-path")]
pub use self::drop_remove_path::DropRemovePath;

#[cfg(feature = "download-to-path")]
mod download_to_path;
#[cfg(feature = "download-to-path")]
pub use self::download_to_path::download_to_path;

#[cfg(feature = "arc-anyhow-error")]
mod arc_anyhow_error;
#[cfg(feature = "arc-anyhow-error")]
pub use self::arc_anyhow_error::ArcAnyhowError;

use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

/// Try to create a dir at the given path.
///
/// # Returns
/// Returns `Ok(true)` if the dir was created.
/// Returns `Ok(false)` if the dir already exists.
/// Returns and error if there was an error creating the dir.
pub fn try_create_dir<P>(path: P) -> std::io::Result<bool>
where
    P: AsRef<Path>,
{
    match std::fs::create_dir(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error),
    }
}

/// Try to remove a dir at the given path.
///
/// # Returns
/// Returns `Ok(true)` if the dir was removed.
/// Returns `Ok(false)` if the dir did not exist.
/// Returns and error if there was an error removing the dir.
pub fn try_remove_dir<P>(path: P) -> std::io::Result<bool>
where
    P: AsRef<Path>,
{
    match std::fs::remove_dir(path) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Syncronously remove a file at a path on drop.
///
/// Currently, this only supports files, NOT directories.
#[derive(Debug)]
pub struct DropRemovePathBlocking {
    /// The path
    path: PathBuf,

    /// Whether dropping this should remove the file.
    should_remove: bool,
}

impl DropRemovePathBlocking {
    /// Make a new [`DropRemovePathBlocking`].
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            path: path.as_ref().into(),
            should_remove: true,
        }
    }

    /// Persist the file at this path.
    pub fn persist(&mut self) {
        self.should_remove = false;
    }

    /// Try to drop this file path, removing it if needed.
    ///
    /// # Return
    /// Returns an error if the file could not be removed.
    /// Returns Ok(true) if the file was removed.
    /// Returns Ok(false) if the file was not removed.
    pub fn try_drop(self) -> Result<bool, (Self, std::io::Error)> {
        let wrapper = ManuallyDrop::new(self);
        let should_remove = wrapper.should_remove;

        if should_remove {
            std::fs::remove_file(&wrapper.path)
                .map_err(|e| (ManuallyDrop::into_inner(wrapper), e))?;
        }

        Ok(should_remove)
    }
}

impl AsRef<Path> for DropRemovePathBlocking {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Deref for DropRemovePathBlocking {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl Drop for DropRemovePathBlocking {
    fn drop(&mut self) {
        // Try to remove the path.
        if self.should_remove {
            if let Err(error) = std::fs::remove_file(self.path.clone()) {
                let message = format!("failed to delete file: '{error}'");
                if std::thread::panicking() {
                    eprintln!("{message}");
                } else {
                    panic!("{message}");
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Write;

    #[test]
    fn try_create_dir_works() {
        let path = "test_tmp/try_create_dir";

        std::fs::create_dir_all("test_tmp").expect("failed to create tmp dir");

        try_remove_dir(path).expect("failed to remove dir");
        assert!(try_create_dir(path).expect("failed to create dir"));
        assert!(!try_create_dir(path).expect("failed to create dir"));
        assert!(try_remove_dir(path).expect("failed to remove dir"));
        assert!(!try_remove_dir(path).expect("failed to remove dir"));
    }

    #[test]
    fn drop_remove_file_blocking_sanity_check() {
        std::fs::create_dir_all("test_tmp").expect("failed to create tmp dir");

        let file_path: &Path = "test_tmp/drop_remove_file_blocking.txt".as_ref();
        let file_data = b"testing 1 2 3";

        {
            let mut file = std::fs::File::create(&file_path).expect("failed to create file");
            let drop_remove_path = DropRemovePathBlocking::new(file_path);

            file.write_all(file_data).expect("failed to write data");

            drop(file);
            drop_remove_path.try_drop().expect("failed to close file");
        }
        let exists = file_path.exists();
        assert!(!exists, "nonpersisted file exists");

        {
            let mut file = std::fs::File::create(&file_path).expect("failed to create file");
            let mut drop_remove_path = DropRemovePathBlocking::new(file_path);

            file.write_all(file_data).expect("failed to write data");

            drop_remove_path.persist();

            drop(file);
            drop_remove_path.try_drop().expect("failed to close file");
        }

        let exists = file_path.exists();
        assert!(exists, "persisted file does not exist");

        // Failed cleanup does not matter
        let _ = std::fs::remove_file(file_path).is_ok();
    }
}
