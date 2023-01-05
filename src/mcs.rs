use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    ptr::null_mut,
    sync::atomic::{fence, AtomicBool, AtomicPtr, Ordering},
};
use pin_project_lite::pin_project;

pub struct MCSLock<T> {
    last: AtomicPtr<MCSNode<T>>,
    data: UnsafeCell<T>,
}

pin_project! {
    pub struct PinnedNode<T> {
        #[pin]
        pinned: MCSNode<T>
    }
}

struct MCSNode<T> {
    next: AtomicPtr<MCSNode<T>>,
    locked: AtomicBool,
}

impl<T> Default for MCSNode<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MCSNode<T> {
    fn new() -> MCSNode<T> {
        MCSNode {
            next: AtomicPtr::new(null_mut()),
            locked: AtomicBool::new(false),
        }
    }
}

impl<T> MCSLock<T> {
    pub const fn new(v: T) -> MCSLock<T> {
        MCSLock {
            last: AtomicPtr::new(null_mut()),
            data: UnsafeCell::new(v),
        }
    }

    /// acquire lock
    pub fn lock<'a>(&'a self) -> MCSLockGuard<T> {
        let pinned_node = PinnedNode::new();

        // set myself as the last node
        let mut guard = MCSLockGuard {
            pinned_node,
            mcs_lock: self,
        };

        let ptr = &mut guard.pinned_node.pinned as *mut MCSNode<T>;
        let prev = self.last.swap(ptr, Ordering::Relaxed);

        // if prev is null then nobody is trying to acquire lock,
        // otherwise enqueue myself
        if !prev.is_null() {
            // set acquiring lock
            guard
                .pinned_node
                .pinned
                .locked
                .store(true, Ordering::Relaxed);

            // enqueue myself
            let prev = unsafe { &*prev };
            prev.next.store(ptr, Ordering::Relaxed);

            // spin until other thread set locked false
            while guard.pinned_node.pinned.locked.load(Ordering::Relaxed) {
                core::hint::spin_loop()
            }
        }

        fence(Ordering::Acquire);
        guard
    }
}

unsafe impl<T> Sync for MCSLock<T> {}
unsafe impl<T> Send for MCSLock<T> {}

pub struct MCSLockGuard<'a, T> {
    pinned_node: PinnedNode<T>,
    mcs_lock: &'a MCSLock<T>,
}

impl<'a, T> MCSLockGuard<'a, T> {
    /// unlock MCS lock
    pub fn unlock(self) {}
}

impl<'a, T> Drop for MCSLockGuard<'a, T> {
    fn drop(&mut self) {
        // if next node is null and self is the last node
        // set the last node to null
        if self
            .pinned_node
            .pinned
            .next
            .load(Ordering::Relaxed)
            .is_null()
        {
            let ptr = &mut self.pinned_node.pinned as *mut MCSNode<T>;
            if self
                .mcs_lock
                .last
                .compare_exchange(ptr, null_mut(), Ordering::Release, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
        }

        // other thread is entering lock and wait the execution
        while self
            .pinned_node
            .pinned
            .next
            .load(Ordering::Relaxed)
            .is_null()
        {
            core::hint::spin_loop()
        }

        // make next thread executable
        let next = unsafe { &mut *self.pinned_node.pinned.next.load(Ordering::Relaxed) };
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

impl<T> PinnedNode<T> {
    fn new() -> Self {
        Self {
            pinned: MCSNode::new(),
        }
    }
}
