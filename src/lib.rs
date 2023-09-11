//! # Synchronization Tools for no_std environments in Rust
//!
//! ## MCS Lock
//!
//! MCS lock is a fair and scalable mutual lock.
//! This can be used as std::sync::Mutex.
//!
//! ```rust
//! use crate::synctools::mcs;
//! use std::sync::Arc;
//! use std::vec::Vec;
//!
//! const NUM_LOOP: usize = 1000000;
//! const NUM_THREADS: usize = 4;
//!
//! fn main() {
//!     // create a MCSLock object
//!     let n = Arc::new(mcs::MCSLock::new(0));
//!     let mut v = Vec::new();
//!
//!     for _ in 0..NUM_THREADS {
//!         let n0 = n.clone();
//!         let t = std::thread::spawn(move || {
//!         let mut node = mcs::MCSNode::new();
//!             for _ in 0..NUM_LOOP {
//!                 // lock and acquire the reference
//!                 let mut r = n0.lock(&mut node);
//!
//!                 // increment atomically
//!                 *r += 1;
//!             }
//!         });
//!
//!         v.push(t);
//!     }
//!
//!     for t in v {
//!         t.join().unwrap();
//!     }
//!
//!     let mut node = mcs::MCSNode::new();
//!     let r = n.lock(&mut node);
//!     assert_eq!(NUM_LOOP * NUM_THREADS, *r);
//! }
//! ```
//!
//! ## Readers Writer Lock
//!
//! Spin lock based readers writer lock can be used as std::sync:RwLock.
//!
//! ```rust
//! use crate::synctools::rwlock;
//! use std::sync::Arc;
//! use std::vec::Vec;
//!
//! const NUM_LOOP: usize = 1000000;
//! const NUM_THREADS: usize = 4;
//!
//! fn main() {
//!     // create a RwLock object
//!     let n = Arc::new(rwlock::RwLock::new(0));
//!     let mut v = Vec::new();
//!
//!     // reader
//!     for _ in 0..(NUM_THREADS - 1) {
//!         let n0 = n.clone();
//!         let t = std::thread::spawn(move || {
//!             for _ in 0..NUM_LOOP {
//!                 // read lock
//!                 let r = n0.read();
//!                 assert_eq!(*r, 0);
//!             }
//!         });
//!
//!         v.push(t);
//!     }
//!
//!     // writer
//!     let n0 = n.clone();
//!     let wr = std::thread::spawn(move || {
//!         for _ in 0..NUM_LOOP {
//!             {
//!                 // write lock
//!                 let mut r = n0.write();
//!                 *r += 1;
//!                 *r -= 1;
//!             }
//!         }
//!     });
//!
//!     v.push(wr);
//!
//!     for t in v {
//!         t.join().unwrap();
//!     }
//! }
//! ```
//!
//! ## Lock Free Stack (AArch64 only)
//!
//! Lock free stack is a concurrent data structure.
//! This can be used only AArch64 and nightly because this
//! uses LL/SC instructions in inline assembly internally.
//!
//! ```rust
//! #[cfg(target_arch = "aarch64")]
//! use crate::synctools::lfstack;
//! use std::sync::Arc;
//! use std::vec::Vec;
//!
//! const NUM_LOOP: usize = 1000000;
//! const NUM_THREADS: usize = 4;
//!
//! #[cfg(target_arch = "aarch64")]
//! fn main() {
//!     // create a stack
//!     let mut stack = Arc::new(lfstack::LFStack::<usize>::new());
//!     let mut v = Vec::new();
//!
//!     for i in 0..NUM_THREADS {
//!         let mut stack0 = stack.clone();
//!         let t = std::thread::spawn(move || {
//!             if i & 1 == 0 { // even thread
//!                 for j in 0..NUM_LOOP {
//!                     let k = i * NUM_LOOP + j;
//!                     // push k to the stack
//!                     stack0.get_mut().push(k);
//!                 }
//!             } else { // odd thread
//!                 for _ in 0..NUM_LOOP {
//!                     loop {
//!                         // pop from the stack
//!                         if let Some(k) = stack0.get_mut().pop() {
//!                             break;
//!                         }
//!                     }
//!                 }
//!             }
//!         });
//!         v.push(t);
//!     }
//!
//!     for t in v {
//!         t.join().unwrap();
//!     }
//!
//!     assert_eq!(stack.get_mut().pop(), None);
//! }
//!
//! #[cfg(not(target_arch = "aarch64"))]
//! fn main() {}
//! ```

#![no_std]

#[cfg(target_arch = "aarch64")]
extern crate alloc;

#[cfg(target_arch = "aarch64")]
pub mod lfstack;

pub mod mcs;
pub mod rwlock;

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod tests {
    use crate::mcs;
    use crate::rwlock;
    use std::sync::Arc;
    use std::vec::Vec;

    const NUM_LOOP: usize = 10000000;
    const NUM_THREADS: usize = 8;

    #[test]
    fn test_mcs() {
        let n = Arc::new(mcs::MCSLock::new(0));
        let mut v = Vec::new();

        for _ in 0..NUM_THREADS {
            let n0 = n.clone();
            let mut node = mcs::MCSNode::new();
            let t = std::thread::spawn(move || {
                for _ in 0..NUM_LOOP {
                    let mut r = n0.lock(&mut node);
                    *r += 1;
                }
            });

            v.push(t);
        }

        for t in v {
            t.join().unwrap();
        }

        let mut node = mcs::MCSNode::new();
        let r = n.lock(&mut node);
        assert_eq!(NUM_LOOP * NUM_THREADS, *r);
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn test_lfstack() {
        use crate::lfstack;
        let mut stack = Arc::new(lfstack::LFStack::<usize>::new());
        let mut v = Vec::new();

        for i in 0..NUM_THREADS {
            let mut stack0 = stack.clone();
            let t = std::thread::spawn(move || {
                if i & 1 == 0 {
                    for j in 0..NUM_LOOP {
                        let k = i * NUM_LOOP + j;
                        stack0.get_mut().push(k);
                    }
                } else {
                    for _ in 0..NUM_LOOP {
                        loop {
                            if let Some(_) = stack0.get_mut().pop() {
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

    #[test]
    fn test_rwlock() {
        let n = Arc::new(rwlock::RwLock::new(0));
        let mut v = Vec::new();

        for _ in 0..(NUM_THREADS - 1) {
            let n0 = n.clone();
            let t = std::thread::spawn(move || {
                for _ in 0..NUM_LOOP {
                    let r = n0.read();
                    assert_eq!(*r, 0);
                }
            });

            v.push(t);
        }

        let n0 = n.clone();
        let wr = std::thread::spawn(move || {
            for _ in 0..NUM_LOOP {
                {
                    let mut r = n0.write();
                    *r += 1;
                    *r -= 1;
                }
            }
        });

        v.push(wr);

        for t in v {
            t.join().unwrap();
        }
    }
}
