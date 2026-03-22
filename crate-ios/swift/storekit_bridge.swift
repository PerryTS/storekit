import Foundation
import StoreKit

// Type alias for C callback function pointer
typealias StoreKitCallback = @convention(c) (UnsafeMutableRawPointer, UnsafePointer<CChar>) -> Void

// Global actor for managing StoreKit state
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
                [
                    "id": product.id,
                    "displayName": product.displayName,
                    "description": product.description,
                    "displayPrice": product.displayPrice,
                    "price": NSDecimalNumber(decimal: product.price).doubleValue,
                    "isAnnual": product.id.contains("ANNUAL")
                ]
            }

            let jsonData = try JSONSerialization.data(withJSONObject: result)
            let jsonString = String(data: jsonData, encoding: .utf8) ?? "[]"
            jsonString.withCString { callback(context, $0) }
        } catch {
            let errorJson = "{\"error\":\"\(error.localizedDescription)\"}"
            errorJson.withCString { callback(context, $0) }
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
            let err = "{\"error\":\"Product not found\",\"success\":false}"
            err.withCString { callback(context, $0) }
            return
        }

        do {
            let result = try await product.purchase()

            switch result {
            case .success(let verification):
                switch verification {
                case .verified(let transaction):
                    await transaction.finish()
                    let jws = verification.jwsRepresentation
                    let json = "{\"success\":true,\"jws\":\"\(jws)\",\"productId\":\"\(transaction.productID)\",\"cancelled\":false}"
                    json.withCString { callback(context, $0) }
                case .unverified(_, let error):
                    let err = "{\"error\":\"Verification failed: \(error.localizedDescription)\",\"success\":false}"
                    err.withCString { callback(context, $0) }
                }
            case .userCancelled:
                let json = "{\"success\":false,\"cancelled\":true}"
                json.withCString { callback(context, $0) }
            case .pending:
                let json = "{\"success\":false,\"pending\":true}"
                json.withCString { callback(context, $0) }
            @unknown default:
                let err = "{\"error\":\"Unknown result\",\"success\":false}"
                err.withCString { callback(context, $0) }
            }
        } catch {
            let err = "{\"error\":\"\(error.localizedDescription)\",\"success\":false}"
            err.withCString { callback(context, $0) }
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
            let json = "{\"success\":true}"
            json.withCString { callback(context, $0) }
        } catch {
            let err = "{\"error\":\"\(error.localizedDescription)\",\"success\":false}"
            err.withCString { callback(context, $0) }
        }
    }
}

// MARK: - Check Active Subscription

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
        let json = "{\"hasSubscription\":\(hasActive)}"
        json.withCString { callback(context, $0) }
    }
}

// MARK: - Get Latest JWS

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
            let json = "{\"jws\":\"\(jws)\"}"
            json.withCString { callback(context, $0) }
        } else {
            let json = "{\"jws\":null}"
            json.withCString { callback(context, $0) }
        }
    }
}

// MARK: - Start Transaction Listener

@_cdecl("swift_storekit_start_listener")
func swiftStoreKitStartListener() {
    Task {
        await StoreKitState.shared.startListener()
    }
}
