// Perry runtime FFI functions for promise handling and NaN-boxing
extern "C" {
    fn js_promise_new() -> *mut u8;
    fn js_promise_resolve(promise: *mut u8, value: f64);
    fn js_nanbox_string(ptr: i64) -> f64;
    fn js_nanbox_pointer(ptr: i64) -> f64;
    fn js_get_string_pointer_unified(val: f64) -> *const u8;
}

// Swift bridge functions (defined via @_cdecl in storekit_bridge.swift)
extern "C" {
    fn swift_storekit_load_products(
        product_ids: *const u8,
        callback: extern "C" fn(*mut u8, *const u8),
        context: *mut u8,
    );

    fn swift_storekit_purchase(
        product_id: *const u8,
        callback: extern "C" fn(*mut u8, *const u8),
        context: *mut u8,
    );

    fn swift_storekit_restore(
        callback: extern "C" fn(*mut u8, *const u8),
        context: *mut u8,
    );

    fn swift_storekit_has_subscription(
        callback: extern "C" fn(*mut u8, *const u8),
        context: *mut u8,
    );

    fn swift_storekit_get_jws(
        callback: extern "C" fn(*mut u8, *const u8),
        context: *mut u8,
    );

    fn swift_storekit_start_listener();
}

/// Callback invoked by Swift when an async StoreKit operation completes.
/// Resolves the Perry promise with the result string.
extern "C" fn storekit_callback(context: *mut u8, result: *const u8) {
    unsafe {
        let promise = context;
        let result_str = js_nanbox_string(result as i64);
        js_promise_resolve(promise, result_str);
    }
}

/// Load products from StoreKit by comma-separated product IDs.
/// Returns a NaN-boxed promise handle.
#[no_mangle]
pub extern "C" fn sb_storekit_load_products(product_ids_ptr: i64) -> f64 {
    unsafe {
        let promise = js_promise_new();
        let str_ptr = js_get_string_pointer_unified(f64::from_bits(product_ids_ptr as u64));
        swift_storekit_load_products(str_ptr, storekit_callback, promise);
        js_nanbox_pointer(promise as i64)
    }
}

/// Purchase a product by its StoreKit product ID.
/// Returns a NaN-boxed promise handle.
#[no_mangle]
pub extern "C" fn sb_storekit_purchase(product_id_ptr: i64) -> f64 {
    unsafe {
        let promise = js_promise_new();
        let str_ptr = js_get_string_pointer_unified(f64::from_bits(product_id_ptr as u64));
        swift_storekit_purchase(str_ptr, storekit_callback, promise);
        js_nanbox_pointer(promise as i64)
    }
}

/// Restore purchases via AppStore.sync().
/// Returns a NaN-boxed promise handle.
#[no_mangle]
pub extern "C" fn sb_storekit_restore() -> f64 {
    unsafe {
        let promise = js_promise_new();
        swift_storekit_restore(storekit_callback, promise);
        js_nanbox_pointer(promise as i64)
    }
}

/// Check if the user has an active subscription.
/// Returns a NaN-boxed promise handle.
#[no_mangle]
pub extern "C" fn sb_storekit_has_subscription() -> f64 {
    unsafe {
        let promise = js_promise_new();
        swift_storekit_has_subscription(storekit_callback, promise);
        js_nanbox_pointer(promise as i64)
    }
}

/// Get the JWS (JSON Web Signature) for the latest transaction.
/// Returns a NaN-boxed promise handle.
#[no_mangle]
pub extern "C" fn sb_storekit_get_jws() -> f64 {
    unsafe {
        let promise = js_promise_new();
        swift_storekit_get_jws(storekit_callback, promise);
        js_nanbox_pointer(promise as i64)
    }
}

/// Start the StoreKit transaction update listener.
/// This runs in the background and finishes verified transactions.
#[no_mangle]
pub extern "C" fn sb_storekit_start_listener() -> f64 {
    unsafe {
        swift_storekit_start_listener();
    }
    0.0
}
