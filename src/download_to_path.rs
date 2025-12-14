use crate::download_to_file;
use crate::DropRemovePath;
use anyhow::Context;
use cfg_if::cfg_if;
use std::path::Path;
use tracing::warn;

/// Using the given client, download the file at a url to a given path.
///
/// Note that this function will overwrite the file at the given path.
///
/// # Temporary Files
/// This will create a temporary ".part" file in the same directory while downloading.
/// On failure, this file will try to be cleaned up.
/// On success, this temporary file will be renamed to the actual file name.
/// As a result, it may be assumed that the file created at the given path is the complete, non-erroneus download.
///
/// # Locking
/// During downloads, the temporary file is locked via advisory locking on platforms that support it.
/// If locking is not supported, overwriting a pre-existing temporary file causes an error.
/// Currently, Unix and Windows support advisory locking.
pub async fn download_to_path<P>(client: &reqwest::Client, url: &str, path: P) -> anyhow::Result<()>
where
    P: AsRef<Path>,
{
    // Get the path.
    let path = path.as_ref();

    // Create temporary path.
    let temporary_path = path.with_added_extension("part");

    // Setup to open the temporary file.
    //
    // We do NOT use mandatory locking on Windows.
    // This is because the file would need to be dropped to be renamed,
    // which leads to a race as we must release ALL locks to do so.
    //
    // TODO: On linux, consider probing for O_TMPFILE support somehow and create an unnamed tmp file and use linkat.
    let mut open_options = tokio::fs::OpenOptions::new();
    open_options.write(true);

    // If we don't have a mechanism to prevent stomping,
    // at least ensure that we can't stomp.
    cfg_if! {
        if #[cfg(any(windows, unix))] {
            // We prevent stomping by locking somehow.
            // Create and overwrite the temporary file.
            open_options.create(true);
        } else {
            // If the temporary file exists, return an error.
            open_options.create_new(true);
        }
    }

    // Open the temporary file.
    let temporary_file = open_options
        .open(&temporary_path)
        .await
        .context("failed to create temporary file")?;

    // Create the remove handle for the temporary path.
    let mut temporary_path = DropRemovePath::new(temporary_path);

    let result = async {
        // Wrap the file in a lock, if the platform supports it.
        cfg_if! {
            if #[cfg(any(unix, windows))] {
                let mut temporary_file_lock = fd_lock::RwLock::new(temporary_file);
                let mut temporary_file = temporary_file_lock.try_write().context("failed to lock temporary file")?;
            } else {
                let mut temporary_file = temporary_file;
            }
        }

        // Perform download.
        download_to_file(client, url, &mut temporary_file)
            .await?;

        // Perform rename from temporary file path to actual file path.
        tokio::fs::rename(&temporary_path, &path)
            .await
            .context("failed to rename temporary file")?;

        // Ensure that the file handle is dropped AFTER we rename.
        //
        // Uwrap the file from the file lock.
        cfg_if! {
            if #[cfg(any(unix, windows))] {
                // Unlock lock.
                drop(temporary_file);

                // Get file from lock.
                let temporary_file = temporary_file_lock.into_inner();
            }
        }

        drop(temporary_file.into_std());

        Ok(())
    }
    .await;

    match result.as_ref() {
        Ok(()) => {
            // Persist the file,
            // since it was renamed and we don't want to remove a non-existent file.
            temporary_path.persist();
        }
        Err(_error) => {
            // Try to clean up the temporary file before returning.
            if let Err((mut temporary_path, error)) = temporary_path.try_drop().await {
                // Don't try to delete the file again.
                temporary_path.persist();

                // Returning the original error is more important,
                // so we just log the temporary file error here.
                warn!("failed to delete temporary file '{error}'");
            }
        }
    }

    result
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        tokio::fs::create_dir_all("test_tmp")
            .await
            .expect("failed to create tmp dir");

        let client = reqwest::Client::new();
        download_to_path(&client, "http://google.com", "test_tmp/google.html")
            .await
            .expect("failed to download");
    }
}
