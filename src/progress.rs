use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio::sync::Semaphore;
use futures::future::join_all;

fn set_fancy_style(progress_bar: &indicatif::ProgressBar) {
    progress_bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{msg} {bar:40.cyan/blue} {pos}/{len} {percent}%")
            .unwrap()
            .progress_chars("=> "),
    );
}

pub trait ProgressBar: Sync + Send {
    fn set_message(&self, message: &'static str);

    fn set_length(&self, length: u64);

    fn inc(&self, amount: u64);

    fn finish(&self);
}

pub struct TerminalBarWrapper {
    bar: indicatif::ProgressBar,
}

impl TerminalBarWrapper {
    pub fn new() -> Self {
        let bar = indicatif::ProgressBar::hidden();
        set_fancy_style(&bar);
        Self { bar }
    }
}

impl ProgressBar for TerminalBarWrapper {
    fn set_message(&self, message: &'static str) {
        self.bar.set_message(message);
    }

    fn set_length(&self, length: u64) {
        self.bar.reset();
        self.bar.set_draw_target(indicatif::ProgressDrawTarget::stderr());
        self.bar.set_length(length);
    }

    fn inc(&self, amount: u64) {
        self.bar.inc(amount);
    }

    fn finish(&self) {
        self.bar.finish();
    }
}

pub type TaskFutureResult = Result<u64, Box<dyn std::error::Error + Send + Sync>>;

pub async fn run_tasks_with_progress<Fut>(tasks: impl Iterator<Item = Fut>, progress_bar: Arc<dyn ProgressBar + Send + Sync>, total_size: u64, max_concurrent_tasks: usize) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
where
    Fut: Future<Output = TaskFutureResult>,
{
    progress_bar.set_length(total_size);

    let first_error = Arc::new(Mutex::new(None));

    let semaphore = Arc::new(Semaphore::new(max_concurrent_tasks));
    let futures = tasks.map(|task| {
        let semaphore = semaphore.clone();
        let progress_bar = Arc::clone(&progress_bar);
        let first_error = Arc::clone(&first_error);

        async move {
            let _permit = semaphore.acquire().await.unwrap();
            match task.await {
                Ok(amount) => {
                    progress_bar.inc(amount);
                }
                Err(e) => {
                    let mut first_error = first_error.lock().unwrap();
                    if first_error.is_none() {
                        *first_error = Some(e);
                    }
                }
            }
        }
    });

    join_all(futures).await;
    progress_bar.finish();

    let mut first_error = first_error.lock().unwrap();
    if let Some(e) = first_error.take() {
        Err(e)
    } else {
        Ok(())
    }
}
