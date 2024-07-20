use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Relaxed, Acquire, Release};

pub struct OneShotChannel<T>{
    data: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
    in_use: AtomicBool
}

unsafe impl<T> Sync for OneShotChannel<T> where T: Send {}

impl<T> OneShotChannel<T> {
    pub fn new() -> Self{
        OneShotChannel{
            data: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
            in_use: AtomicBool::new(false),
        }
    }
    
    /// Panics! if message in the channel already
    ///
    /// Hint: Check status with `is_ready`
    pub unsafe fn send(&self, data: T){
        if !self.in_use.swap(true, Relaxed){
            panic!("cannot send more than 1 message!")
        }
        unsafe {
            let val = &mut *self.data.get();
            val.write(data);
        }
        self.ready.store(true, Release);
    }

    pub fn blocking_recieve(&self) -> T{
        // spin on status
        while !self.ready.load(Acquire){}
        unsafe {
            return (*self.data.get()).assume_init_read();
        };
    }

    pub fn is_ready(&self) -> bool {
        // we can use relaxed as:
        // there is no way for it to return true before it is set!
        // the `Release` ordering prevents the write from being moved up
        self.ready.load(Relaxed)
    }

    /// Panics! if no message is ready!
    ///
    /// Hint: Check status with `is_ready`
    ///
    /// Safety: only call this once!
    pub fn recieve(&self) -> T {
        if !self.ready.swap(false, Acquire){
            panic!("no message ready!")
        }
        unsafe {(*self.data.get()).assume_init_read()}
    }
}

impl<T> Drop for OneShotChannel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe {
                self.data.get_mut().assume_init_drop()
            }
        }        
    }
}
