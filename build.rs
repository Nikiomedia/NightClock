#[path = "src/icon.rs"]
mod icon;

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/icon.rs");
    println!("cargo:rerun-if-changed=build.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let ico_path = out_dir.join("nightclock.ico");
    let ico = icon::encode_ico(&[16, 20, 24, 32, 40, 48, 64, 128, 256]);
    std::fs::write(&ico_path, ico).expect("Icon schreiben");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico_path.to_str().expect("Pfad"));
    res.set("FileDescription", "NightClock - Nachtuhr mit Wecker");
    res.set("ProductName", "NightClock");
    res.set("LegalCopyright", "NightClock");
    if let Err(e) = res.compile() {
        println!("cargo:warning=Icon konnte nicht eingebettet werden: {e}");
    }
}
