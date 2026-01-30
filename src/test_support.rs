use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    fn set_env(key: &str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe {
            env::set_var(key, value);
        }
    }

    fn remove_env(key: &str) {
        unsafe {
            env::remove_var(key);
        }
    }

    #[test]
    fn env_lock_is_usable_after_panic_in_prior_holder() {
        let handle = thread::spawn(|| {
            let _guard = env_lock();
            panic!("poison env lock");
        });

        assert!(handle.join().is_err());

        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let _guard = env_lock();
            tx.send(()).expect("send acquired");
        });

        assert_eq!(rx.recv_timeout(Duration::from_secs(5)), Ok(()));
        assert!(handle.join().is_ok());
    }

    #[test]
    fn env_lock_serializes_access_under_contention() {
        const THREADS: usize = 4;
        let barrier = Arc::new(Barrier::new(THREADS));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(THREADS);

        for _ in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let _guard = env_lock();
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                loop {
                    let current = max_active.load(Ordering::SeqCst);
                    if now <= current {
                        break;
                    }
                    if max_active
                        .compare_exchange(current, now, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(10));
                active.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn env_lock_releases_under_contention_and_allows_reacquire() {
        const WAITERS: usize = 2;
        let barrier = Arc::new(Barrier::new(WAITERS + 1));
        let (tx, rx) = mpsc::channel();
        let mut handles = Vec::with_capacity(WAITERS);

        for id in 0..WAITERS {
            let barrier = Arc::clone(&barrier);
            let tx = tx.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                let _guard = env_lock();
                tx.send(id).expect("send acquired");
                thread::sleep(Duration::from_millis(5));
            }));
        }

        let guard = env_lock();
        barrier.wait();
        assert_eq!(
            rx.recv_timeout(Duration::from_millis(25)),
            Err(mpsc::RecvTimeoutError::Timeout)
        );
        drop(guard);

        let mut acquired = Vec::with_capacity(WAITERS);
        for _ in 0..WAITERS {
            acquired.push(rx.recv_timeout(Duration::from_secs(1)).unwrap());
        }
        acquired.sort();
        assert_eq!(acquired, vec![0, 1]);

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        let _guard = env_lock();
    }

    #[test]
    fn env_lock_serializes_env_updates() {
        const THREADS: usize = 4;
        let barrier = Arc::new(Barrier::new(THREADS));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(THREADS);

        for thread_id in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            handles.push(thread::spawn(move || {
                barrier.wait();
                let _guard = env_lock();
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                loop {
                    let current = max_active.load(Ordering::SeqCst);
                    if now <= current {
                        break;
                    }
                    if max_active
                        .compare_exchange(current, now, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        break;
                    }
                }
                let value = format!("thread-{thread_id}");
                set_env("GRALPH_ENV_LOCK_SERIALIZE_TEST", &value);
                thread::sleep(Duration::from_millis(5));
                assert_eq!(
                    env::var("GRALPH_ENV_LOCK_SERIALIZE_TEST").as_deref(),
                    Ok(value.as_str())
                );
                active.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
        let _guard = env_lock();
        remove_env("GRALPH_ENV_LOCK_SERIALIZE_TEST");
    }

    #[test]
    fn env_lock_supports_safe_env_restore() {
        let _guard = env_lock();
        let key = "GRALPH_ENV_LOCK_SAFE_TEST";
        let original = env::var_os(key);

        set_env(key, "temporary-value");
        assert_eq!(env::var(key).as_deref(), Ok("temporary-value"));

        if let Some(value) = &original {
            set_env(key, value);
        } else {
            remove_env(key);
        }

        assert_eq!(env::var_os(key), original);
    }

    #[test]
    fn env_lock_preserves_env_after_guard_drop() {
        let key = "GRALPH_ENV_LOCK_DROP_RESTORE_TEST";
        let original = {
            let _guard = env_lock();
            env::var_os(key)
        };

        {
            let _guard = env_lock();
            set_env(key, "temporary-value");
            assert_eq!(env::var(key).as_deref(), Ok("temporary-value"));
            if let Some(value) = &original {
                set_env(key, value);
            } else {
                remove_env(key);
            }
        }

        let _guard = env_lock();
        assert_eq!(env::var_os(key), original);
    }

    #[test]
    fn env_lock_reacquires_after_drop_in_same_thread() {
        let key = "GRALPH_ENV_LOCK_REACQUIRE_TEST";
        let original = {
            let _guard = env_lock();
            env::var_os(key)
        };

        {
            let _guard = env_lock();
            set_env(key, "first");
            assert_eq!(env::var(key).as_deref(), Ok("first"));
        }

        {
            let _guard = env_lock();
            set_env(key, "second");
            assert_eq!(env::var(key).as_deref(), Ok("second"));
            if let Some(value) = &original {
                set_env(key, value);
            } else {
                remove_env(key);
            }
        }

        let _guard = env_lock();
        assert_eq!(env::var_os(key), original);
    }

    #[test]
    fn env_lock_allows_sequential_acquisition_single_thread() {
        let key = "GRALPH_ENV_LOCK_SEQUENCE_TEST";
        let original = {
            let _guard = env_lock();
            env::var_os(key)
        };

        for idx in 0..3 {
            let _guard = env_lock();
            let value = format!("seq-{idx}");
            set_env(key, &value);
            assert_eq!(env::var(key).as_deref(), Ok(value.as_str()));
        }

        let _guard = env_lock();
        if let Some(value) = &original {
            set_env(key, value);
        } else {
            remove_env(key);
        }
        assert_eq!(env::var_os(key), original);
    }

    #[test]
    fn env_lock_reacquires_after_multiple_sequential_drops() {
        let key = "GRALPH_ENV_LOCK_MULTI_DROP_TEST";
        let original = {
            let _guard = env_lock();
            env::var_os(key)
        };
        for idx in 0..5 {
            let value = format!("multi-drop-{idx}");
            {
                let _guard = env_lock();
                set_env(key, &value);
                assert_eq!(env::var(key).as_deref(), Ok(value.as_str()));
            }

            {
                let _guard = env_lock();
                assert_eq!(env::var(key).as_deref(), Ok(value.as_str()));
            }
        }

        let _guard = env_lock();
        if let Some(value) = &original {
            set_env(key, value);
        } else {
            remove_env(key);
        }
        assert_eq!(env::var_os(key), original);
    }

    #[test]
    fn env_lock_recovers_after_repeated_panics_across_threads() {
        const THREADS: usize = 6;
        const ROUNDS: usize = 4;
        let barrier = Arc::new(Barrier::new(THREADS));
        let recovered = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(THREADS);

        for thread_id in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            let recovered = Arc::clone(&recovered);
            handles.push(thread::spawn(move || {
                barrier.wait();
                for round in 0..ROUNDS {
                    let result = std::panic::catch_unwind(|| {
                        let _guard = env_lock();
                        panic!("poison env lock {thread_id}-{round}");
                    });
                    assert!(result.is_err());

                    let _guard = env_lock();
                    let value = format!("{thread_id}-{round}");
                    set_env("GRALPH_ENV_LOCK_POISON_RECOVER_TEST", &value);
                    assert_eq!(
                        env::var("GRALPH_ENV_LOCK_POISON_RECOVER_TEST").as_deref(),
                        Ok(value.as_str())
                    );
                    recovered.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        assert_eq!(recovered.load(Ordering::SeqCst), THREADS * ROUNDS);
        let _guard = env_lock();
        remove_env("GRALPH_ENV_LOCK_POISON_RECOVER_TEST");
    }

    #[test]
    fn env_lock_restores_env_after_panic_in_guarded_scope() {
        let key = "GRALPH_ENV_LOCK_PANIC_SCOPE_TEST";
        let original = {
            let _guard = env_lock();
            env::var_os(key)
        };

        let key_owned = key.to_string();
        let handle = thread::spawn(move || {
            let _guard = env_lock();
            set_env(&key_owned, "panic-scope");
            panic!("panic in guarded scope");
        });

        assert!(handle.join().is_err());

        let _guard = env_lock();
        assert_eq!(env::var(key).as_deref(), Ok("panic-scope"));
        if let Some(value) = &original {
            set_env(key, value);
        } else {
            remove_env(key);
        }
        assert_eq!(env::var_os(key), original);
    }

    #[test]
    fn env_lock_never_allows_parallel_access_under_high_contention() {
        const THREADS: usize = 16;
        const ITERATIONS: usize = 20;
        let barrier = Arc::new(Barrier::new(THREADS));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(THREADS);

        for _ in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            handles.push(thread::spawn(move || {
                barrier.wait();
                for _ in 0..ITERATIONS {
                    let _guard = env_lock();
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    loop {
                        let current = max_active.load(Ordering::SeqCst);
                        if now <= current {
                            break;
                        }
                        if max_active
                            .compare_exchange(current, now, Ordering::SeqCst, Ordering::SeqCst)
                            .is_ok()
                        {
                            break;
                        }
                    }
                    thread::sleep(Duration::from_millis(1));
                    active.fetch_sub(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn env_lock_enforces_serialization_under_contention() {
        const THREADS: usize = 6;
        const ITERATIONS: usize = 8;
        let barrier = Arc::new(Barrier::new(THREADS));
        let active = Arc::new(AtomicUsize::new(0));
        let overlaps = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(THREADS);

        for _ in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            let active = Arc::clone(&active);
            let overlaps = Arc::clone(&overlaps);
            handles.push(thread::spawn(move || {
                barrier.wait();
                for _ in 0..ITERATIONS {
                    let _guard = env_lock();
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    if now > 1 {
                        overlaps.fetch_add(1, Ordering::SeqCst);
                    }
                    thread::sleep(Duration::from_millis(1));
                    active.fetch_sub(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        assert_eq!(overlaps.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn env_lock_recovers_and_restores_env_after_panic() {
        let key = "GRALPH_ENV_LOCK_PANIC_RESTORE_TEST";
        let original = {
            let _guard = env_lock();
            env::var_os(key)
        };

        let result = std::panic::catch_unwind(|| {
            let _guard = env_lock();
            set_env(key, "poisoned");
            panic!("panic while holding env lock");
        });

        assert!(result.is_err());

        let _guard = env_lock();
        if let Some(value) = &original {
            set_env(key, value);
        } else {
            remove_env(key);
        }
        assert_eq!(env::var_os(key), original);
    }

    #[test]
    fn env_lock_serializes_env_restore_under_contention() {
        const THREADS: usize = 8;
        let key = "GRALPH_ENV_LOCK_RESTORE_CONTEND_TEST";
        let original = {
            let _guard = env_lock();
            env::var_os(key)
        };

        let barrier = Arc::new(Barrier::new(THREADS));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(THREADS);

        for thread_id in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            let original = original.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                let _guard = env_lock();
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                loop {
                    let current = max_active.load(Ordering::SeqCst);
                    if now <= current {
                        break;
                    }
                    if max_active
                        .compare_exchange(current, now, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                    {
                        break;
                    }
                }
                let value = format!("contended-{thread_id}");
                set_env(key, &value);
                assert_eq!(env::var(key).as_deref(), Ok(value.as_str()));
                if let Some(value) = &original {
                    set_env(key, value);
                } else {
                    remove_env(key);
                }
                assert_eq!(env::var_os(key), original);
                thread::sleep(Duration::from_millis(2));
                active.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        assert_eq!(max_active.load(Ordering::SeqCst), 1);
        let _guard = env_lock();
        assert_eq!(env::var_os(key), original);
    }

    #[test]
    fn env_lock_restores_env_before_next_thread_runs() {
        const THREADS: usize = 6;
        let key = "GRALPH_ENV_LOCK_RESTORE_SEQUENCE_TEST";
        let original = {
            let _guard = env_lock();
            env::var_os(key)
        };

        let barrier = Arc::new(Barrier::new(THREADS));
        let mismatches = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::with_capacity(THREADS);

        for thread_id in 0..THREADS {
            let barrier = Arc::clone(&barrier);
            let mismatches = Arc::clone(&mismatches);
            let original = original.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                let _guard = env_lock();
                if env::var_os(key) != original {
                    mismatches.fetch_add(1, Ordering::SeqCst);
                }
                let value = format!("sequence-{thread_id}");
                set_env(key, &value);
                assert_eq!(env::var(key).as_deref(), Ok(value.as_str()));
                if let Some(value) = &original {
                    set_env(key, value);
                } else {
                    remove_env(key);
                }
                assert_eq!(env::var_os(key), original);
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        assert_eq!(mismatches.load(Ordering::SeqCst), 0);
        let _guard = env_lock();
        assert_eq!(env::var_os(key), original);
    }
}
