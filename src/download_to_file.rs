use anyhow::{ensure, Context};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Download a url using a GET request to a tokio file.
pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    file: &mut File,
) -> anyhow::Result<()> {
    // Send the request
    let mut response = client
        .get(url)
        .send()
        .await
        .context("failed to get headers")?
        .error_for_status()
        .context("invalid http status")?;

    // Pre-allocate file space if possible.
    let content_length = response.content_length();
    if let Some(content_length) = content_length {
        file.set_len(content_length)
            .await
            .context("failed to pre-allocate file")?;
    }

    // Keep track of the file size in case the server lies
    let mut actual_length = 0;

    // Download the file chunk-by-chunk
    while let Some(chunk) = response.chunk().await.context("failed to get next chunk")? {
        file.write_all(&chunk)
            .await
            .context("failed to write to file")?;

        // This will panic if the server sends back a chunk larger than 4GB,
        // which is incredibly unlikely/probably impossible.
        actual_length += u64::try_from(chunk.len()).unwrap();
    }

    // Ensure file size matches content_length
    if let Some(content_length) = content_length {
        ensure!(
            content_length == actual_length,
            "content-length mismatch, {content_length} (content length) != {actual_length} (actual length)",
        );
    }

    // Sync data
    file.flush().await.context("failed to flush file")?;
    file.sync_all().await.context("failed to sync file data")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        let client = reqwest::Client::new();
        tokio::fs::create_dir_all("test_tmp")
            .await
            .expect("failed to create tmp dir");
        let mut file = File::create("test_tmp/download_to_file_google.html")
            .await
            .expect("failed to open");
        download_to_file(&client, "http://google.com", &mut file)
            .await
            .expect("failed to download");
    }
}
