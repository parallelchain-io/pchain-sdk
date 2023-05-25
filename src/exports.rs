/*
    Copyright © 2023, ParallelChain Lab 
    Licensed under the Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
*/

//! Defines a function, `alloc` that is exported from Contract WASM modules so that the ParallelChain VM 
//! can allocate segments in WASM linear memory. The name of this module is rather awkward, since `alloc` *is not* the 
//! only function exported from Contract WASM modules. We plan to merge this module with imports in a future version of the SDK.

#[no_mangle]
pub extern "C" fn alloc(len: u32) -> *mut u8 {
    let mut buf = Vec::with_capacity(len as usize);
    let ptr = buf.as_mut_ptr();
    
    std::mem::forget(buf);

    return ptr;
}
