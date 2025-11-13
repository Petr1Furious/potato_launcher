use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

pub enum BackgroundTaskResult<T> {
    Finished(T),
    Cancelled,
}

pub struct BackgroundTask<T>
where
    T: Send + 'static,
{
    result: Arc<Mutex<Option<BackgroundTaskResult<T>>>>, // nice
    cancellation_token: CancellationToken,
}

impl<T> BackgroundTask<T>
where
    T: Send + 'static,
{
    pub fn with_callback<F>(
        future: F,
        runtime: &tokio::runtime::Runtime,
        callback: Box<dyn FnOnce() + Send>,
    ) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        let result = Arc::new(Mutex::new(None));
        let cancellation_token = CancellationToken::new();

        let result_clone = result.clone();
        let cancellation_token_clone = cancellation_token.clone();

        runtime.spawn(async move {
            tokio::select! {
                res = future => {
                    let mut result_lock = result_clone.lock().unwrap();
                    match *result_lock {
                        Some(BackgroundTaskResult::Cancelled) => {
                        }
                        _ => {
                            *result_lock = Some(BackgroundTaskResult::Finished(res));
                        }
                    }
                    callback();
                }
                _ = cancellation_token_clone.cancelled() => {
                    let mut result_lock = result_clone.lock().unwrap();
                    *result_lock = Some(BackgroundTaskResult::Cancelled);
                    callback();
                }
            }
        });

        BackgroundTask {
            result,
            cancellation_token,
        }
    }

    pub fn has_result(&self) -> bool {
        self.result.lock().unwrap().is_some()
    }

    pub fn take_result(self) -> BackgroundTaskResult<T> {
        let mut result_lock = self.result.lock().unwrap();
        result_lock
            .take()
            .expect("Check has_result before calling take_result")
    }

    pub fn cancel(&mut self) {
        let mut result_lock = self.result.lock().unwrap();
        *result_lock = Some(BackgroundTaskResult::Cancelled);
        self.cancellation_token.cancel();
    }
}

impl<T> Drop for BackgroundTask<T>
where
    T: Send + 'static,
{
    fn drop(&mut self) {
        self.cancel();
    }
}
