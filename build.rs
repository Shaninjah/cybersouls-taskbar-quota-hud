use winres::{VersionInfo, WindowsResource};

fn main() {
    let version = env!("CARGO_PKG_VERSION");

    // Embed the icon and richer PE version metadata into the executable.
    let mut res = WindowsResource::new();
    let numeric_version = pack_version(version);

    // winres otherwise looks for an unprefixed host windres during cross-builds.
    let target = std::env::var("TARGET").unwrap_or_default();
    if target == "x86_64-pc-windows-gnu" && std::env::var("HOST").unwrap_or_default() != target {
        res.set_windres_path("x86_64-w64-mingw32-windres")
            .set_ar_path("x86_64-w64-mingw32-ar");
    }

    res.set_icon("src/icons/icon.ico")
        .set("FileVersion", version)
        .set("ProductVersion", version)
        .set_version_info(VersionInfo::FILEVERSION, numeric_version)
        .set_version_info(VersionInfo::PRODUCTVERSION, numeric_version);

    res.compile().expect("Failed to compile Windows resources");

    if target.ends_with("-windows-gnu") {
        // The resource archive has no referenced symbols, so GNU linkers can
        // omit it. Link its object explicitly to retain the icon and VERSIONINFO.
        let resource =
            std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("resource.o");
        println!("cargo:rustc-link-arg={}", resource.display());
    }
}

fn pack_version(version: &str) -> u64 {
    let core = version.split('-').next().unwrap_or(version);
    let mut parts = core.split('.').map(|part| part.parse::<u64>().unwrap_or(0));

    let major = parts.next().unwrap_or(0).min(u16::MAX as u64);
    let minor = parts.next().unwrap_or(0).min(u16::MAX as u64);
    let patch = parts.next().unwrap_or(0).min(u16::MAX as u64);

    (major << 48) | (minor << 32) | (patch << 16)
}
