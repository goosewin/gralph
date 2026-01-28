use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}
