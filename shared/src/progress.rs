use futures::future::join_all;
use std::error::Error;
use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct Unit {
    pub name: String,
    pub size: u64,
}

pub trait ProgressBar<M>: Sync + Send {
    fn set_message(&self, message: M);

    fn set_length(&self, length: u64);

    fn inc(&self, amount: u64);

    fn finish(&self);

    fn reset(&self) {
        self.set_length(0);
    }

    fn set_unit(&self, unit: Unit);
}

pub struct NoProgressBar;

impl<M> ProgressBar<M> for NoProgressBar {
    fn set_message(&self, _message: M) {}

    fn set_length(&self, _length: u64) {}

    fn inc(&self, _amount: u64) {}

    fn finish(&self) {}

    fn set_unit(&self, _unit: Unit) {}
}

pub fn no_progress_bar() -> Arc<dyn ProgressBar<i32> + Send + Sync> {
    Arc::new(NoProgressBar)
}

pub async fn run_tasks_with_progress<M, T, Fut>(
    tasks: impl Iterator<Item = Fut>,
    progress_bar: Arc<dyn ProgressBar<M> + Send + Sync>,
    total_size: u64,
    max_concurrent_tasks: usize,
) -> Result<Vec<T>, Box<dyn std::error::Error + Send + Sync>>
where
    Fut: Future<Output = Result<T, Box<dyn Error + Send + Sync>>>,
{
    progress_bar.set_length(total_size);

    let first_error = Arc::new(Mutex::new(None));
    let cancellation_token = CancellationToken::new();

    let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));
    let futures = tasks.map(|task| {
        let semaphore = semaphore.clone();
        let progress_bar = Arc::clone(&progress_bar);
        let first_error = Arc::clone(&first_error);
        let cancellation_token = cancellation_token.clone();

        async move {
            let _permit = semaphore.acquire().await.unwrap();

            if first_error.lock().unwrap().is_some() {
                return None;
            }

            match task.await {
                Ok(result) => {
                    progress_bar.inc(1);
                    Some(result)
                }
                Err(e) => {
                    let mut first_error = first_error.lock().unwrap();
                    if first_error.is_none() {
                        *first_error = Some(e);
                        cancellation_token.cancel();
                    }
                    None
                }
            }
        }
    });

    tokio::select! {
        results = join_all(futures) => {
            progress_bar.finish();
            let mut first_error = first_error.lock().unwrap();
            if let Some(e) = first_error.take() {
                Err(e)
            } else {
                let results: Result<Vec<_>, _> = results.into_iter().map(|x| {
                    x.ok_or_else(|| "Task failed but no error was set".into())
                }).collect();
                results
            }
        }
        _ = cancellation_token.cancelled() => {
            progress_bar.finish();
            let mut first_error = first_error.lock().unwrap();
            if let Some(e) = first_error.take() {
                Err(e)
            } else {
                Err("Got cancelled but no error was set".into())
            }
        }
    }
}
