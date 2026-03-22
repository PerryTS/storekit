# perry-storekit

StoreKit 2 in-app purchase bindings for Perry apps.

## Platforms

- **iOS 16+** and **macOS 13+** -- full StoreKit 2 support
- **Android** -- stub implementation (all functions return error/default JSON)

## Installation

Add as a local dependency in your Perry app:

```json
{
  "dependencies": {
    "perry-storekit": "file:../perry-storekit"
  }
}
```

Or install from npm:

```bash
npm install perry-storekit
```

## Usage

```typescript
import {
  sb_storekit_load_products,
  sb_storekit_purchase,
  sb_storekit_restore,
  sb_storekit_has_subscription,
  sb_storekit_get_jws,
  sb_storekit_start_listener,
} from "perry-storekit";

// Start the transaction listener early (handles background updates)
sb_storekit_start_listener();

// Load products by comma-separated App Store Connect product IDs
const productsJson: string = await sb_storekit_load_products(
  "com.example.pro_monthly,com.example.pro_annual"
);
const products = JSON.parse(productsJson);

// Purchase a product
const resultJson: string = await sb_storekit_purchase("com.example.pro_monthly");
const result = JSON.parse(resultJson);
if (result.success) {
  // result.jws contains the signed transaction for server validation
  // result.productId contains the purchased product ID
}

// Check if user has an active subscription
const subJson: string = await sb_storekit_has_subscription();
const sub = JSON.parse(subJson);
// sub.hasSubscription is true or false

// Get the JWS for the latest transaction (for server-side validation)
const jwsJson: string = await sb_storekit_get_jws();
const jws = JSON.parse(jwsJson);
// jws.jws is a string or null

// Restore purchases (calls AppStore.sync())
const restoreJson: string = await sb_storekit_restore();
const restore = JSON.parse(restoreJson);
// restore.success is true or false
```

## API Reference

All functions return Perry NaN-boxed promise handles. When awaited in Perry TypeScript, they resolve to JSON strings.

### `sb_storekit_load_products(productIds: string): Promise<string>`

Load products from the App Store by comma-separated product IDs. Products are cached in memory for subsequent purchase calls.

**Returns:** JSON array of product objects:
```json
[
  {
    "id": "com.example.pro_monthly",
    "displayName": "Pro Monthly",
    "description": "Unlock all features",
    "displayPrice": "$9.99",
    "price": 9.99,
    "isAnnual": false
  }
]
```

On error: `{"error": "..."}`

### `sb_storekit_purchase(productId: string): Promise<string>`

Purchase a product by its ID. The product must have been loaded first via `sb_storekit_load_products`.

**Returns:**
```json
{"success": true, "jws": "...", "productId": "...", "cancelled": false}
```

Other outcomes:
- User cancelled: `{"success": false, "cancelled": true}`
- Pending (e.g. Ask to Buy): `{"success": false, "pending": true}`
- Error: `{"error": "...", "success": false}`

### `sb_storekit_restore(): Promise<string>`

Restore purchases by calling `AppStore.sync()`.

**Returns:** `{"success": true}` or `{"error": "...", "success": false}`

### `sb_storekit_has_subscription(): Promise<string>`

Check whether the user has an active (non-revoked) subscription.

**Returns:** `{"hasSubscription": true}` or `{"hasSubscription": false}`

### `sb_storekit_get_jws(): Promise<string>`

Get the JWS (JSON Web Signature) for the latest transaction entitlement. Useful for server-side receipt validation.

**Returns:** `{"jws": "eyJ..."}` or `{"jws": null}`

### `sb_storekit_start_listener(): void`

Start a background listener for StoreKit transaction updates. Automatically finishes verified transactions. Call this once at app startup.

**Returns:** `0.0` (synchronous, no promise)

## How It Works

This package uses Perry's native library system to bridge TypeScript to native platform code:

1. **TypeScript** declares the function signatures (`src/index.ts`)
2. **Rust** (`crate-ios/src/lib.rs`) implements the FFI boundary, creating Perry promises and forwarding calls to Swift
3. **Swift** (`crate-ios/swift/storekit_bridge.swift`) calls StoreKit 2 APIs and returns results via C callbacks
4. The Rust build script (`crate-ios/build.rs`) compiles the Swift code into a static library and links it

On Android, the stub crate (`crate-stub/`) provides the same function signatures but returns error JSON immediately.

## Perry Async Model

Perry uses NaN-boxing for its value representation. All async StoreKit operations return a NaN-boxed pointer to a Perry promise. The Rust layer creates the promise via `js_promise_new()`, passes a callback to Swift, and Swift resolves the promise with a JSON string when the operation completes. In TypeScript, you simply `await` the function call.

## License

MIT
