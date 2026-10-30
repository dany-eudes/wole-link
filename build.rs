//! Windows: puts the icon and the version information (what Explorer shows under Properties →
//! Details) into the .exe. MinGW's windres compiles them; the MSVC build in CI will use rc.exe.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/wole-link.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let numbers: Vec<&str> = version.split(['.', '-']).take(3).collect();
    let comma = format!("{},0", numbers.join(","));
    let icon = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/icon/wole-link.ico");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc = out.join("wole-link.rc");
    std::fs::write(
        &rc,
        format!(
            r#"#pragma code_page(65001)
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {comma}
PRODUCTVERSION {comma}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "CompanyName", "@celeraman"
      VALUE "FileDescription", "WOL•E Link"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "wole-link"
      VALUE "LegalCopyright", "© 2026 @celeraman. MIT License."
      VALUE "OriginalFilename", "wole-link.exe"
      VALUE "ProductName", "WOL•E Link"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
            icon = icon.display().to_string().replace('\\', "/"),
        ),
    )
    .unwrap();

    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        let windres = format!("{}-w64-mingw32-windres", if arch == "x86" { "i686" } else { "x86_64" });
        let object = out.join("wole-link-res.o");
        let status = Command::new(&windres)
            .args(["--input-format=rc", "--output-format=coff", "-c", "65001"])
            .arg(&rc)
            .arg(&object)
            .status()
            .unwrap_or_else(|e| panic!("{windres}: {e}"));
        assert!(status.success(), "{windres} failed");
        println!("cargo:rustc-link-arg-bins={}", object.display());
    }
}
