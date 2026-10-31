//! Windows: puts the icon and the version information (what Explorer shows under Properties →
//! Details) into each .exe, `wole-link` and `wole-link-cli`: MinGW's windres for the local test
//! builds, the Windows SDK's rc.exe for the MSVC release build in CI.

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon/wole-link.ico");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    resource("wole-link", "WOL•E Link");
    resource("wole-link-cli", "WOL•E Link (command line)");
}

/// Compiles the icon and version information of the program `bin` and links them into it only.
fn resource(bin: &str, description: &str) {
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let numbers: Vec<&str> = version.split(['.', '-']).take(3).collect();
    let comma = format!("{},0", numbers.join(","));
    let icon = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/icon/wole-link.ico");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc = out.join(format!("{bin}.rc"));
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
      VALUE "FileDescription", "{description}"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "{bin}"
      VALUE "LegalCopyright", "© 2026 @celeraman. MIT License."
      VALUE "OriginalFilename", "{bin}.exe"
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
        let object = out.join(format!("{bin}-res.o"));
        let status = Command::new(&windres)
            .args(["--input-format=rc", "--output-format=coff", "-c", "65001"])
            .arg(&rc)
            .arg(&object)
            .status()
            .unwrap_or_else(|e| panic!("{windres}: {e}"));
        assert!(status.success(), "{windres} failed");
        println!("cargo:rustc-link-arg-bin={bin}={}", object.display());
    } else {
        // MSVC (the release build in CI): rc.exe from the Windows SDK, which isn't on PATH
        // outside a Developer Command Prompt.
        let rc_exe = find_rc().expect("rc.exe not found: install the Windows SDK or set RC");
        let res = out.join(format!("{bin}.res"));
        let status = Command::new(&rc_exe)
            .args(["/nologo", "/fo"])
            .arg(&res)
            .arg(&rc)
            .status()
            .unwrap_or_else(|e| panic!("{}: {e}", rc_exe.display()));
        assert!(status.success(), "rc.exe failed");
        println!("cargo:rustc-link-arg-bin={bin}={}", res.display());
    }
}

/// `RC`, then rc.exe on PATH, then the newest one in the Windows 10/11 SDK.
fn find_rc() -> Option<PathBuf> {
    if let Some(rc) = env::var_os("RC") {
        return Some(PathBuf::from(rc));
    }
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some(PathBuf::from("rc.exe"));
    }
    let arch = if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("x86") { "x86" } else { "x64" };
    let kits = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&kits)
        .ok()?
        .flatten()
        .map(|e| e.path().join(arch).join("rc.exe"))
        .filter(|p| p.is_file())
        .collect();
    // Folder names are versions (10.0.22621.0): sort them as numbers, not text.
    versions.sort_by_key(|p| {
        let name = p.parent().and_then(|a| a.parent()).and_then(|v| v.file_name()).unwrap_or_default();
        name.to_string_lossy().split('.').map(|n| n.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>()
    });
    versions.pop()
}
