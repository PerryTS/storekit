// StoreKit 2 bindings for Perry — FFI surface.
//
// These ambient declarations match the symbols listed in
// `package.json :: perry.nativeLibrary.functions`. The perry compiler
// resolves imports of these names directly against the linked
// `libperry_storekit.a` (Swift bridge on Apple platforms, stub
// elsewhere); there is no JS implementation in this file.
//
// Each native call returns a `Promise<string>` carrying a JSON
// payload. The shape of the payload is documented per-function below
// and matches the typed `Product` / `PurchaseResult` aliases at the
// bottom of this file. Most apps will want a small wrapper layer on
// top — see the README for a copy-pasteable example.
//
// Closes PerryTS/perry#537.

// ── Native FFI ──────────────────────────────────────────────────────

/**
 * Load StoreKit products by ID. Pass IDs as a comma-separated string
 * (e.g. `"com.example.pro_monthly,com.example.pro_annual"`) — the Swift
 * side splits on `,`. Resolves with a JSON-encoded `Product[]` on
 * success or `{"error": "..."}` on failure.
 */
export declare function js_storekit_load_products(productIds: string): Promise<string>;

/**
 * Drive the StoreKit purchase sheet for a single product. The product
 * must already have been fetched with `js_storekit_load_products` —
 * StoreKit 2 requires the in-memory `Product` value to call
 * `purchase()`, so this binding caches loaded products in Swift and
 * looks them up by ID. Resolves with a JSON `PurchaseResult`.
 */
export declare function js_storekit_purchase(productId: string): Promise<string>;

/**
 * Call `AppStore.sync()` to restore previous purchases. Resolves with
 * `{"success": true}` or `{"error": "...", "success": false}`. Apple
 * recommends only invoking this from a user-tapped "Restore Purchases"
 * button, not on launch.
 */
export declare function js_storekit_restore(): Promise<string>;

/**
 * Check whether the user has at least one verified, non-revoked
 * entitlement. Resolves with `{"hasSubscription": boolean}`. For server-
 * side validation, use `js_storekit_get_jws` instead.
 */
export declare function js_storekit_has_subscription(): Promise<string>;

/**
 * Return the most recent verified entitlement's JWS — the signed token
 * a server-side validator feeds to Apple's App Store Server API.
 * Resolves with `{"jws": "..."}`, or `{"jws": null}` if there is no
 * active entitlement.
 */
export declare function js_storekit_get_jws(): Promise<string>;

/**
 * Start the `Transaction.updates` background task. This finishes
 * verified transactions arriving outside an explicit `purchase()` call
 * — Ask-to-Buy approval, family-shared entitlements, auto-renew, etc.
 * Call this exactly once at app launch.
 */
export declare function js_storekit_start_listener(): void;

// ── Payload types ───────────────────────────────────────────────────
//
// These are the shapes the JSON strings above parse into. Importing
// them lets callers type their own thin wrappers without redeclaring
// the contract.

/** Product subscription period unit (StoreKit 2's `Product.SubscriptionPeriod.Unit`). */
export type SubscriptionPeriodUnit = "day" | "week" | "month" | "year" | "unknown";

/** StoreKit product type — see `Product.ProductType`. */
export type ProductType =
  | "consumable"
  | "nonConsumable"
  | "nonRenewable"
  | "autoRenewable"
  | "unknown";

/** A single product, as JSON-encoded by `js_storekit_load_products`. */
export type Product = {
  /** App Store Connect product ID, e.g. `com.example.pro_monthly`. */
  id: string;
  /** Localised display name configured in App Store Connect. */
  displayName: string;
  /** Localised description. */
  description: string;
  /** Localised price string with currency symbol (`"$9.99"`). */
  displayPrice: string;
  /** Numeric price in the currency below. */
  price: number;
  /** ISO 4217 currency code (`"USD"`). */
  priceCurrencyCode: string;
  /** Product type. */
  type: ProductType;
  /** For subscription products, the renewal period unit. Absent for one-shot products. */
  subscriptionPeriodUnit?: SubscriptionPeriodUnit;
  /** For subscription products, the renewal period count (e.g. `1` month). */
  subscriptionPeriodValue?: number;
};

/**
 * Result of a `js_storekit_purchase` call. `success: true` means a
 * verified transaction was finished; `success: false` covers user
 * cancellation (`cancelled: true`), pending parental approval
 * (`pending: true`), and errors (`error: "..."`).
 */
export type PurchaseResult =
  | {
      success: true;
      /** App Store-issued JWS for server-side validation. */
      jws: string;
      productId: string;
      /** StoreKit 2 transaction ID (decimal string). */
      transactionId: string;
      /** ISO 8601 purchase timestamp. */
      purchaseDate: string;
      cancelled: false;
    }
  | {
      success: false;
      cancelled?: boolean;
      pending?: boolean;
      error?: string;
    };

/** Result of `js_storekit_has_subscription`. */
export type HasSubscriptionResult = { hasSubscription: boolean };

/** Result of `js_storekit_get_jws`. `null` when no active entitlement exists. */
export type JwsResult = { jws: string | null };

/** Result of `js_storekit_restore`. */
export type RestoreResult = { success: boolean; error?: string };
