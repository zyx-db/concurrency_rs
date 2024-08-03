//TODO: optimization
use std::{ops::Deref, ptr::NonNull, sync::atomic::{fence, AtomicU8, Ordering::{Relaxed, Release, Acquire}}};
use std::cell::UnsafeCell;

struct ArcData<T> {
    // # of arcs
    data_count: AtomicU8,
    // # of arcs + weaks
    total_count: AtomicU8,
    data: UnsafeCell<Option<T>>,
}

pub struct Arc<T> {
    weak: Weak<T>
}

pub struct Weak<T> {
    ptr: NonNull<ArcData<T>>
}

impl<T> Arc<T> {
    pub fn new(value: T) -> Self {
        Arc {
            weak: Weak {
                ptr: NonNull::from(Box::leak(Box::new(ArcData {
                    data_count: AtomicU8::new(1),
                    total_count: AtomicU8::new(1),
                    data: UnsafeCell::new(Some(value)),
                })))
            }
        }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.weak.data().total_count.load(Relaxed) == 1 {
            fence(Acquire);
            let arcdata = unsafe {arc.weak.ptr.as_mut()};
            let option = arcdata.data.get_mut();
            let data = option.as_mut().unwrap();
            Some(data)
        } else {
            None
        }
    }

    pub fn downgrade(&self) -> Weak<T> {
        self.weak.clone()
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let ptr = self.weak.data().data.get();
        unsafe {(*ptr).as_ref().unwrap()}
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let weak = self.weak.clone();
        self.weak.data().data_count.fetch_add(1, Relaxed);
        Arc {weak}
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.weak.data().data_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            let ptr = self.weak.data().data.get();
            unsafe {
                (*ptr) = None;
            }
        }
    }
}

impl<T> Weak<T> {
    fn data(&self) -> &ArcData<T> {
        unsafe {self.ptr.as_ref()}
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        let mut n = self.data().data_count.load(Relaxed);
        loop {
            if n == 0 {
                return None;
            }
            // prevent overflow
            assert!(n <= u8::MAX / 2);
            if let Err(e) = 
                self.data().
                    data_count
                    .compare_exchange_weak(n, n + 1, Relaxed, Relaxed)
            {
                n = e;
                continue;
            };
            return Some(Arc {weak: self.clone()} );
        }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        // dont allow overflow of the atomic
        if self.data().total_count.fetch_add(1, Relaxed) > u8::MAX / 2 {
            std::process::abort()
        };
        Weak {
            ptr: self.ptr
        }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        if self.data().data_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

unsafe impl<T: Send + Sync> Send for Weak<T> {}
unsafe impl<T: Send + Sync> Sync for Weak<T> {}
