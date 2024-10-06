use reqwest::Client;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

use crate::progress::{run_tasks_with_progress, ProgressBar};

pub fn get_files_in_dir(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            files.extend(
                get_files_in_dir(&entry.path())
                    .into_iter()
                    .map(|x| x.to_path_buf()),
            );
        }
    }
    files
}

pub async fn hash_file(path: &Path) -> Result<String, Box<dyn Error + Send + Sync>> {
    let data = async_fs::read(path).await?;
    Ok(format!("{:x}", Sha1::digest(&data)))
}

pub async fn hash_files<M>(
    files: Vec<PathBuf>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let tasks_count = files.len() as u64;

    let tasks = files
        .into_iter()
        .map(|path| async move { hash_file(&path).await });

    run_tasks_with_progress(tasks, progress_bar, tasks_count, num_cpus::get()).await
}

pub async fn download_file(
    client: &Client,
    url: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let parent_dir = path.parent().expect("Invalid file path");
    async_fs::create_dir_all(parent_dir).await?;
    let mut file = async_fs::File::create(path).await?;

    file.write_all(&response).await?;
    Ok(())
}

pub struct DownloadEntry {
    pub url: String,
    pub path: PathBuf,
}

pub async fn download_files<M>(
    download_entries: Vec<DownloadEntry>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let max_concurrent_downloads: usize = num_cpus::get() * 4;
    let client = Client::new();

    let total_size = download_entries.len() as u64;

    let futures = download_entries.into_iter().map(|entry| {
        let client = client.clone();
        async move { download_file(&client, &entry.url, &entry.path).await }
    });

    run_tasks_with_progress(futures, progress_bar, total_size, max_concurrent_downloads).await?;
    Ok(())
}

pub async fn fetch_file(
    client: &Client,
    url: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec())
}

pub async fn fetch_files<M>(
    urls: Vec<String>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> Result<Vec<Vec<u8>>, Box<dyn Error + Send + Sync>> {
    let max_concurrent_downloads: usize = num_cpus::get() * 4;
    let client = Client::new();

    let total_size = urls.len() as u64;

    let futures = urls.into_iter().map(|url| {
        let client = client.clone();
        async move {
            match fetch_file(&client, &url).await {
                Ok(data) => Ok(data),
                Err(e) => Err(e),
            }
        }
    });

    run_tasks_with_progress(futures, progress_bar, total_size, max_concurrent_downloads).await
}

pub struct CheckDownloadEntry {
    pub url: String,
    pub remote_sha1: Option<String>,
    pub path: PathBuf,
}

#[derive(thiserror::Error, Debug)]
pub enum CheckDownloadError {
    #[error("Hash of file {0} is missing")]
    HashMissing(PathBuf),
}

pub async fn get_download_entries<M>(
    check_entries: Vec<CheckDownloadEntry>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> Result<Vec<DownloadEntry>, Box<dyn Error + Send + Sync>> {
    let to_hash: Vec<_> = check_entries
        .iter()
        .filter_map(|entry| {
            if entry.path.exists() && entry.remote_sha1.is_some() {
                Some(entry.path.clone())
            } else {
                None
            }
        })
        .collect();

    let hashes = hash_files(to_hash.clone(), progress_bar.clone()).await?;
    let hashes = to_hash
        .into_iter()
        .zip(hashes.into_iter())
        .map(|(path, hash)| (path, hash))
        .collect::<HashMap<_, _>>();

    let mut download_entries = Vec::new();
    for entry in check_entries {
        let mut need_download = false;
        if !entry.path.exists() {
            need_download = true;
        } else if let Some(remote_sha1) = &entry.remote_sha1 {
            if remote_sha1
                != hashes
                    .get(&entry.path)
                    .ok_or(CheckDownloadError::HashMissing(entry.path.clone()))?
            {
                need_download = true;
            }
        }

        if need_download {
            download_entries.push(DownloadEntry {
                url: entry.url.clone(),
                path: entry.path.clone(),
            });
        }
    }

    Ok(download_entries)
}
