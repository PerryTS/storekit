use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Non-Apple target: lib.rs compiles the stub fallback instead of the
    // Swift bridge, so there is nothing to build or link here. This keeps
    // the crate buildable on hosts without Xcode/swiftc (e.g. Linux CI).
    if target_os != "ios" && target_os != "macos" && target_os != "tvos" {
        return;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let swift_source = Path::new(&manifest_dir)
        .join("swift")
        .join("storekit_bridge.swift");

    println!("cargo:rerun-if-changed={}", swift_source.display());
    println!("cargo:rerun-if-env-changed=PERRY_STOREKIT_BRIDGE");

    let target = env::var("TARGET").unwrap_or_default();

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
        // Apple TV device (arm64)
        t if t.contains("aarch64-apple-tvos") && !t.contains("sim") => {
            let tvos_version = "16.0";
            (
                format!("arm64-apple-tvos{}", tvos_version),
                "appletvos".to_string(),
            )
        }
        // Apple TV simulator (arm64)
        t if t.contains("aarch64-apple-tvos-sim") => {
            let tvos_version = "16.0";
            (
                format!("arm64-apple-tvos{}-simulator", tvos_version),
                "appletvsimulator".to_string(),
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

    // Decide whether to compile the Swift bridge from source (needs Xcode's
    // swiftc + Apple SDK, i.e. a macOS host) or link the prebuilt archive
    // shipped under prebuilt/<rust-triple>/ (works on any host, which is how
    // Linux builders produce iOS/macOS libraries).
    //
    // PERRY_STOREKIT_BRIDGE=source|prebuilt forces one path; otherwise we
    // autodetect by asking xcrun for a swiftc that can target the SDK.
    let use_source = match env::var("PERRY_STOREKIT_BRIDGE").as_deref() {
        Ok("source") => true,
        Ok("prebuilt") => false,
        _ => swiftc_for_sdk(&sdk_name).is_some(),
    };

    if use_source {
        compile_bridge_from_source(&swift_source, &swift_target, &sdk_name, &target_os);
    } else {
        link_prebuilt_bridge(&manifest_dir, &target);
    }

    println!("cargo:rustc-link-lib=static=storekit_bridge");
}

/// Locate a swiftc able to target `sdk_name` via xcrun. Returns `None` on
/// hosts without Xcode (or without that SDK), which routes the build to the
/// prebuilt bridge archive.
fn swiftc_for_sdk(sdk_name: &str) -> Option<String> {
    let output = Command::new("xcrun")
        .args(["--sdk", sdk_name, "--find", "swiftc"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(path)
    }
}

fn link_prebuilt_bridge(manifest_dir: &str, target: &str) {
    let prebuilt_dir = Path::new(manifest_dir).join("prebuilt").join(target);
    let prebuilt_lib = prebuilt_dir.join("libstorekit_bridge.a");

    if !prebuilt_lib.exists() {
        panic!(
            "No Swift toolchain for Apple targets on this host, and no prebuilt \
             StoreKit bridge at {}. Published @perryts/storekit packages ship the \
             prebuilt archives; when building from a git checkout, run \
             scripts/build-bridges.sh on a Mac (or set PERRY_STOREKIT_BRIDGE=source \
             on a macOS host with Xcode).",
            prebuilt_lib.display()
        );
    }

    println!("cargo:rerun-if-changed={}", prebuilt_lib.display());
    println!("cargo:rustc-link-search=native={}", prebuilt_dir.display());
}

fn compile_bridge_from_source(
    swift_source: &Path,
    swift_target: &str,
    sdk_name: &str,
    target_os: &str,
) {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Get SDK path via xcrun
    let sdk_path_output = Command::new("xcrun")
        .args(["--sdk", sdk_name, "--show-sdk-path"])
        .output()
        .expect("Failed to run xcrun to find SDK path");
    let sdk_path = String::from_utf8(sdk_path_output.stdout)
        .unwrap()
        .trim()
        .to_string();

    let swiftc_path = swiftc_for_sdk(sdk_name).expect("Failed to find swiftc");

    let output_lib = PathBuf::from(&out_dir).join("libstorekit_bridge.a");

    // Compile Swift source into a static library
    let status = Command::new(&swiftc_path)
        .args([
            "-emit-library",
            "-static",
            "-module-name",
            "StoreKitBridge",
            "-target",
            swift_target,
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

    // Tell cargo where to find the Swift static library
    println!("cargo:rustc-link-search=native={}", out_dir);

    // Link Swift standard libraries
    // Find the Swift library path for the target platform
    let swift_lib_dir = if target_os == "ios" || target_os == "tvos" {
        // For iOS and tvOS, Swift libraries are in the SDK
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
