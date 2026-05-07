# @perryts/storekit

StoreKit 2 in-app purchase bindings for [Perry](https://github.com/PerryTS/perry) — closes [PerryTS/perry#537](https://github.com/PerryTS/perry/issues/537).

## Platforms

| Target          | Implementation                                                          |
| --------------- | ----------------------------------------------------------------------- |
| iOS 16+         | Native — Swift bridge over StoreKit 2 (`Product.products(for:)`, etc.). |
| macOS 13+       | Native — same Swift bridge.                                             |
| Linux / Windows | Stub — every call resolves with a `"not available"` JSON payload.       |
| Android         | Stub. (Google Play Billing is a separate binding — see issue #537.)     |

## Installation

```bash
npm install @perryts/storekit
```

The package targets perry-ffi ABI v0.5 (`perry.nativeLibrary.abiVersion: "0.5"` in `package.json`). Perry validates compatibility at build time.

## Quick start

```typescript
import {
  js_storekit_start_listener,
  js_storekit_load_products,
  js_storekit_purchase,
  js_storekit_has_subscription,
  js_storekit_get_jws,
  js_storekit_restore,
} from "@perryts/storekit";

// Boot the Transaction.updates listener once at launch — handles
// Ask-to-Buy approvals, family-shared entitlements, auto-renew, etc.
js_storekit_start_listener();

// Load products you have configured in App Store Connect.
const productsJson = await js_storekit_load_products(
  "com.example.pro_monthly,com.example.pro_annual",
);
const products = JSON.parse(productsJson);

// Drive the purchase sheet. The product must have been loaded first —
// StoreKit 2 needs the in-memory `Product` value to call `purchase()`.
const purchaseJson = await js_storekit_purchase("com.example.pro_monthly");
const purchase = JSON.parse(purchaseJson);
if (purchase.success) {
  // purchase.jws → server-side receipt validation
  // purchase.transactionId, purchase.purchaseDate → audit log
}

// Check entitlements at any time.
const subJson = await js_storekit_has_subscription();
const { hasSubscription } = JSON.parse(subJson);
```

## Typed wrapper (recommended)

The native FFI returns JSON strings so the cross-language contract stays simple. In your app, wrap the calls with the types this package re-exports:

```typescript
import {
  js_storekit_load_products,
  js_storekit_purchase,
  js_storekit_has_subscription,
  js_storekit_get_jws,
  js_storekit_restore,
  type Product,
  type PurchaseResult,
  type HasSubscriptionResult,
  type JwsResult,
  type RestoreResult,
} from "@perryts/storekit";

export async function loadProducts(ids: string[]): Promise<Product[]> {
  const json = await js_storekit_load_products(ids.join(","));
  const parsed = JSON.parse(json);
  if (parsed && typeof parsed === "object" && "error" in parsed) {
    throw new Error(parsed.error as string);
  }
  return parsed as Product[];
}

export async function purchase(productId: string): Promise<PurchaseResult> {
  const json = await js_storekit_purchase(productId);
  return JSON.parse(json) as PurchaseResult;
}

export async function hasSubscription(): Promise<boolean> {
  const json = await js_storekit_has_subscription();
  return (JSON.parse(json) as HasSubscriptionResult).hasSubscription;
}

export async function getJWS(): Promise<string | null> {
  const json = await js_storekit_get_jws();
  return (JSON.parse(json) as JwsResult).jws;
}

export async function restorePurchases(): Promise<RestoreResult> {
  const json = await js_storekit_restore();
  return JSON.parse(json) as RestoreResult;
}
```

The `Product` and `PurchaseResult` shapes match the sketch in [issue #537](https://github.com/PerryTS/perry/issues/537), with one practical addition: `PurchaseResult.jws` carries the App Store-issued JWS, which is what Apple's [App Store Server API](https://developer.apple.com/documentation/appstoreserverapi) expects for server-side validation. (The legacy base64 `receipt` is no longer the recommended StoreKit 2 path.)

## API reference

### `js_storekit_start_listener(): void`

Start `Transaction.updates` in a detached `Task`. Verified transactions are automatically `finish()`-ed. Call exactly once at app launch — calling again cancels the previous listener.

### `js_storekit_load_products(commaSeparatedIds: string): Promise<string>`

Resolves with a JSON array of [`Product`](./src/index.ts) objects. On failure: `{"error": "..."}`.

Loaded products are cached in a Swift `actor` so `js_storekit_purchase` can look them up by ID.

### `js_storekit_purchase(productId: string): Promise<string>`

Resolves with a JSON [`PurchaseResult`](./src/index.ts). Possible shapes:

```json
{ "success": true,  "jws": "eyJ…", "productId": "…", "transactionId": "…", "purchaseDate": "2026-05-07T10:23:11.123Z", "cancelled": false }
{ "success": false, "cancelled": true }
{ "success": false, "pending": true }
{ "success": false, "error": "…" }
```

### `js_storekit_has_subscription(): Promise<string>`

Resolves with `{"hasSubscription": boolean}`. True iff at least one of `Transaction.currentEntitlements` is verified and has no `revocationDate`.

### `js_storekit_get_jws(): Promise<string>`

Resolves with `{"jws": "…"}` (most recent verified entitlement) or `{"jws": null}` (no active entitlement). Hand the JWS to your server, validate against Apple, then trust it.

### `js_storekit_restore(): Promise<string>`

Calls `AppStore.sync()`. Resolves with `{"success": true}` or `{"error": "…", "success": false}`. Apple recommends only invoking this from a user-tapped "Restore Purchases" button.

## How it's wired

```
TypeScript                Rust (perry-ffi 0.5)         Swift (StoreKit 2)
-------------------       ----------------------       ----------------------
js_storekit_purchase  →   #[no_mangle] extern "C"  →   @_cdecl bridge fn
                          fn js_storekit_purchase      runs Task { … }
                          returns *mut Promise         calls back with JSON
                          ←─── promise.resolve_string(json) ←──────────
```

* `crate-ios/` — Apple-platform crate. Depends on `perry-ffi = "0.5"` (tracked at `git+https://github.com/PerryTS/perry`). Its `build.rs` compiles `swift/storekit_bridge.swift` to a static lib and links it; `package.json` lists `StoreKit` and `Foundation` so perry's link step adds `-framework`.
* `crate-stub/` — non-Apple crate. Same exported `js_storekit_*` symbol set, but every call resolves immediately with a `"not available on this platform"` payload so calling code can fall back to a Stripe/web flow without `#ifdef`-style platform checks.
* `package.json :: perry.nativeLibrary` — declares `abiVersion: "0.5"`, the FFI symbol list, and per-target `crate` / `lib` / `frameworks`.

## Server-side validation

This binding does not validate JWS receipts itself — that's plain HTTPS against [Apple's App Store Server API](https://developer.apple.com/documentation/appstoreserverapi/verify_a_transaction). A typical flow:

1. Client: `js_storekit_purchase("…")` → JWS.
2. Client → your server: `POST /verify-storekit { jws }`.
3. Server: validate signature, check `transactionId`, mark entitlement.
4. Server → client: confirmation.

Periodically poll `js_storekit_has_subscription()` (or react to `Transaction.updates`) to keep the local cache fresh.

## License

MIT
