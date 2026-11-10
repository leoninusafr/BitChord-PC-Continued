// Embed the app icon into the Windows executable (Explorer, taskbar, shortcuts).
fn main() {
    println!("cargo:rerun-if-changed=assets/icon/icon.ico");
    theme_covers();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon/icon.ico")
            .set_manifest(MANIFEST)
            .set("FileDescription", "BitChord Music Player")
            .set("ProductName", "BitChord")
            .set("ProductVersion", "2.1.0")
            .set("FileVersion", "2.1.0")
            .set("OriginalFilename", "bitchord.exe")
            .set("LegalCopyright", "GPL-3.0 License");
        res.compile()
            .expect("failed to embed Windows icon and version info");
    }
}

// The segment heap returns freed memory to the OS far more readily than the default
// NT heap, which keeps the resident footprint down after bursts (images, JSON).
const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <heapType xmlns="http://schemas.microsoft.com/SMI/2020/WindowsSettings">SegmentHeap</heapType>
    </windowsSettings>
  </application>
</assembly>
"#;

// Embed every theme preset's cover thumbnail (assets/themes/covers/<id>.jpg) as a
// table the settings page looks up by preset id.
fn theme_covers() {
    let dir = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/themes/covers");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "jpg")).collect())
        .unwrap_or_default();
    entries.sort();
    let mut out = String::from("&[\n");
    for p in entries {
        let id = p.file_stem().unwrap().to_string_lossy();
        out += &format!("    ({id:?}, include_bytes!({:?}).as_slice()),\n", p.display().to_string());
    }
    out += "]\n";
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("theme_covers.rs"), out).unwrap();
}
