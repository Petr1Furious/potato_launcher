use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::fs;
use sha1::{Sha1, Digest};
use reqwest::Client;
use tokio::fs as async_fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use tokio::sync::mpsc::UnboundedSender;

pub fn get_files_in_dir(path: &PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.clone());
    } else if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            files.extend(get_files_in_dir(&entry.path()));
        }
    }
    files
}

async fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let data = async_fs::read(path).await?;
    Ok(format!("{:x}", Sha1::digest(&data)))
}

pub async fn hash_files(files: impl Iterator<Item = PathBuf>, tx: UnboundedSender<()>) -> HashMap<PathBuf, String> {
    let semaphore = Arc::new(Semaphore::new(num_cpus::get()));
    let hashes = Arc::new(Mutex::new(HashMap::new()));
    let mut tasks = Vec::new();

    for path in files {
        let semaphore = semaphore.clone();
        let hashes = hashes.clone();
        let thread_tx = tx.clone();

        tasks.push(tokio::spawn(async move {
            let permit = semaphore.acquire_owned().await.unwrap();
            let hash = hash_file(&path).await.unwrap();
            let mut hashes = hashes.lock().unwrap();
            hashes.insert(path, hash);
            thread_tx.send(()).unwrap();
            drop(permit);
        }));
    }

    futures::future::join_all(tasks).await;
    let hashes = Arc::try_unwrap(hashes).expect("Arc unwrap failed").into_inner();
    hashes.unwrap()
}

async fn download_file(client: &Client, url: &str, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.get(url).send().await?.bytes().await?;

    let parent_dir = path.parent().expect("Invalid file path");
    async_fs::create_dir_all(parent_dir).await?;
    let mut file = async_fs::File::create(path).await?;

    file.write_all(&response).await?;
    Ok(())
}

pub async fn download_files(urls: impl Iterator<Item = String>, paths: impl Iterator<Item = PathBuf>, tx: UnboundedSender<()>) {
    const MAX_CONCURRENT_DOWNLOADS: usize = 10;

    let client = Client::new();
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_DOWNLOADS));

    let tasks: Vec<_> = urls.into_iter().zip(paths.into_iter()).map(|(url, path)| {
        let client = client.clone();
        let semaphore = semaphore.clone();
        let thread_tx = tx.clone();
        tokio::spawn(async move {
            let permit = semaphore.acquire_owned().await.unwrap();
            if let Err(e) = download_file(&client, &url, &path).await {
                panic!("Failed to download {}: {:?}", url, e);
            }
            thread_tx.send(()).unwrap();
            drop(permit);
        })
    }).collect();

    futures::future::join_all(tasks).await;
}
