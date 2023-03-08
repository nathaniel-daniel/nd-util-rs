use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

/// Asyncronously remove a file at a path on drop.
///
/// Currently, this only supports files, NOT directories.
#[derive(Debug)]
pub struct DropRemovePath {
    /// The path
    path: PathBuf,

    /// Whether dropping this should remove the file.
    should_remove: bool,
}

impl DropRemovePath {
    /// Make a new [`DropRemovePath`].
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
    pub async fn try_drop(self) -> Result<bool, (Self, std::io::Error)> {
        let wrapper = ManuallyDrop::new(self);
        let should_remove = wrapper.should_remove;

        if should_remove {
            tokio::fs::remove_file(&wrapper.path)
                .await
                .map_err(|e| (ManuallyDrop::into_inner(wrapper), e))?;
        }

        Ok(should_remove)
    }
}

impl AsRef<Path> for DropRemovePath {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Deref for DropRemovePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl Drop for DropRemovePath {
    fn drop(&mut self) {
        let should_remove = self.should_remove;
        let path = std::mem::take(&mut self.path);

        // Try to remove the path.
        tokio::spawn(async move {
            if should_remove {
                if let Err(error) = tokio::fs::remove_file(path).await {
                    let message = format!("failed to delete file: '{error}'");
                    if std::thread::panicking() {
                        eprintln!("{message}");
                    } else {
                        panic!("{message}");
                    }
                }
            }
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn drop_remove_tokio_file_sanity_check() {
        tokio::fs::create_dir_all("test_tmp")
            .await
            .expect("failed to create tmp dir");

        let file_path: &Path = "test_tmp/test.txt".as_ref();
        let file_data = b"testing 1 2 3";

        {
            let mut file = tokio::fs::File::create(&file_path)
                .await
                .expect("failed to create file");
            let drop_remove_path = DropRemovePath::new(file_path);

            file.write_all(file_data)
                .await
                .expect("failed to write data");

            drop(file);
            drop_remove_path
                .try_drop()
                .await
                .expect("failed to close file");
        }
        let exists = file_path.exists();
        assert!(!exists, "nonpersisted file exists");

        {
            let mut file = tokio::fs::File::create(&file_path)
                .await
                .expect("failed to create file");
            let mut drop_remove_path = DropRemovePath::new(file_path);

            file.write_all(file_data)
                .await
                .expect("failed to write data");

            drop_remove_path.persist();

            drop(file);
            drop_remove_path
                .try_drop()
                .await
                .expect("failed to close file");
        }

        let exists = file_path.exists();
        assert!(exists, "persisted file does not exist");

        // Failed cleanup does not matter
        let _ = tokio::fs::remove_file(file_path).await.is_ok();
    }
}
