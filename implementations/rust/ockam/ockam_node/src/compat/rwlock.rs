#![allow(unsafe_code)]
#![allow(missing_docs)]

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

// FIXME: Completely unsafe async RwLock implementation

/// Async RwLock
#[derive(Debug)]
pub struct RwLock<T: ?Sized> {
    value: UnsafeCell<T>,
}

unsafe impl<T: Send + ?Sized> Send for RwLock<T> {}
unsafe impl<T: Send + Sync + ?Sized> Sync for RwLock<T> {}

impl<T> RwLock<T> {
    pub const fn new(t: T) -> RwLock<T> {
        Self {
            value: UnsafeCell::new(t),
        }
    }

    pub fn into_inner(self) -> T {
        self.value.into_inner()
    }
}

impl<T: ?Sized> RwLock<T> {
    pub fn try_read(&self) -> Option<RwLockReadGuard<'_, T>> {
        Some(RwLockReadGuard(self))
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        RwLockReadGuard(self)
    }

    pub fn try_upgradable_read(&self) -> Option<RwLockUpgradableReadGuard<'_, T>> {
        Some(RwLockUpgradableReadGuard {
            reader: RwLockReadGuard(self),
        })
    }

    pub async fn upgradable_read(&self) -> RwLockUpgradableReadGuard<'_, T> {
        RwLockUpgradableReadGuard {
            reader: RwLockReadGuard(self),
        }
    }

    pub fn try_write(&self) -> Option<RwLockWriteGuard<'_, T>> {
        Some(RwLockWriteGuard {
            writer: RwLockWriteGuardInner(self),
        })
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        RwLockWriteGuard {
            writer: RwLockWriteGuardInner(self),
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }
}

impl<T> From<T> for RwLock<T> {
    fn from(val: T) -> RwLock<T> {
        RwLock::new(val)
    }
}

impl<T: Default + ?Sized> Default for RwLock<T> {
    fn default() -> RwLock<T> {
        RwLock::new(Default::default())
    }
}

// A guard that releases the read lock when dropped.
pub struct RwLockReadGuard<'a, T: ?Sized>(&'a RwLock<T>);

unsafe impl<T: Sync + ?Sized> Send for RwLockReadGuard<'_, T> {}
unsafe impl<T: Sync + ?Sized> Sync for RwLockReadGuard<'_, T> {}

impl<T: ?Sized> Deref for RwLockReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0.value.get() }
    }
}

pub struct RwLockUpgradableReadGuard<'a, T: ?Sized> {
    reader: RwLockReadGuard<'a, T>,
}

unsafe impl<T: Send + Sync + ?Sized> Send for RwLockUpgradableReadGuard<'_, T> {}
unsafe impl<T: Sync + ?Sized> Sync for RwLockUpgradableReadGuard<'_, T> {}

impl<'a, T: ?Sized> RwLockUpgradableReadGuard<'a, T> {
    fn into_writer(self) -> RwLockWriteGuard<'a, T> {
        self.reader.0.try_write().unwrap()
    }

    pub fn downgrade(guard: Self) -> RwLockReadGuard<'a, T> {
        guard.reader
    }

    pub fn try_upgrade(guard: Self) -> Result<RwLockWriteGuard<'a, T>, Self> {
        Ok(Self::into_writer(guard))
    }

    pub async fn upgrade(guard: Self) -> RwLockWriteGuard<'a, T> {
        Self::into_writer(guard)
    }
}

impl<T: ?Sized> Deref for RwLockUpgradableReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.reader.0.value.get() }
    }
}

struct RwLockWriteGuardInner<'a, T: ?Sized>(&'a RwLock<T>);

pub struct RwLockWriteGuard<'a, T: ?Sized> {
    writer: RwLockWriteGuardInner<'a, T>,
}

unsafe impl<T: Send + ?Sized> Send for RwLockWriteGuard<'_, T> {}
unsafe impl<T: Sync + ?Sized> Sync for RwLockWriteGuard<'_, T> {}

impl<'a, T: ?Sized> RwLockWriteGuard<'a, T> {
    pub fn downgrade(guard: Self) -> RwLockReadGuard<'a, T> {
        RwLockReadGuard(guard.writer.0)
    }

    pub fn downgrade_to_upgradable(guard: Self) -> RwLockUpgradableReadGuard<'a, T> {
        RwLockUpgradableReadGuard {
            reader: Self::downgrade(guard),
        }
    }
}

impl<T: ?Sized> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.writer.0.value.get() }
    }
}

impl<T: ?Sized> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.writer.0.value.get() }
    }
}
