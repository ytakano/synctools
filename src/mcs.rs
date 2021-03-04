use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::ptr::null_mut;
use core::sync::atomic::{fence, AtomicBool, AtomicPtr, Ordering};

pub struct MCSLock<T> {
    last: AtomicPtr<MCSNode<T>>,
    data: UnsafeCell<T>,
}

struct MCSNode<T> {
    next: AtomicPtr<MCSNode<T>>,
    locked: AtomicBool,
}

impl<T> MCSLock<T> {
    pub const fn new(v: T) -> MCSLock<T> {
        MCSLock {
            last: AtomicPtr::new(null_mut()),
            data: UnsafeCell::new(v),
        }
    }

    /// acquire lock
    pub fn lock(&self) -> MCSLockGuard<T> {
        // set myself as the last node
        let mut guard = MCSLockGuard {
            node: MCSNode {
                next: AtomicPtr::new(null_mut()),
                locked: AtomicBool::new(false),
            },
            mcs_lock: self,
        };

        let ptr = &mut guard.node as *mut MCSNode<T>;
        let prev = self.last.swap(ptr, Ordering::Relaxed);

        // if prev is null then nobody is trying to acquire lock,
        // otherwise enqueue myself
        if prev != null_mut() {
            // set acquiring lock
            guard.node.locked.store(true, Ordering::Relaxed);

            // enqueue myself
            let prev = unsafe { &*prev };
            prev.next.store(ptr, Ordering::Relaxed);

            // spin until other thread set locked false
            while guard.node.locked.load(Ordering::Relaxed) {}
        }

        fence(Ordering::Acquire);
        guard
    }
}

unsafe impl<T> Sync for MCSLock<T> {}
unsafe impl<T> Send for MCSLock<T> {}

pub struct MCSLockGuard<'a, T> {
    node: MCSNode<T>,
    mcs_lock: &'a MCSLock<T>,
}

impl<'a, T> Drop for MCSLockGuard<'a, T> {
    fn drop(&mut self) {
        // if next node is null and self is the last node
        // set the last node to null
        if self.node.next.load(Ordering::Relaxed) == null_mut() {
            let ptr = &mut self.node as *mut MCSNode<T>;
            if let Ok(_) = self.mcs_lock.last.compare_exchange(
                ptr,
                null_mut(),
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                return;
            }
        }

        // other thread is entering lock and wait the execution
        while self.node.next.load(Ordering::Relaxed) == null_mut() {}

        // make next thread executable
        let next = unsafe { &mut *self.node.next.load(Ordering::Relaxed) };
        next.locked.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for MCSLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mcs_lock.data.get() }
    }
}

impl<'a, T> DerefMut for MCSLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mcs_lock.data.get() }
    }
}
