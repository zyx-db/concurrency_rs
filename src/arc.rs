use std::{ops::Deref, ptr::NonNull, sync::atomic::{fence, AtomicU8, Ordering::{Relaxed, Release, Acquire}}};

struct ArcData<T> {
    ref_count: AtomicU8,
    data: T,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>
}

// TODO: weak references
impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Arc {
            ptr: NonNull::from(Box::leak(Box::new(
                ArcData {
                    ref_count: AtomicU8::new(1),
                    data
                }
            )))
        }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().ref_count.load(Relaxed) == 1 {
            fence(Acquire);
            unsafe { Some(&mut arc.ptr.as_mut().data) }
        } else {
            None
        }
    }

    fn data(&self) -> &ArcData<T> {
        unsafe {self.ptr.as_ref()}
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data().data
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // TODO: handle overflow of ref count
        self.data().ref_count.fetch_add(1, Relaxed);
        Arc {
            ptr: self.ptr
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}
