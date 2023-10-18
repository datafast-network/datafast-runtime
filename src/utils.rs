use std::mem::{MaybeUninit, transmute};

pub fn init_slice<'a, T>(src: &[T], dst: &'a mut [MaybeUninit<T>]) -> &'a mut [T]
    where
        T: Copy,
{
    unsafe {
        let uninit_src: &[MaybeUninit<T>] = transmute(src);
        dst.copy_from_slice(uninit_src);
        &mut *(dst as *mut [MaybeUninit<T>] as *mut [T])
    }
}