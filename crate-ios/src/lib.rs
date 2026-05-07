//! StoreKit 2 bindings for Perry on Apple platforms — closes PerryTS/perry#537.
//!
//! The Rust side is a thin shim: every exported `js_storekit_*` function
//! allocates a [`JsPromise`], hands the underlying `*mut Promise` to a
//! Swift `@_cdecl` entry point as the callback "context", and returns the
//! same pointer to the perry runtime. When the Swift side finishes its
//! `Task { … }` it invokes [`storekit_callback`] from whatever thread the
//! Swift concurrency runtime parked the continuation on. The perry-ffi
//! resolution machinery is `Send`-safe and queues the actual fulfillment
//! back onto perry's main thread.
//!
//! Swift returns a JSON string for every call; the TypeScript wrapper
//! parses it. We do not attempt to construct typed JsValues on the Rust
//! side — keeping the cross-language contract a single string makes the
//! Swift bridge trivial and avoids leaking perry-runtime layout into
//! Swift code.

use perry_ffi::{read_string, JsPromise, JsString, Promise, StringHeader};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

extern "C" {
    fn swift_storekit_load_products(
        product_ids: *const c_char,
        callback: extern "C" fn(*mut c_void, *const c_char),
        context: *mut c_void,
    );

    fn swift_storekit_purchase(
        product_id: *const c_char,
        callback: extern "C" fn(*mut c_void, *const c_char),
        context: *mut c_void,
    );

    fn swift_storekit_restore(
        callback: extern "C" fn(*mut c_void, *const c_char),
        context: *mut c_void,
    );

    fn swift_storekit_has_subscription(
        callback: extern "C" fn(*mut c_void, *const c_char),
        context: *mut c_void,
    );

    fn swift_storekit_get_jws(
        callback: extern "C" fn(*mut c_void, *const c_char),
        context: *mut c_void,
    );

    fn swift_storekit_start_listener();
}

/// Invoked by Swift exactly once per outstanding promise. Reconstructs
/// the `JsPromise` from the raw pointer we passed in as `context`,
/// resolves it with the Swift-built JSON string, and lets the perry
/// runtime carry the result back to the awaiter.
extern "C" fn storekit_callback(context: *mut c_void, result: *const c_char) {
    let promise = unsafe { JsPromise::from_raw(context as *mut Promise) };
    let result_str = if result.is_null() {
        "{\"error\":\"null result from StoreKit bridge\"}"
    } else {
        unsafe { std::ffi::CStr::from_ptr(result) }
            .to_str()
            .unwrap_or("{\"error\":\"non-UTF8 result from StoreKit bridge\"}")
    };
    promise.resolve_string(result_str);
}

/// Read a perry JS string into an owned `CString` we can hand to Swift.
/// Returns `None` if the input is null/non-UTF8 or contains an interior NUL.
unsafe fn js_string_to_cstring(ptr: *const StringHeader) -> Option<CString> {
    let handle = JsString::from_raw(ptr as *mut StringHeader);
    let s = read_string(handle)?;
    CString::new(s).ok()
}

/// `loadProducts(commaSeparatedIds): Promise<string>` — fetch StoreKit
/// products by ID. The Swift side returns a JSON-encoded array; the
/// TypeScript wrapper splits incoming `string[]` by `,` and `JSON.parse`s
/// the result on the way out.
///
/// # Safety
///
/// `product_ids_ptr` must be null or point to a perry-runtime `StringHeader`.
#[no_mangle]
pub unsafe extern "C" fn js_storekit_load_products(
    product_ids_ptr: *const StringHeader,
) -> *mut Promise {
    let promise = JsPromise::new();
    let raw = promise.as_raw();

    let Some(c_ids) = js_string_to_cstring(product_ids_ptr) else {
        promise
            .resolve_string("{\"error\":\"product_ids is null, non-UTF8, or contains NUL\"}");
        return raw;
    };

    swift_storekit_load_products(c_ids.as_ptr(), storekit_callback, raw as *mut c_void);
    raw
}

/// `purchase(productId): Promise<string>` — drive the StoreKit purchase
/// sheet for a single product. JSON result includes `success`, `jws`,
/// `productId`, `transactionId`, `purchaseDate`, `cancelled`, `pending`.
///
/// # Safety
///
/// `product_id_ptr` must be null or point to a perry-runtime `StringHeader`.
#[no_mangle]
pub unsafe extern "C" fn js_storekit_purchase(
    product_id_ptr: *const StringHeader,
) -> *mut Promise {
    let promise = JsPromise::new();
    let raw = promise.as_raw();

    let Some(c_id) = js_string_to_cstring(product_id_ptr) else {
        promise.resolve_string(
            "{\"error\":\"product_id is null, non-UTF8, or contains NUL\",\"success\":false}",
        );
        return raw;
    };

    swift_storekit_purchase(c_id.as_ptr(), storekit_callback, raw as *mut c_void);
    raw
}

/// `restorePurchases(): Promise<string>` — calls `AppStore.sync()`.
#[no_mangle]
pub extern "C" fn js_storekit_restore() -> *mut Promise {
    let promise = JsPromise::new();
    let raw = promise.as_raw();
    unsafe { swift_storekit_restore(storekit_callback, raw as *mut c_void) };
    raw
}

/// `hasSubscription(): Promise<string>` — JSON `{ "hasSubscription": bool }`.
/// True iff at least one verified entitlement has no revocation date.
#[no_mangle]
pub extern "C" fn js_storekit_has_subscription() -> *mut Promise {
    let promise = JsPromise::new();
    let raw = promise.as_raw();
    unsafe { swift_storekit_has_subscription(storekit_callback, raw as *mut c_void) };
    raw
}

/// `getJWS(): Promise<string>` — most recent verified entitlement's JWS,
/// or `{"jws": null}` if there is none. Server-side validators feed the
/// JWS to Apple's App Store Server API for receipt verification.
#[no_mangle]
pub extern "C" fn js_storekit_get_jws() -> *mut Promise {
    let promise = JsPromise::new();
    let raw = promise.as_raw();
    unsafe { swift_storekit_get_jws(storekit_callback, raw as *mut c_void) };
    raw
}

/// `startListener(): void` — start the `Transaction.updates` background
/// task that finishes verified transactions arriving outside an explicit
/// `purchase()` call (Ask-to-Buy approval, family-shared entitlements,
/// auto-renew, …). Call this once at app launch.
#[no_mangle]
pub extern "C" fn js_storekit_start_listener() {
    unsafe { swift_storekit_start_listener() };
}
