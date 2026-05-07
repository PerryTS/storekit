//! Non-Apple stub for `perry-storekit`. Every `js_storekit_*` entry point
//! resolves immediately with a `"not available on this platform"` JSON
//! payload so calling code can fall back to a Stripe-card flow (or
//! whatever the app's non-IAP path is) without special-casing the
//! presence of the binding.

use perry_ffi::{JsPromise, Promise, StringHeader};

fn resolved_with(msg: &str) -> *mut Promise {
    let promise = JsPromise::new();
    let raw = promise.as_raw();
    promise.resolve_string(msg);
    raw
}

#[no_mangle]
pub extern "C" fn js_storekit_load_products(_product_ids: *const StringHeader) -> *mut Promise {
    resolved_with("{\"error\":\"StoreKit not available on this platform\"}")
}

#[no_mangle]
pub extern "C" fn js_storekit_purchase(_product_id: *const StringHeader) -> *mut Promise {
    resolved_with("{\"error\":\"StoreKit not available on this platform\",\"success\":false}")
}

#[no_mangle]
pub extern "C" fn js_storekit_restore() -> *mut Promise {
    resolved_with("{\"error\":\"StoreKit not available on this platform\",\"success\":false}")
}

#[no_mangle]
pub extern "C" fn js_storekit_has_subscription() -> *mut Promise {
    resolved_with("{\"hasSubscription\":false}")
}

#[no_mangle]
pub extern "C" fn js_storekit_get_jws() -> *mut Promise {
    resolved_with("{\"jws\":null}")
}

#[no_mangle]
pub extern "C" fn js_storekit_start_listener() {}
