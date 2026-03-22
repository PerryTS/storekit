use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let swift_dir = Path::new(&manifest_dir).join("swift");
    let swift_source = swift_dir.join("storekit_bridge.swift");

    println!("cargo:rerun-if-changed={}", swift_source.display());

    let target = env::var("TARGET").unwrap_or_default();
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Determine the swiftc -target flag and SDK path based on the Rust target triple
    let (swift_target, sdk_name) = match target.as_str() {
        // iOS device (arm64)
        t if t.contains("aarch64-apple-ios") && !t.contains("sim") && !t.contains("macabi") => {
            let ios_version = "16.0";
            (
                format!("arm64-apple-ios{}", ios_version),
                "iphoneos".to_string(),
            )
        }
        // iOS simulator (arm64)
        t if t.contains("aarch64-apple-ios-sim") => {
            let ios_version = "16.0";
            (
                format!("arm64-apple-ios{}-simulator", ios_version),
                "iphonesimulator".to_string(),
            )
        }
        // iOS simulator (x86_64)
        t if t.contains("x86_64-apple-ios") => {
            let ios_version = "16.0";
            (
                format!("x86_64-apple-ios{}-simulator", ios_version),
                "iphonesimulator".to_string(),
            )
        }
        // macOS (arm64)
        t if t.contains("aarch64-apple-darwin") => {
            let macos_version = "13.0";
            (
                format!("arm64-apple-macosx{}", macos_version),
                "macosx".to_string(),
            )
        }
        // macOS (x86_64)
        t if t.contains("x86_64-apple-darwin") => {
            let macos_version = "13.0";
            (
                format!("x86_64-apple-macosx{}", macos_version),
                "macosx".to_string(),
            )
        }
        _ => {
            // Fallback to macOS arm64
            let macos_version = "13.0";
            (
                format!("arm64-apple-macosx{}", macos_version),
                "macosx".to_string(),
            )
        }
    };

    // Get SDK path via xcrun
    let sdk_path_output = Command::new("xcrun")
        .args(["--sdk", &sdk_name, "--show-sdk-path"])
        .output()
        .expect("Failed to run xcrun to find SDK path");
    let sdk_path = String::from_utf8(sdk_path_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    // Find the Swift resource directory for the target platform
    // This helps swiftc locate the correct Swift standard library
    let swift_lib_output = Command::new("xcrun")
        .args(["--sdk", &sdk_name, "--find", "swiftc"])
        .output()
        .expect("Failed to find swiftc");
    let swiftc_path = String::from_utf8(swift_lib_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    let output_lib = PathBuf::from(&out_dir).join("libstorekit_bridge.a");

    // Compile Swift source into a static library
    let status = Command::new(&swiftc_path)
        .args([
            "-emit-library",
            "-static",
            "-module-name",
            "StoreKitBridge",
            "-target",
            &swift_target,
            "-sdk",
            &sdk_path,
            "-framework",
            "StoreKit",
            "-framework",
            "Foundation",
            "-o",
            output_lib.to_str().unwrap(),
            swift_source.to_str().unwrap(),
        ])
        .status()
        .expect("Failed to compile Swift source");

    if !status.success() {
        panic!("swiftc failed to compile storekit_bridge.swift");
    }

    // Tell cargo to link the Swift static library
    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=storekit_bridge");

    // Link Swift standard libraries
    // Find the Swift library path for the target platform
    let swift_lib_dir = if target_os == "ios" {
        // For iOS, Swift libraries are in the SDK
        format!("{}/usr/lib/swift", sdk_path)
    } else {
        // For macOS, Swift libraries are in the toolchain
        let toolchain_output = Command::new("xcrun")
            .args(["--toolchain", "default", "--find", "swift"])
            .output()
            .ok();

        if let Some(output) = toolchain_output {
            let swift_path = String::from_utf8(output.stdout).unwrap().trim().to_string();
            let toolchain_dir = Path::new(&swift_path)
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or(Path::new("/usr"));
            format!("{}/lib/swift/{}", toolchain_dir.display(), sdk_name)
        } else {
            format!("/usr/lib/swift/{}", sdk_name)
        }
    };

    if Path::new(&swift_lib_dir).exists() {
        println!("cargo:rustc-link-search=native={}", swift_lib_dir);
    }

    // Also add the platform-specific Swift library path from the SDK
    let sdk_swift_lib = format!("{}/usr/lib/swift", sdk_path);
    if Path::new(&sdk_swift_lib).exists() {
        println!("cargo:rustc-link-search=native={}", sdk_swift_lib);
    }
}
