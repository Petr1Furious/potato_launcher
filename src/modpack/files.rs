use reqwest::Client;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;

use crate::progress::{run_tasks_with_progress, ProgressBar, TaskFutureResult};

pub fn get_files_in_dir(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            files.extend(get_files_in_dir(&entry.path()).into_iter().map(|x| x.to_path_buf()));
        }
    }
    files
}

async fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let data = async_fs::read(path).await?;
    Ok(format!("{:x}", Sha1::digest(&data)))
}

pub async fn hash_files(
    files: impl Iterator<Item = PathBuf>,
    progress_bar: Arc<dyn ProgressBar + Send + Sync>,
) -> Result<HashMap<PathBuf, String>, Box<dyn std::error::Error + Send + Sync>> {
    let hashes = Arc::new(Mutex::new(HashMap::new()));

    let files: Vec<PathBuf> = files.collect();
    let tasks_count = files.len() as u64;

    let tasks = files.into_iter().map(|path| {
        let hashes = Arc::clone(&hashes);

        async move {
            let result = hash_file(&path).await;
            match result {
                Ok(hash) => {
                    let mut hashes = hashes.lock().unwrap();
                    hashes.insert(path, hash);
                    TaskFutureResult::Ok(1)
                }
                Err(e) => TaskFutureResult::Err(e.into()),
            }
        }
    });

    run_tasks_with_progress(tasks, progress_bar, tasks_count, num_cpus::get()).await?;

    let hashes = Arc::try_unwrap(hashes)
        .expect("Arc unwrap failed")
        .into_inner();
    Ok(hashes.unwrap())
}

async fn download_file(
    client: &Client,
    url: &str,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let response = client.get(url).send().await?.bytes().await?;

    let parent_dir = path.parent().expect("Invalid file path");
    async_fs::create_dir_all(parent_dir).await?;
    let mut file = async_fs::File::create(path).await?;

    file.write_all(&response).await?;
    Ok(())
}

pub async fn download_files(
    urls: impl Iterator<Item = String>,
    paths: impl Iterator<Item = PathBuf>,
    progress_bar: Arc<dyn ProgressBar + Send + Sync>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    const MAX_CONCURRENT_DOWNLOADS: usize = 10;
    let client = Client::new();

    let urls: Vec<String> = urls.collect();
    let total_size = urls.len() as u64;

    let futures = urls.into_iter().zip(paths).map(|(url, path)| {
        let client = client.clone();
        async move {
            if let Err(e) = download_file(&client, &url, &path).await {
                return TaskFutureResult::Err(e);
            }
            Ok(1)
        }
    });

    run_tasks_with_progress(futures, progress_bar, total_size, MAX_CONCURRENT_DOWNLOADS).await
}
