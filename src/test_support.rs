use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[derive(Debug, PartialEq)]
    enum Event {
        Attempting,
        Acquired,
    }

    #[test]
    fn env_lock_recovers_from_poisoned_mutex() {
        let handle = thread::spawn(|| {
            let _guard = env_lock();
            panic!("poison env lock");
        });

        assert!(handle.join().is_err());

        let result = std::panic::catch_unwind(|| {
            let _guard = env_lock();
        });

        assert!(result.is_ok());
    }

    #[test]
    fn env_lock_serializes_access() {
        let guard = env_lock();
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            tx.send(Event::Attempting).expect("send attempting");
            let _guard = env_lock();
            tx.send(Event::Acquired).expect("send acquired");
        });

        assert_eq!(rx.recv(), Ok(Event::Attempting));
        assert!(matches!(
            rx.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));

        drop(guard);

        assert_eq!(
            rx.recv_timeout(Duration::from_secs(1)),
            Ok(Event::Acquired)
        );
        assert!(handle.join().is_ok());
    }
}
