use loom::{sync::Arc, thread};
use synctools::mcs::{MCSLock, MCSNode};

#[test]
fn model_check_mcslock() {
    loom::model(|| {
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
                        *guard += 1;
                    }
                })
            })
            .collect();

        for thread in threads {
            thread.join().unwrap();
        }

        let mut node = MCSNode::new();
        assert_eq!(num_threads * num_iterations, *lock.lock(&mut node));
    });
}
