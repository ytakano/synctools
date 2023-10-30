use core::{marker::PhantomData, ptr::null_mut};

#[cfg(not(loom))]
use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{fence, AtomicBool, AtomicPtr, Ordering},
};

#[cfg(loom)]
use loom::{
    cell::UnsafeCell,
    hint::spin_loop,
    sync::atomic::{fence, AtomicBool, AtomicPtr, Ordering},
};

pub struct MCSLock<T> {
    last: AtomicPtr<MCSNode<T>>,
    data: UnsafeCell<T>,
}

pub struct MCSNode<T> {
    next: AtomicPtr<MCSNode<T>>,
    locked: AtomicBool,
}

impl<T> Default for MCSNode<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MCSNode<T> {
    pub fn new() -> MCSNode<T> {
        MCSNode {
            next: AtomicPtr::new(null_mut()),
            locked: AtomicBool::new(false),
        }
    }
}

impl<T> MCSLock<T> {
    pub fn new(v: T) -> MCSLock<T> {
        MCSLock {
            last: AtomicPtr::new(null_mut()),
            data: UnsafeCell::new(v),
        }
    }

    /// acquire lock
    pub fn lock<'a>(&'a self, node: &'a mut MCSNode<T>) -> MCSLockGuard<T> {
        node.next = AtomicPtr::new(null_mut());
        node.locked = AtomicBool::new(false);

        // set myself as the last node
        let guard = MCSLockGuard {
            node,
            mcs_lock: self,
            _phantom: PhantomData,
        };

        let ptr = guard.node as *mut MCSNode<T>;
        let prev = self.last.swap(ptr, Ordering::AcqRel);

        // if prev is null then nobody is trying to acquire lock
        if prev.is_null() {
            return guard;
        }

        // enqueue myself
        let prev = unsafe { &*prev };
        prev.next.store(ptr, Ordering::Release);

        // spin until other thread sets locked true
        while !guard.node.locked.load(Ordering::Relaxed) {
            spin_loop();

            #[cfg(loom)]
            loom::thread::yield_now();
        }
        fence(Ordering::Acquire);

        guard
    }
}

unsafe impl<T> Sync for MCSLock<T> {}
unsafe impl<T> Send for MCSLock<T> {}

pub struct MCSLockGuard<'a, T> {
    node: &'a mut MCSNode<T>,
    mcs_lock: &'a MCSLock<T>,
    _phantom: PhantomData<*mut ()>,
}

impl<'a, T> MCSLockGuard<'a, T> {
    /// unlock MCS lock
    pub fn unlock(self) {}

    #[cfg(loom)]
    pub fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(*mut T) -> R,
    {
        self.mcs_lock.data.with_mut(f)
    }
}

impl<'a, T> Drop for MCSLockGuard<'a, T> {
    fn drop(&mut self) {
        // if next node is null and self is the last node
        // set the last node to null
        if self.node.next.load(Ordering::Relaxed).is_null() {
            let ptr = self.node as *mut MCSNode<T>;
            if self
                .mcs_lock
                .last
                .compare_exchange(ptr, null_mut(), Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }

            // other thread is entering lock and wait the execution
            while self.node.next.load(Ordering::Relaxed).is_null() {
                spin_loop();

                #[cfg(loom)]
                loom::thread::yield_now();
            }
        }

        // make next thread executable
        let next = unsafe { &mut *self.node.next.load(Ordering::Acquire) };
        next.locked.store(true, Ordering::Release);
    }
}

#[cfg(not(loom))]
impl<'a, T> Deref for MCSLockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mcs_lock.data.get() }
    }
}

#[cfg(not(loom))]
impl<'a, T> DerefMut for MCSLockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mcs_lock.data.get() }
    }
}
