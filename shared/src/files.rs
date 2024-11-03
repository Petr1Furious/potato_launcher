use reqwest::Client;
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt};
use walkdir::WalkDir;

use crate::progress::{run_tasks_with_progress, ProgressBar};
use crate::utils::BoxResult;

pub fn get_files_in_dir(path: &Path) -> BoxResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    if path.is_file() {
        files.push(path.to_path_buf());
    } else if path.is_dir() {
        let entries = std::fs::read_dir(path)?;
        for entry in entries.flatten() {
            files.extend(get_files_in_dir(&entry.path())?);
        }
    }
    Ok(files)
}

pub async fn hash_file(path: &Path) -> BoxResult<String> {
    let mut file = fs::File::open(path).await?;
    let mut hasher = Sha1::new();
    let mut buffer = [0; 1024];

    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

pub async fn hash_files<M>(
    files: Vec<PathBuf>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> BoxResult<Vec<String>> {
    let tasks_count = files.len() as u64;

    let tasks = files
        .into_iter()
        .map(|path| async move { hash_file(&path).await });

    run_tasks_with_progress(tasks, progress_bar, tasks_count, num_cpus::get()).await
}

pub async fn download_file(client: &Client, url: &str, path: &Path) -> BoxResult<()> {
    let response = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let parent_dir = path.parent().expect("Invalid file path");
    tokio::fs::create_dir_all(parent_dir).await?;
    let mut file = tokio::fs::File::create(path).await?;

    file.write_all(&response).await?;
    Ok(())
}

#[derive(Debug)]
pub struct DownloadEntry {
    pub url: String,
    pub path: PathBuf,
}

pub async fn download_files<M>(
    download_entries: Vec<DownloadEntry>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> BoxResult<()> {
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

pub async fn fetch_file(client: &Client, url: &str) -> BoxResult<Vec<u8>> {
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
) -> BoxResult<Vec<Vec<u8>>> {
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

#[derive(Debug)]
pub struct CheckEntry {
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
    check_entries: Vec<CheckEntry>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
) -> BoxResult<Vec<DownloadEntry>> {
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

async fn remove_empty_dirs(path: &Path) -> BoxResult<()> {
    for entry in WalkDir::new(path)
        .contents_first(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().is_dir()
            && fs::read_dir(entry.path())
                .await?
                .next_entry()
                .await?
                .is_none()
        {
            fs::remove_dir(entry.path()).await?;
        }
    }
    Ok(())
}

#[derive(thiserror::Error, Debug)]
pub enum CopyFilesError {
    #[error("Source entry {0} does not exist")]
    SourceEntryMissing(PathBuf),
    #[error("Invalid path")]
    InvalidPath,
}

// copy files mapped files and directories
// and delete all other files and directores in the target directory
// mapping: target -> source
pub async fn sync_mapping(target_dir: &Path, mapping: &HashMap<PathBuf, PathBuf>) -> BoxResult<()> {
    let mut mappings_files = HashMap::new();
    for (target, source) in mapping {
        if !target.starts_with(target_dir) {
            return Err(CopyFilesError::InvalidPath.into());
        }
        if source.is_file() {
            mappings_files.insert(target.clone(), source.clone());
        } else if source.is_dir() {
            let files = get_files_in_dir(&source)?;
            for file in files {
                let relative_path = file.strip_prefix(&source).unwrap();
                let target_path = target.join(relative_path);
                mappings_files.insert(target_path, file);
            }
        } else {
            return Err(CopyFilesError::SourceEntryMissing(source.clone()).into());
        }
    }

    let paths = get_files_in_dir(&target_dir)?;
    for path in paths {
        if !mappings_files.contains_key(&path) {
            fs::remove_file(&path).await?;
        }
    }

    remove_empty_dirs(&target_dir).await?;

    let fut = mappings_files.iter().map(|(target, source)| async move {
        fs::create_dir_all(target.parent().ok_or(CopyFilesError::InvalidPath)?).await?;
        if target.is_dir() {
            fs::remove_dir(&target).await?;
        }
        if !target.exists() || hash_file(&source).await? != hash_file(&target).await? {
            fs::copy(&source, &target).await?;
        }
        BoxResult::<()>::Ok(())
    });

    let results = futures::future::join_all(fut).await;
    for result in results {
        result?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;

    use maplit::hashmap;

    use super::*;

    #[tokio::test]
    async fn test_sync_mapping() {
        let temp_dir = env::temp_dir().join("instance_builder_test");
        let source_dir = temp_dir.join("source");
        let target_dir = temp_dir.join("target");
        let file1 = source_dir.join("file1");
        let dir1 = source_dir.join("dir1");
        let file2 = dir1.join("file2");
        let dir2 = source_dir.join("dir2");
        let file3 = dir2.join("file3");

        let file1_target = target_dir.join("file1");
        let file4 = target_dir.join("file4");
        let dir1_target = target_dir.join("dir1");
        let file2_target = dir1_target.join("file2");
        let file5 = dir1_target.join("file5");

        fs::create_dir_all(&dir1).await.unwrap();
        fs::create_dir_all(&dir2).await.unwrap();
        fs::create_dir_all(&dir1_target).await.unwrap();
        fs::write(&file1, "file1").await.unwrap();
        fs::write(&file2, "file2").await.unwrap();
        fs::write(&file3, "file3").await.unwrap();
        fs::write(&file1_target, "file1_other").await.unwrap();
        fs::write(&file4, "file4").await.unwrap();
        fs::write(&file2_target, "file2").await.unwrap();
        fs::write(&file5, "file5").await.unwrap();

        let mappings = hashmap! {
            file1_target.clone() => file1.clone(),
            file2_target.clone() => file2.clone(),
            target_dir.join("dir2") => dir2.clone(),
        };

        sync_mapping(&target_dir, &mappings).await.unwrap();

        assert!(file1_target.exists());
        assert!(fs::read_to_string(&file1_target).await.unwrap() == "file1");
        assert!(file2_target.exists());
        assert!(fs::read_to_string(&file2_target).await.unwrap() == "file2");
        assert!(target_dir.join("dir2").join("file3").exists());
        assert!(!file4.exists());
        assert!(!file5.exists());

        fs::remove_dir_all(&source_dir).await.unwrap();
        fs::remove_dir_all(&target_dir).await.unwrap();
    }
}
