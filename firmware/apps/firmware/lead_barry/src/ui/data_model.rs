use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

pub type DataModel<T> = Mutex<CriticalSectionRawMutex, T>;
