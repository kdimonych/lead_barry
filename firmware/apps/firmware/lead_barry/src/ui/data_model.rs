use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

pub type SharedDataModel<T> = Mutex<CriticalSectionRawMutex, T>;
