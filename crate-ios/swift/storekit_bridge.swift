import Foundation
import StoreKit

// C-compatible callback the Rust side hands us. The first argument is the
// `*mut Promise` Rust gave us as opaque context; the second is a NUL-
// terminated UTF-8 JSON payload with the call result. Rust takes ownership
// of the bytes by `String(cString:)`-equivalent (CStr → str) before it
// resolves the promise, so the lifetime of our CString only needs to
// outlast the single synchronous `callback(...)` call.
typealias StoreKitCallback = @convention(c) (UnsafeMutableRawPointer, UnsafePointer<CChar>) -> Void

// MARK: - State

actor StoreKitState {
    static let shared = StoreKitState()

    var products: [Product] = []
    var updateListenerTask: Task<Void, Error>?

    func setProducts(_ products: [Product]) {
        self.products = products
    }

    func getProduct(id: String) -> Product? {
        products.first { $0.id == id }
    }

    func startListener() {
        updateListenerTask?.cancel()
        updateListenerTask = Task.detached {
            for await result in Transaction.updates {
                if case .verified(let transaction) = result {
                    await transaction.finish()
                }
            }
        }
    }
}

// MARK: - JSON helpers

private let isoFormatter: ISO8601DateFormatter = {
    let f = ISO8601DateFormatter()
    f.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
    return f
}()

private func sendJson(_ obj: Any, _ callback: @escaping StoreKitCallback, _ context: UnsafeMutableRawPointer) {
    do {
        let data = try JSONSerialization.data(withJSONObject: obj)
        let str = String(data: data, encoding: .utf8) ?? "{}"
        str.withCString { callback(context, $0) }
    } catch {
        let fallback = "{\"error\":\"json encode failed\"}"
        fallback.withCString { callback(context, $0) }
    }
}

private func sendError(_ message: String, success: Bool? = nil, _ callback: @escaping StoreKitCallback, _ context: UnsafeMutableRawPointer) {
    var obj: [String: Any] = ["error": message]
    if let success = success { obj["success"] = success }
    sendJson(obj, callback, context)
}

// MARK: - Product type stringification

@available(iOS 15.0, macOS 12.0, *)
private func describeType(_ product: Product) -> String {
    switch product.type {
    case .consumable: return "consumable"
    case .nonConsumable: return "nonConsumable"
    case .nonRenewable: return "nonRenewable"
    case .autoRenewable: return "autoRenewable"
    default: return "unknown"
    }
}

// MARK: - Load Products

@_cdecl("swift_storekit_load_products")
func swiftStoreKitLoadProducts(
    _ productIds: UnsafePointer<CChar>,
    _ callback: @escaping StoreKitCallback,
    _ context: UnsafeMutableRawPointer
) {
    let idsString = String(cString: productIds)
    let ids = idsString.split(separator: ",").map(String.init)

    Task {
        do {
            let products = try await Product.products(for: Set(ids))
            await StoreKitState.shared.setProducts(products)

            let result = products.map { product -> [String: Any] in
                var dict: [String: Any] = [
                    "id": product.id,
                    "displayName": product.displayName,
                    "description": product.description,
                    "displayPrice": product.displayPrice,
                    "price": NSDecimalNumber(decimal: product.price).doubleValue,
                    "priceCurrencyCode": product.priceFormatStyle.currencyCode,
                    "type": describeType(product),
                ]
                if let sub = product.subscription {
                    let unit: String
                    switch sub.subscriptionPeriod.unit {
                    case .day: unit = "day"
                    case .week: unit = "week"
                    case .month: unit = "month"
                    case .year: unit = "year"
                    @unknown default: unit = "unknown"
                    }
                    dict["subscriptionPeriodUnit"] = unit
                    dict["subscriptionPeriodValue"] = sub.subscriptionPeriod.value
                }
                return dict
            }

            sendJson(result, callback, context)
        } catch {
            sendError(error.localizedDescription, callback, context)
        }
    }
}

// MARK: - Purchase

@_cdecl("swift_storekit_purchase")
func swiftStoreKitPurchase(
    _ productId: UnsafePointer<CChar>,
    _ callback: @escaping StoreKitCallback,
    _ context: UnsafeMutableRawPointer
) {
    let id = String(cString: productId)

    Task {
        guard let product = await StoreKitState.shared.getProduct(id: id) else {
            sendError("Product not found — call loadProducts first", success: false, callback, context)
            return
        }

        do {
            let result = try await product.purchase()

            switch result {
            case .success(let verification):
                switch verification {
                case .verified(let transaction):
                    await transaction.finish()
                    let payload: [String: Any] = [
                        "success": true,
                        "jws": verification.jwsRepresentation,
                        "productId": transaction.productID,
                        "transactionId": String(transaction.id),
                        "purchaseDate": isoFormatter.string(from: transaction.purchaseDate),
                        "cancelled": false,
                    ]
                    sendJson(payload, callback, context)
                case .unverified(_, let error):
                    sendError("Verification failed: \(error.localizedDescription)", success: false, callback, context)
                }
            case .userCancelled:
                sendJson(["success": false, "cancelled": true], callback, context)
            case .pending:
                sendJson(["success": false, "pending": true], callback, context)
            @unknown default:
                sendError("Unknown purchase result", success: false, callback, context)
            }
        } catch {
            sendError(error.localizedDescription, success: false, callback, context)
        }
    }
}

// MARK: - Restore Purchases

@_cdecl("swift_storekit_restore")
func swiftStoreKitRestore(
    _ callback: @escaping StoreKitCallback,
    _ context: UnsafeMutableRawPointer
) {
    Task {
        do {
            try await AppStore.sync()
            sendJson(["success": true], callback, context)
        } catch {
            sendError(error.localizedDescription, success: false, callback, context)
        }
    }
}

// MARK: - Active Subscription Check

@_cdecl("swift_storekit_has_subscription")
func swiftStoreKitHasSubscription(
    _ callback: @escaping StoreKitCallback,
    _ context: UnsafeMutableRawPointer
) {
    Task {
        var hasActive = false
        for await result in Transaction.currentEntitlements {
            if case .verified(let transaction) = result {
                if transaction.revocationDate == nil {
                    hasActive = true
                    break
                }
            }
        }
        sendJson(["hasSubscription": hasActive], callback, context)
    }
}

// MARK: - Latest JWS

@_cdecl("swift_storekit_get_jws")
func swiftStoreKitGetJws(
    _ callback: @escaping StoreKitCallback,
    _ context: UnsafeMutableRawPointer
) {
    Task {
        var latestJWS: String? = nil
        for await result in Transaction.currentEntitlements {
            if case .verified(_) = result {
                latestJWS = result.jwsRepresentation
            }
        }

        if let jws = latestJWS {
            sendJson(["jws": jws], callback, context)
        } else {
            // JSONSerialization needs NSNull for an explicit JSON null.
            sendJson(["jws": NSNull()], callback, context)
        }
    }
}

// MARK: - Transaction Updates Listener

@_cdecl("swift_storekit_start_listener")
func swiftStoreKitStartListener() {
    Task {
        await StoreKitState.shared.startListener()
    }
}
