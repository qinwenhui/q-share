//! Build script for the q-share GUI.
//!
//! Embeds `assets/icon.ico` as a Windows resource so the compiled
//! `qshare.exe` carries the app icon in Explorer, the taskbar, and
//! UAC prompts. No-op on every other target.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    // CARGO_CFG_WINDOWS is set whenever the *target* is Windows (even when
    // cross-compiling from another host), unlike cfg!(windows) which looks
    // at the host running the build script.
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        // Resolve against the crate dir, not the process cwd — cargo may
        // invoke this build script from anywhere (workspace root, CI, a
        // cargo-bundle app build).
        let ico = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icon.ico");
        winresource::WindowsResource::new()
            .set_icon(ico.to_str().expect("icon path is UTF-8"))
            .compile()
            .expect("failed to embed icon.ico into the Windows executable");
    }
}
