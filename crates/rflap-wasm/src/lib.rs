//! WebAssembly build of [`rflap_api`]: a raw C ABI, no wasm-bindgen, no JS glue to generate.
//!
//! The web editor (`web/src/wasm.js`) drives it like this:
//!
//! 1. `rflap_alloc(len)` reserves `len` bytes in the module memory and returns their address;
//! 2. the host copies the UTF-8 request there and calls `rflap_call(ptr, len)`;
//! 3. the answer is a buffer `[u32 little-endian length][UTF-8 JSON]` (the JSON is exactly what
//!    [`rflap_api::call`] returns); the host reads it and
//! 4. frees the request with `rflap_free(ptr, len)` and the answer with `rflap_free_result`.
//!
//! The functions are also plain Rust functions, so the tests call them on the host target.

/// Reserves `len` zeroed bytes and returns their address (never null, even for `len == 0`).
///
/// Free them with [`rflap_free`] and the same `len`.
#[unsafe(no_mangle)]
pub extern "C" fn rflap_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8
}

/// Frees a buffer from [`rflap_alloc`].
///
/// # Safety
/// `ptr` and `len` must be exactly what `rflap_alloc` returned and was given, and the buffer must
/// not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rflap_free(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        // SAFETY: by the contract, `ptr` came from a boxed slice of exactly `len` bytes.
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)) });
    }
}

/// Runs the request in `ptr[..len]` (UTF-8 JSON) and returns `[u32 le length][UTF-8 JSON]`.
///
/// Never returns null; bad UTF-8 is answered with an `invalid_json` error like any bad request.
/// Free the result with [`rflap_free_result`].
///
/// # Safety
/// `ptr[..len]` must be readable (for example a buffer from [`rflap_alloc`]).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rflap_call(ptr: *const u8, len: usize) -> *const u8 {
    let bytes: &[u8] = if len == 0 {
        &[]
    } else {
        // SAFETY: by the contract, `ptr[..len]` is readable.
        unsafe { std::slice::from_raw_parts(ptr, len) }
    };
    let answer = match std::str::from_utf8(bytes) {
        Ok(request) => rflap_api::call(request),
        Err(e) => format!(
            r#"{{"ok":false,"error":{{"code":"invalid_json","message":"the request is not valid UTF-8: {e}"}}}}"#
        ),
    };
    let body = answer.as_bytes();
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    Box::into_raw(out.into_boxed_slice()) as *const u8
}

/// Frees an answer from [`rflap_call`].
///
/// # Safety
/// `ptr` must be exactly what `rflap_call` returned, and the buffer must not be used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rflap_free_result(ptr: *const u8) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: by the contract, `ptr` starts a boxed slice of `4 + length` bytes.
    unsafe {
        let len = u32::from_le_bytes(std::slice::from_raw_parts(ptr, 4).try_into().unwrap());
        let total = 4 + len as usize;
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            ptr as *mut u8,
            total,
        )));
    }
}
