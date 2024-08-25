use std::sync::mpsc;

pub struct Task<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> Task<T> {
    pub fn new(receiver: mpsc::Receiver<T>) -> Self {
        Task::<T> { receiver }
    }

    pub fn take_result(&self) -> Option<T> {
        if let Ok(result) = self.receiver.try_recv() {
            Some(result)
        } else {
            None
        }
    }
}
