#![no_std]
#![feature(asm)]

#[cfg(feature = "AARCH64")]
extern crate alloc;

#[cfg(feature = "AARCH64")]
pub mod lfstack;

pub mod mcs;

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod tests {
    use crate::mcs;
    use std::sync::Arc;
    use std::vec::Vec;

    const NUM_LOOP: usize = 1000000;
    const NUM_THREADS: usize = 4;

    #[test]
    fn test_mcs() {
        let n = Arc::new(mcs::MCSLock::new(0));
        let mut v = Vec::new();

        for _ in 0..NUM_THREADS {
            let n0 = n.clone();
            let t = std::thread::spawn(move || {
                for _ in 0..NUM_LOOP {
                    let mut r = n0.lock();
                    *r += 1;
                }
            });

            v.push(t);
        }

        for t in v {
            t.join().unwrap();
        }

        let r = n.lock();
        assert_eq!(NUM_LOOP * NUM_THREADS, *r);
    }

    #[cfg(feature = "AARCH64")]
    #[test]
    fn test_lfstack() {
        use crate::lfstack;
        let stack = Arc::new(lfstack::LFStack::<usize>::new());
        let mut v = Vec::new();

        for i in 0..NUM_THREADS {
            let stack0 = stack.clone();
            let t = std::thread::spawn(move || {
                if i & 1 == 0 {
                    for j in 0..NUM_LOOP {
                        let k = i * NUM_LOOP + j;
                        stack0.get_mut().push(k);
                    }
                } else {
                    for _ in 0..NUM_LOOP {
                        loop {
                            if let Some(k) = stack0.get_mut().pop() {
                                break;
                            }
                        }
                    }
                }
            });
            v.push(t);
        }

        for t in v {
            t.join().unwrap();
        }

        assert_eq!(stack.get_mut().pop(), None);
    }
}
