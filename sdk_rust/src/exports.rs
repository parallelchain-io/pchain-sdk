#[no_mangle]
pub extern "C" fn alloc(len: u32) -> *mut u8 {
    let mut buf = Vec::with_capacity(len as usize);
    let ptr = buf.as_mut_ptr();
    
    // IMPORTANT: However you use `alloc`, you need to make sure that this
    // is dropped at some point.
    std::mem::forget(buf);

    return ptr;
}
