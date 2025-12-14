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

use std::path::Path;

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn push_path_extension_works() {
        let base_path = PathBuf::from("file.txt");
        let extension = "part";
        let with_push_extension_path = with_push_extension(&base_path, extension);
        let mut push_extension_path = base_path;
        push_extension(&mut push_extension_path, extension);

        let expected = Path::new("file.txt.part");
        assert!(with_push_extension_path == expected);
        assert!(push_extension_path == expected);

        let base_path = PathBuf::from("file");
        let extension = "part";
        let with_push_extension_path = with_push_extension(&base_path, extension);
        let mut push_extension_path = base_path;
        push_extension(&mut push_extension_path, extension);

        let expected = Path::new("file.part");
        assert!(with_push_extension_path == expected);
        assert!(push_extension_path == expected);
    }

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
}
