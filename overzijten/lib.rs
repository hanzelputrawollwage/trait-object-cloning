#![feature(rustc_private)]

use std::alloc::{alloc, dealloc, Layout};
use std::mem::forget;
use std::ptr::write;

pub trait CloneIntoBox {
    unsafe fn clone_into_ptr(&self, ptr: *mut u8);
}

impl<T: Clone> CloneIntoBox for T {
    unsafe fn clone_into_ptr(&self, ptr: *mut u8) {
        write(ptr as *mut T, self.clone())
    }
}

pub trait CloneIntoBoxExt: CloneIntoBox {
    fn clone_into_box(&self) -> Box<Self> {
        struct Guard {
            ptr: *mut u8,
            layout: Layout,
        }
        impl Drop for Guard {
            fn drop(&mut self) {
                unsafe {
                    dealloc(self.ptr, self.layout);
                }
            }
        }

        let layout = Layout::for_value::<Self>(self);
        let ptr = unsafe { alloc(layout) };
        let guard = Guard { ptr, layout };
        unsafe {
            self.clone_into_ptr(ptr);
        }
        forget(guard);
        unsafe { Box::from_raw(assign_thin_mut(self, ptr)) }
    }
}
impl<T: CloneIntoBox + ?Sized> CloneIntoBoxExt for T {}

fn assign_thin_mut<T: ?Sized>(meta: *const T, thin: *mut u8) -> *mut T {
    let mut fat = meta as *mut T;
    unsafe {
        *(&mut fat as *mut *mut T as *mut *mut u8) = thin;
    }
    fat
}

#[cfg(test)]
mod tests {
    use super::*;

    pub trait FnClone: Fn() -> String + CloneIntoBox {}
    impl<T: ?Sized> FnClone for T where T: Fn() -> String + CloneIntoBox {}

    impl<'a> Clone for Box<dyn FnClone + 'a> {
        fn clone(&self) -> Self {
            (**self).clone_into_box()
        }
    }

    #[test]
    fn test_clone_fn() {
        let s = String::from("Hello,");
        let f: Box<dyn FnClone> = Box::new(move || format!("{} world!", s));
        assert_eq!(f(), "Hello, world!");
        let ff = f.clone();
        assert_eq!(ff(), "Hello, world!");
    }

    #[test]
    #[should_panic(expected = "PanicClone::clone() is called")]
    fn test_clone_panic() {
        struct PanicClone;
        impl Clone for PanicClone {
            fn clone(&self) -> Self {
                panic!("PanicClone::clone() is called");
            }
        }
        let s = String::from("Hello,");
        let p = PanicClone;
        let f: Box<dyn FnClone> = Box::new(move || {
            let _ = &p;
            format!("{} world!", s)
        });
        assert_eq!(f(), "Hello, world!");
        let _ = f.clone();
    }
}
