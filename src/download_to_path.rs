use crate::DropRemovePathBlocking;
use anyhow::Context;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tracing::warn;

const LOCKING_SUPPORTED: bool = cfg!(unix) || cfg!(windows);

fn download_to_path_blocking(
    handle: tokio::runtime::Handle,
    client: &reqwest::Client,
    url: &str,
    path: PathBuf,
) -> anyhow::Result<()> {
    // Create temporary path.
    let temporary_path = path.with_added_extension("part");

    // Setup to open the temporary file.
    //
    // We do NOT use mandatory locking on Windows.
    // This is because the file would need to be dropped to be renamed,
    // which leads to a race as we must release ALL locks to do so.
    //
    // TODO: On linux, consider probing for O_TMPFILE support somehow and create an unnamed tmp file and use linkat.
    let mut open_options = std::fs::OpenOptions::new();
    open_options.write(true);

    // If we don't have a mechanism to prevent stomping,
    // at least ensure that we can't stomp.
    if LOCKING_SUPPORTED {
        // We prevent stomping by locking somehow.
        // Create and overwrite the temporary file.
        open_options.create(true);
    } else {
        // If the temporary file exists, return an error.
        open_options.create_new(true);
    }

    // Open the temporary file.
    let mut temporary_file = open_options
        .open(&temporary_path)
        .context("failed to create temporary file")?;

    // Create the remove handle for the temporary path.
    let mut temporary_path = DropRemovePathBlocking::new(temporary_path);

    // We fall back to create_new for temp file creation if we know we are on an unsupported platform.
    // Therefore, we don't need to do anything special here if we are on an unsupported platform.
    // TODO: Check for and handle unsupported errors.
    if LOCKING_SUPPORTED {
        temporary_file
            .try_lock()
            .context("failed to lock temporary file")?;
    }

    let result = (|| {
        // Send the request
        let mut response = handle
            .block_on(client.get(url).send())
            .context("failed to get headers")?
            .error_for_status()?;

        // Download the file chunk-by-chunk
        while let Some(chunk) = handle
            .block_on(response.chunk())
            .context("failed to get next chunk")?
        {
            temporary_file
                .write_all(&chunk)
                .context("failed to write to file")?;
        }

        // Sync data
        temporary_file.flush().context("failed to flush file")?;
        temporary_file
            .sync_all()
            .context("failed to sync file data")?;

        // Perform rename from temporary file path to actual file path.
        std::fs::rename(&temporary_path, &path).context("failed to rename temporary file")?;

        Ok(())
    })();

    // Since we renamed (or failed), we can unlock the file and drop it.
    if LOCKING_SUPPORTED {
        temporary_file.unlock()?;
    }
    drop(temporary_file);

    match result.as_ref() {
        Ok(()) => {
            // Persist the file,
            // since it was renamed and we don't want to remove a non-existent file.
            temporary_path.persist();
        }
        Err(_error) => {
            // Try to clean up the temporary file before returning.
            if let Err((mut temporary_path, error)) = temporary_path.try_drop() {
                // Don't try to delete the file again.
                temporary_path.persist();

                // Returning the original error is more important,
                // so we just log the temporary file error here.
                warn!("failed to delete temporary file: \"{error}\"");
            }
        }
    }

    result
}

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
    let handle = tokio::runtime::Handle::try_current()?;
    let client = client.clone();
    let url = url.to_string();
    let path = path.as_ref().to_path_buf();
    tokio::task::spawn_blocking(move || download_to_path_blocking(handle, &client, &url, path))
        .await??;
    Ok(())
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
