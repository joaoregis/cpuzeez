//! Windows only: embed the app icon and version info in cpuzeez.exe, so
//! Explorer, the taskbar and "Apps & features" show the CpuZeeZ logo.

fn main() {
    println!("cargo:rerun-if-changed=assets/cpuzeez.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/cpuzeez.ico")
        .set("ProductName", "CpuZeeZ")
        .set("FileDescription", "CpuZeeZ - CPU, mainboard, memory and GPU information")
        .set("OriginalFilename", "cpuzeez.exe")
        .set("LegalCopyright", "MIT OR Apache-2.0");
    if let Err(e) = res.compile() {
        // Building without the Windows SDK still works, just without the icon.
        println!("cargo:warning=no icon embedded: {e}");
    }
}
