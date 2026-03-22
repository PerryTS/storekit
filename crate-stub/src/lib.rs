// Perry runtime FFI functions for promise handling and NaN-boxing
extern "C" {
    fn js_promise_new() -> *mut u8;
    fn js_promise_resolve(promise: *mut u8, value: f64);
    fn js_nanbox_string(ptr: i64) -> f64;
    fn js_nanbox_pointer(ptr: i64) -> f64;
}

/// Helper to create a promise, resolve it immediately with an error/stub JSON string,
/// and return the NaN-boxed promise handle.
fn resolve_with_error(msg: &str) -> f64 {
    unsafe {
        let promise = js_promise_new();
        let c_str = std::ffi::CString::new(msg).unwrap();
        let val = js_nanbox_string(c_str.as_ptr() as i64);
        std::mem::forget(c_str);
        js_promise_resolve(promise, val);
        js_nanbox_pointer(promise as i64)
    }
}

#[no_mangle]
pub extern "C" fn sb_storekit_load_products(_product_ids: i64) -> f64 {
    resolve_with_error("{\"error\":\"StoreKit not available on this platform\"}")
}

#[no_mangle]
pub extern "C" fn sb_storekit_purchase(_product_id: i64) -> f64 {
    resolve_with_error("{\"error\":\"StoreKit not available on this platform\",\"success\":false}")
}

#[no_mangle]
pub extern "C" fn sb_storekit_restore() -> f64 {
    resolve_with_error("{\"error\":\"StoreKit not available on this platform\",\"success\":false}")
}

#[no_mangle]
pub extern "C" fn sb_storekit_has_subscription() -> f64 {
    resolve_with_error("{\"hasSubscription\":false}")
}

#[no_mangle]
pub extern "C" fn sb_storekit_get_jws() -> f64 {
    resolve_with_error("{\"jws\":null}")
}

#[no_mangle]
pub extern "C" fn sb_storekit_start_listener() -> f64 { 0.0 }
