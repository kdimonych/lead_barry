use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

pub type DataModel<T> = Mutex<ThreadModeRawMutex, T>;
