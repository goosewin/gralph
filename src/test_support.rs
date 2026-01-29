use std::sync::Mutex;

static ENV_LOCK: Mutex<()> = Mutex::new(());

pub fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc, Barrier};
    use std::thread;
    use std::time::Duration;

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

        assert_eq!(rx.recv_timeout(Duration::from_secs(1)), Ok(()));
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
                    recovered.fetch_add(1, Ordering::SeqCst);
                }
            }));
        }

        for handle in handles {
            assert!(handle.join().is_ok());
        }

        assert_eq!(recovered.load(Ordering::SeqCst), THREADS * ROUNDS);
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
}
