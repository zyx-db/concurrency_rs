use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Release, Relaxed};

pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>
}

pub struct SpinLockGuard<'a, T>{
    lock: &'a SpinLock<T>
}

// needed for us to share a SpinLock across threads
unsafe impl<T> Sync for SpinLock<T> where T: Send {}

impl<T> SpinLock<T> {
    pub fn lock(&self) -> SpinLockGuard<T>{
        loop {
            match self.locked.compare_exchange(false, true, Acquire, Relaxed){
                Ok(_) => {
                    break
                }
                Err(_) => {
                    // this hint tells the cpu:
                    // "hey, we can do other stuff!"
                    // "this work is not 'useful' in terms of cpu cycles"
                    std::hint::spin_loop();
                }
            }
        }
        SpinLockGuard {lock: self}
    }
    
    /// Safety: the &mut T from lock() cannot be around!
    /// once we unlock, there can be other concurrent readers / writers!
    /// this can lead to a data race
    fn unlock(&self){
        self.locked.store(false, Release)
    }
}

impl<T> Drop for SpinLockGuard<'_, T>{
    fn drop(&mut self) {
        self.lock.unlock();    
    }
}

impl<T> Deref for SpinLockGuard<'_, T>{
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &mut *self.lock.value.get()}
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T>{
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.value.get() } 
    }
}

#[cfg(test)]
mod tests {
    //use super::*;

    //#[test]
    //fn it_works() {
    //}
}
