/// # How to test
///
/// `RUST_BACKTRACE=1 RUSTFLAGS="--cfg loom"  cargo test --test test_mcslock --release`
#[cfg(loom)]
#[test]
fn model_check_mcslock() {
    loom::model(|| {
        #[cfg(loom)]
        use loom::{sync::Arc, thread};

        #[cfg(loom)]
        use synctools::mcs::{MCSLock, MCSNode};

        let lock = Arc::new(MCSLock::new(0));
        let num_threads = 2;
        let num_iterations = 2;

        let threads: Vec<_> = (0..num_threads)
            .map(|_| {
                let lock = lock.clone();
                thread::spawn(move || {
                    for _ in 0..num_iterations {
                        let mut node = MCSNode::new();
                        let mut guard = lock.lock(&mut node);
                        guard.with_mut(|data| unsafe {
                            *data += 1;
                        })
                    }
                })
            })
            .collect();

        for thread in threads {
            thread.join().unwrap();
        }

        let mut node = MCSNode::new();
        let data = lock.lock(&mut node).with_mut(|data| unsafe { *data });

        assert_eq!(num_threads * num_iterations, data);
    });
}
