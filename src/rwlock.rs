use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

pub struct RwLock<T> {
    rcnt: AtomicUsize,
    wcnt: AtomicUsize,
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub const fn new(v: T) -> RwLock<T> {
        RwLock {
            rcnt: AtomicUsize::new(0),
            wcnt: AtomicUsize::new(0),
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(v),
        }
    }

    /// acquire reader lock
    pub fn read(&self) -> RwLockReadGuard<T> {
        loop {
            while self.wcnt.load(Ordering::Relaxed) > 0 {}
            self.rcnt.fetch_add(1, Ordering::Acquire);
            if self.wcnt.load(Ordering::Relaxed) == 0 {
                break;
            }
            self.rcnt.fetch_sub(1, Ordering::Relaxed);
        }

        RwLockReadGuard { rwlock: self }
    }

    /// acquire writer lock
    pub fn write(&self) -> RwLockWriteGuard<T> {
        self.wcnt.fetch_add(1, Ordering::Relaxed);
        while self.rcnt.load(Ordering::Relaxed) > 0 {}

        loop {
            while self.lock.load(Ordering::Relaxed) {}
            if let Ok(_) =
                self.lock
                    .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            {
                break;
            }
        }

        RwLockWriteGuard { rwlock: self }
    }
}

pub struct RwLockReadGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<'a, T> RwLockReadGuard<'a, T> {
    /// unlock read lock
    pub fn unlock(self) {}
}

pub struct RwLockWriteGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<'a, T> RwLockWriteGuard<'a, T> {
    /// unlock write lock
    pub fn unlock(self) {}
}

unsafe impl<T> Sync for RwLock<T> {}
unsafe impl<T> Send for RwLock<T> {}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.rwlock.data.get() }
    }
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.rwlock.data.get() }
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.rwlock.data.get() }
    }
}

/// release read lock
impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        self.rwlock.rcnt.fetch_sub(1, Ordering::Release);
    }
}

/// release write lock
impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        self.rwlock.lock.store(false, Ordering::Relaxed);
        self.rwlock.wcnt.fetch_sub(1, Ordering::Release);
    }
}
