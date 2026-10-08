//! Resolves and links against a prebuilt Pollux Core ABI artifact.
//!
//! This crate never builds Pollux Core from source, and never depends on
//! a local Core checkout or a vendored binary. The only supported source
//! is the published `pollux-core-<CORE_VERSION>-<target>` artifact from
//! the public `sxnnyside-project/pollux-core` release matching
//! `CORE_VERSION` — this is a binary/ABI dependency on that release, not
//! a Cargo source dependency on Core itself.
//!
//! See `ARTIFACT_LAYOUT.md` for the artifact shape this expects.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The exactly one Core artifact version this build targets. Bumped
/// deliberately, in its own change, never inferred or left to "latest".
const CORE_VERSION: &str = "0.1.0";
const DIST_REPO: &str = "sxnnyside-project/pollux-core-distribution";
/// The ABI version `src/ffi.rs`'s declarations were hand-written against.
/// Kept in sync with `pollux_ffi::ABI_VERSION` in Pollux Core by hand —
/// mirrors the same "declared here once, verified against reality"
/// pattern `src/ffi.rs`'s own doc comment describes.
const EXPECTED_ABI_VERSION: &str = "pollux-abi/1";

#[derive(Clone, Copy)]
struct TargetInfo {
    lib_file: &'static str,
    archive_ext: &'static str,
}

const SUPPORTED_TARGETS: &[(&str, TargetInfo)] = &[
    (
        "x86_64-unknown-linux-gnu",
        TargetInfo {
            lib_file: "libpollux_ffi.so",
            archive_ext: "tar.gz",
        },
    ),
    (
        "aarch64-unknown-linux-gnu",
        TargetInfo {
            lib_file: "libpollux_ffi.so",
            archive_ext: "tar.gz",
        },
    ),
    (
        "x86_64-apple-darwin",
        TargetInfo {
            lib_file: "libpollux_ffi.dylib",
            archive_ext: "tar.gz",
        },
    ),
    (
        "aarch64-apple-darwin",
        TargetInfo {
            lib_file: "libpollux_ffi.dylib",
            archive_ext: "tar.gz",
        },
    ),
    (
        "x86_64-pc-windows-msvc",
        TargetInfo {
            lib_file: "pollux_ffi.dll",
            archive_ext: "zip",
        },
    ),
];

fn target_info(target: &str) -> TargetInfo {
    match SUPPORTED_TARGETS.iter().find(|(name, _)| *name == target) {
        Some((_, info)) => *info,
        None => {
            let supported: Vec<&str> = SUPPORTED_TARGETS.iter().map(|(name, _)| *name).collect();
            panic!(
                "pollux-polyglot-native-bridge does not support target `{target}`.\n\
                 Supported targets: {}.\n\
                 Expected artifact: pollux-core-{CORE_VERSION}-{target}.<tar.gz|zip> \
                 (does not exist for this target).",
                supported.join(", ")
            );
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=POLLUX_CORE_DIR");
    let target = env::var("TARGET").expect("cargo always sets TARGET for build scripts");
    let info = target_info(&target);
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let artifact_dir = if let Ok(local_dir) = env::var("POLLUX_CORE_DIR") {
        let dir = PathBuf::from(local_dir);
        validate_artifact_dir(&dir, &info, "POLLUX_CORE_DIR");
        dir
    } else {
        download_artifact(&manifest_dir, &target, &info)
    };

    // Core's published macOS dylib carries its CI build machine's absolute
    // path as its own install name (`otool -D` on the real v0.1.0 release
    // artifact confirms this), not a relocatable `@rpath/...` one. `-rpath`
    // only helps a reference shaped `@rpath/foo.dylib` — an absolute,
    // wrong-machine path ignores it entirely and dyld fails to load it.
    // Stage a copy and patch its ID rather than mutate the downloaded
    // artifact directory.
    let lib_dir = stage_and_patch_lib(&artifact_dir, &info, &target);

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    if !target.contains("windows") {
        println!("cargo:rustc-link-lib=dylib=pollux_ffi");
    }
    if cfg!(unix) {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }
    println!("cargo:rerun-if-changed={}", artifact_dir.display());
}

/// Copies the resolved library into `OUT_DIR` and, on macOS, rewrites its
/// install name to `@rpath/<lib>` so it can actually be found at runtime
/// via the rpath we add — see the call site for why this is necessary.
fn stage_and_patch_lib(artifact_dir: &Path, info: &TargetInfo, target: &str) -> PathBuf {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("cargo always sets OUT_DIR"));
    let staged_dir = out_dir.join("lib");
    fs::create_dir_all(&staged_dir).expect("failed to create staged lib directory");

    let src = artifact_dir.join("lib").join(info.lib_file);
    let dst = staged_dir.join(info.lib_file);
    fs::copy(&src, &dst).unwrap_or_else(|e| {
        panic!(
            "failed to stage {} to {}: {e}",
            src.display(),
            dst.display()
        )
    });

    if target.contains("apple-darwin") {
        let status = Command::new("install_name_tool")
            .args(["-id", &format!("@rpath/{}", info.lib_file)])
            .arg(&dst)
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => panic!(
                "install_name_tool failed ({s}) patching the install name of {}",
                dst.display()
            ),
            Err(e) => panic!(
                "could not run `install_name_tool` to patch {}: {e}",
                dst.display()
            ),
        }
    } else if target.contains("windows") {
        // Copy DLL to cargo target profile and deps dirs so tests and binaries find it at runtime.
        if let Some(profile_dir) = out_dir.ancestors().nth(3) {
            let deps_dir = profile_dir.join("deps");
            let _ = fs::copy(&dst, profile_dir.join(info.lib_file));
            let _ = fs::copy(&dst, deps_dir.join(info.lib_file));
        }
    }

    staged_dir
}

/// Fails the build clearly and specifically: which file is missing,
/// which target/version was expected, never a generic "not found".
fn validate_artifact_dir(dir: &Path, info: &TargetInfo, source: &str) {
    if !dir.is_dir() {
        panic!(
            "Pollux Core artifact directory not found at {} (from {source}).",
            dir.display()
        );
    }
    let header = dir.join("include/pollux.h");
    if !header.is_file() {
        panic!(
            "Pollux Core artifact at {} (from {source}) is missing include/pollux.h. \
             The archive layout is incorrect — see ARTIFACT_LAYOUT.md.",
            dir.display()
        );
    }
    let lib = dir.join("lib").join(info.lib_file);
    if !lib.is_file() {
        panic!(
            "Pollux Core artifact at {} (from {source}) is missing lib/{}. \
             The archive layout is incorrect — see ARTIFACT_LAYOUT.md.",
            dir.display(),
            info.lib_file
        );
    }

    let sums_path = dir.join("SHA256SUMS");
    if sums_path.is_file() {
        verify_checksums(dir, &sums_path);
    }

    let abi_path = dir.join("abi");
    if abi_path.is_file() {
        let declared_abi = fs::read_to_string(&abi_path)
            .unwrap_or_else(|e| panic!("could not read {}: {e}", abi_path.display()))
            .trim()
            .to_string();
        if declared_abi != EXPECTED_ABI_VERSION {
            panic!(
                "Pollux Core artifact at {} (from {source}) declares ABI `{declared_abi}`, \
                 but pollux-polyglot-native-bridge's src/ffi.rs was written against \
                 `{EXPECTED_ABI_VERSION}`. Refusing to link against an incompatible ABI.",
                dir.display()
            );
        }
    }

    // The downloaded artifact directory is always named per LAYOUT.md's
    // convention by construction, so this check always applies here.
    if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
        if let Some(rest) = name.strip_prefix("pollux-core-") {
            if let Some(version) = rest.split('-').next() {
                if version != CORE_VERSION {
                    panic!(
                        "Pollux Core artifact directory {} (from {source}) is named for \
                         version {version}, but this build is pinned to CORE_VERSION \
                         {CORE_VERSION}. Refusing to link against an unexpected Core version.",
                        dir.display()
                    );
                }
            }
        }
    }
}

/// Verifies every file listed in `SHA256SUMS` (relative to `dir`) against
/// its actual SHA-256 digest. Fails loudly and names the exact file on any
/// mismatch or missing entry — never trusts an artifact merely because it
/// is present.
fn verify_checksums(dir: &Path, sums_path: &Path) {
    let sums = fs::read_to_string(sums_path)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", sums_path.display()));

    for line in sums.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((expected_hex, rel_path)) = line.split_once("  ") else {
            panic!(
                "malformed SHA256SUMS line in {}: `{line}`",
                sums_path.display()
            );
        };
        let file_path = dir.join(rel_path);
        let bytes = fs::read(&file_path).unwrap_or_else(|e| {
            panic!(
                "SHA256SUMS in {} lists `{rel_path}`, but it could not be read at {}: {e}",
                sums_path.display(),
                file_path.display()
            )
        });
        let actual_hex = sha256_hex(&bytes);
        if actual_hex != expected_hex {
            panic!(
                "checksum mismatch for `{rel_path}` under {}: expected {expected_hex}, got {actual_hex}. \
                 Refusing to link against a Core artifact that does not match its published checksum.",
                dir.display()
            );
        }
    }
}

/// Downloads and extracts `pollux-core-<CORE_VERSION>-<target>` from the
/// public distribution repository's `v<CORE_VERSION>` release, then
/// verifies it exactly as a local artifact would be.
fn download_artifact(manifest_dir: &Path, target: &str, info: &TargetInfo) -> PathBuf {
    let package_name = format!("pollux-core-{CORE_VERSION}-{target}");
    let archive_name = format!("{package_name}.{}", info.archive_ext);
    let url =
        format!("https://github.com/{DIST_REPO}/releases/download/v{CORE_VERSION}/{archive_name}");

    let cache_dir = manifest_dir.join("target").join("pollux-core-dist-cache");
    fs::create_dir_all(&cache_dir).expect("failed to create download cache directory");
    let archive_path = cache_dir.join(&archive_name);
    let extracted_dir = cache_dir.join(&package_name);

    if extracted_dir.join("lib").is_dir() {
        // Already downloaded and extracted by a previous build in this
        // target/ directory; still re-verify before trusting it.
        validate_artifact_dir(&extracted_dir, info, "downloaded cache");
        return extracted_dir;
    }

    println!("cargo:warning=downloading Pollux Core artifact from {url}");
    let status = Command::new("curl")
        .args(["-fL", "--retry", "3", "-o"])
        .arg(&archive_path)
        .arg(&url)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!(
            "failed to download {url} (curl exited with {s}). \
             Requested Core version v{CORE_VERSION} for target {target} may not exist \
             in {DIST_REPO}'s releases."
        ),
        Err(e) => panic!("could not run `curl` to download {url}: {e}."),
    }

    fs::create_dir_all(&extracted_dir).expect("failed to create extraction directory");
    match info.archive_ext {
        "tar.gz" => {
            let status = Command::new("tar")
                .args(["-xzf"])
                .arg(&archive_path)
                .args(["-C"])
                .arg(&cache_dir)
                .status()
                .expect("failed to run `tar` to extract the downloaded archive");
            if !status.success() {
                panic!("`tar` failed to extract {}", archive_path.display());
            }
        }
        "zip" => {
            let status = Command::new("unzip")
                .args(["-o"])
                .arg(&archive_path)
                .args(["-d"])
                .arg(&cache_dir)
                .status()
                .expect("failed to run `unzip` to extract the downloaded archive");
            if !status.success() {
                panic!("`unzip` failed to extract {}", archive_path.display());
            }
        }
        other => unreachable!("unhandled archive extension {other}"),
    }

    validate_artifact_dir(&extracted_dir, info, "downloaded artifact");
    extracted_dir
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
