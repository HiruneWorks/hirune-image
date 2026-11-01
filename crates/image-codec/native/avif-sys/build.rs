use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tar::Archive;

const ENV_CMAKE: &str = "CMAKE";
const ENV_PREBUILT_ONLY: &str = "JC_LIBAVIF_SYS_PREBUILT_ONLY";
const ENV_NO_PREBUILT: &str = "JC_LIBAVIF_SYS_NO_PREBUILT";
const ENV_PREBUILT_BASE_URL: &str = "JC_LIBAVIF_SYS_PREBUILT_BASE_URL";
const ENV_DOCS_RS: &str = "DOCS_RS";

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("missing CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("missing OUT_DIR"));
    let versions = read_versions(&manifest_dir.join("upstream/versions.toml"));
    let target = env::var("TARGET").expect("missing TARGET");
    let build_mode = BuildMode::from_env();

    emit_rerun_env(ENV_CMAKE);
    emit_rerun_env("HIRUNE_NATIVE_SOURCE_CACHE");
    emit_rerun_env(ENV_PREBUILT_ONLY);
    emit_rerun_env(ENV_NO_PREBUILT);
    emit_rerun_env(ENV_PREBUILT_BASE_URL);
    emit_rerun_env(ENV_DOCS_RS);
    emit_rerun_rules(&manifest_dir.join("build.rs"));
    emit_rerun_rules(&manifest_dir.join("upstream/versions.toml"));

    println!(
        "cargo:rustc-env=JC_LIBAVIF_SYS_UPSTREAM_LIBAVIF_VERSION={}",
        versions.libavif
    );
    println!(
        "cargo:rustc-env=JC_LIBAVIF_SYS_UPSTREAM_LIBAOM_VERSION={}",
        versions.libaom
    );

    if env_flag(ENV_DOCS_RS) {
        return;
    }

    let source_dir = out_dir.join("libavif-src");
    let aom_build_dir = source_dir.join("ext/aom/build.libavif");
    let build_dir = out_dir.join("libavif-build");
    let install_dir = out_dir.join("libavif-install");
    let download_dir = out_dir.join("libavif-downloads");

    recreate_dir(&install_dir).expect("failed to prepare libavif install directory");

    let cmake = cmake_command();
    let built_from_source = match build_mode {
        BuildMode::PrebuiltOnly => {
            fetch_prebuilt(&install_dir, &target)
                .unwrap_or_else(|error| panic!("prebuilt-only mode failed: {error}"));
            false
        }
        BuildMode::SourceOnly => {
            let cmake = cmake.unwrap_or_else(|error| {
                panic!("source-only mode requires a working cmake executable: {error}")
            });
            build_from_downloaded_sources(
                &versions,
                &source_dir,
                &download_dir,
                &aom_build_dir,
                &build_dir,
                &install_dir,
                &cmake,
            );
            true
        }
        BuildMode::PreferPrebuilt => match fetch_prebuilt(&install_dir, &target) {
            Ok(()) => false,
            Err(prebuilt_error) => {
                println!(
                    "cargo:warning=prebuilt artifact unavailable, falling back to source build: {prebuilt_error}"
                );
                let cmake = cmake.unwrap_or_else(|cmake_error| {
                    panic!(
                        "prebuilt fallback failed ({prebuilt_error}) and source build is unavailable ({cmake_error})"
                    )
                });
                build_from_downloaded_sources(
                    &versions,
                    &source_dir,
                    &download_dir,
                    &aom_build_dir,
                    &build_dir,
                    &install_dir,
                    &cmake,
                );
                true
            }
        },
    };

    if built_from_source {
        println!(
            "cargo:warning=jc-libavif-sys downloaded upstream sources and built them with cmake"
        );
    } else {
        println!("cargo:warning=jc-libavif-sys used a verified prebuilt native archive");
    }

    emit_link_directives(&install_dir);
    if built_from_source {
        println!("cargo:rustc-link-search=native={}", aom_build_dir.display());
        println!("cargo:rustc-link-lib=static=aom");
    }
}

fn build_from_downloaded_sources(
    versions: &Versions,
    source_dir: &Path,
    download_dir: &Path,
    aom_build_dir: &Path,
    build_dir: &Path,
    install_dir: &Path,
    cmake: &OsStr,
) {
    download_upstream_sources(versions, source_dir, download_dir);
    recreate_dir(build_dir).expect("failed to prepare libavif build directory");
    // NASM 3 changed its help topics. Both NASM 2 and 3 expose -Ox in the full help.
    let nasm_check = source_dir.join("ext/aom/build/cmake/aom_optimization.cmake");
    let content = fs::read_to_string(&nasm_check).expect("libaom NASM configuration");
    fs::write(&nasm_check, content.replace("CMAKE_ASM_NASM_COMPILER} -hf", "CMAKE_ASM_NASM_COMPILER} -h all")).expect("NASM help compatibility patch");
    build_libaom(&source_dir.join("ext/aom"), aom_build_dir, cmake);
    let msvc_library = aom_build_dir.join("Release/aom.lib");
    if msvc_library.exists() {
        fs::copy(&msvc_library, aom_build_dir.join("aom.lib")).expect("stage MSVC local libaom");
    }
    configure_and_build(source_dir, build_dir, install_dir, cmake);
    copy_upstream_notices(versions, source_dir, install_dir);
}

fn download_upstream_sources(versions: &Versions, source_dir: &Path, download_dir: &Path) {
    if source_dir.exists() {
        check_output_path(source_dir).expect("source directory stays within OUT_DIR");
        fs::remove_dir_all(source_dir).expect("failed to clean staged source directory");
    }
    recreate_dir(download_dir).expect("failed to prepare upstream download directory");

    download_and_unpack_to(
        &libavif_source_url(&versions.libavif),
        &libavif_source_dir_name(&versions.libavif),
        source_dir,
        download_dir,
        "libavif",
    )
    .unwrap_or_else(|error| panic!("failed to stage libavif sources: {error}"));

    download_and_unpack_to(
        &libaom_source_url(&versions.libaom),
        &libaom_source_dir_name(&versions.libaom),
        &source_dir.join("ext/aom"),
        download_dir,
        "libaom",
    )
    .unwrap_or_else(|error| panic!("failed to stage libaom sources: {error}"));
}

fn download_and_unpack_to(
    url: &str,
    extracted_dir_name: &str,
    destination: &Path,
    scratch_root: &Path,
    package: &str,
) -> Result<(), String> {
    let expected = match package {
        "libavif" => "0a545e953cc049bf5bcf4ee467306a2f113a75110edf59e61248873101cd26c1",
        "libaom" => "19e45a5a7192d690565229983dad900e76b513a02306c12053fb9a262cbeca7d",
        _ => return Err("unknown native source archive".into()),
    };
    let cache = env::var_os("HIRUNE_NATIVE_SOURCE_CACHE").map(PathBuf::from).unwrap_or_else(|| scratch_root.to_path_buf());
    fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let archive_path = cache.join(format!("{package}-{expected}.tar.gz"));
    let unpack_dir = scratch_root.join(format!("{package}-unpack"));

    if unpack_dir.exists() {
        check_output_path(&unpack_dir).map_err(|e| e.to_string())?;
        fs::remove_dir_all(&unpack_dir).map_err(|error| {
            format!(
                "failed to remove stale unpack directory {}: {error}",
                unpack_dir.display()
            )
        })?;
    }
    fs::create_dir_all(&unpack_dir)
        .map_err(|error| format!("failed to create {}: {error}", unpack_dir.display()))?;

    if !archive_path.exists() { download_to_path(url, &archive_path)?; }
    let actual = format!("{:x}", Sha256::digest(fs::read(&archive_path).map_err(|e| e.to_string())?));
    if actual != expected { return Err(format!("native source checksum mismatch: {package}")); }
    unpack_tar_gz(&archive_path, &unpack_dir)?;

    let extracted = unpack_dir.join(extracted_dir_name);
    if !extracted.exists() {
        return Err(format!(
            "expected extracted directory {} after downloading {url}",
            extracted.display()
        ));
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    if destination.exists() {
        fs::remove_dir_all(destination).map_err(|error| {
            format!(
                "failed to remove stale destination {}: {error}",
                destination.display()
            )
        })?;
    }
    copy_source_tree(&extracted, destination).map_err(|error| {
        format!(
            "failed to move {} to {}: {error}",
            extracted.display(),
            destination.display()
        )
    })?;

    fs::remove_file(&archive_path)
        .map_err(|error| format!("failed to remove {}: {error}", archive_path.display()))?;
    fs::remove_dir_all(&unpack_dir)
        .map_err(|error| format!("failed to remove {}: {error}", unpack_dir.display()))?;

    Ok(())
}

fn copy_upstream_notices(versions: &Versions, source_dir: &Path, install_dir: &Path) {
    let licenses_dir = install_dir.join("licenses");
    recreate_dir(&licenses_dir).expect("failed to prepare licenses directory");

    copy_notice(
        &source_dir.join("LICENSE"),
        &licenses_dir.join("libavif-LICENSE"),
    );
    copy_notice(
        &source_dir.join("ext/aom/LICENSE"),
        &licenses_dir.join("libaom-LICENSE"),
    );
    copy_notice(
        &source_dir.join("ext/aom/PATENTS"),
        &licenses_dir.join("libaom-PATENTS"),
    );

    let summary = format!(
        "libavif {}\nsource: {}\nlicense file: libavif-LICENSE\n\nlibaom {}\nsource: {}\nlicense files: libaom-LICENSE, libaom-PATENTS\n",
        versions.libavif,
        libavif_source_url(&versions.libavif),
        versions.libaom,
        libaom_source_url(&versions.libaom),
    );
    fs::write(licenses_dir.join("UPSTREAM-SOURCES.txt"), summary)
        .expect("failed to write upstream license summary");
}

fn copy_notice(source: &Path, destination: &Path) {
    fs::copy(source, destination).unwrap_or_else(|error| {
        panic!(
            "failed to copy upstream notice {} to {}: {error}",
            source.display(),
            destination.display()
        )
    });
}

fn emit_link_directives(install_dir: &Path) {
    let lib_dir = library_dir(install_dir);
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=avif");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "linux" | "android" | "freebsd" | "netbsd" | "openbsd" | "dragonfly" => {
            println!("cargo:rustc-link-lib=m");
            println!("cargo:rustc-link-lib=pthread");
        }
        _ => {}
    }
}

fn fetch_prebuilt(install_dir: &Path, target: &str) -> Result<(), String> {
    let base_url = prebuilt_base_url()?;
    let archive_name = format!("jc-libavif-sys-native-{target}.tar.gz");
    let archive_url = format!("{base_url}/{archive_name}");
    let checksum_url = format!("{archive_url}.sha256");
    let archive_path = install_dir.join(&archive_name);
    let checksum_path = install_dir.join(format!("{archive_name}.sha256"));

    download_to_path(&archive_url, &archive_path)?;
    download_to_path(&checksum_url, &checksum_path)?;
    verify_sha256(&archive_path, &checksum_path)?;
    unpack_tar_gz(&archive_path, install_dir)?;

    fs::remove_file(&archive_path).map_err(|error| {
        format!(
            "failed to remove downloaded archive {}: {error}",
            archive_path.display()
        )
    })?;
    fs::remove_file(&checksum_path).map_err(|error| {
        format!(
            "failed to remove downloaded checksum {}: {error}",
            checksum_path.display()
        )
    })?;

    let _ = library_dir(install_dir);
    Ok(())
}

fn download_to_path(url: &str, destination: &Path) -> Result<(), String> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    if let Some(path) = file_url_path(url) {
        fs::copy(&path, destination).map_err(|error| {
            format!(
                "failed to copy {} to {}: {error}",
                path.display(),
                destination.display()
            )
        })?;
        return Ok(());
    }

    let response = ureq::get(url)
        .call()
        .map_err(|error| format!("failed to download {url}: {error}"))?;
    let mut reader = response.into_reader();
    let mut file = fs::File::create(destination)
        .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;
    io::copy(&mut reader, &mut file)
        .map_err(|error| format!("failed to write {}: {error}", destination.display()))?;
    Ok(())
}

fn file_url_path(url: &str) -> Option<PathBuf> {
    url.strip_prefix("file://").map(PathBuf::from)
}

fn unpack_tar_gz(archive_path: &Path, destination: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path)
        .map_err(|error| format!("failed to open {}: {error}", archive_path.display()))?;
    let decoder = GzDecoder::new(BufReader::new(file));
    let mut archive = Archive::new(decoder);
    archive.unpack(destination).map_err(|error| {
        format!(
            "failed to extract {} into {}: {error}",
            archive_path.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn verify_sha256(archive_path: &Path, checksum_path: &Path) -> Result<(), String> {
    let expected = fs::read_to_string(checksum_path)
        .map_err(|error| format!("failed to read {}: {error}", checksum_path.display()))?
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("checksum file {} is empty", checksum_path.display()))?
        .to_owned();

    let mut file = BufReader::new(
        fs::File::open(archive_path)
            .map_err(|error| format!("failed to open {}: {error}", archive_path.display()))?,
    );
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to read {}: {error}", archive_path.display()))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let actual = hex_lower(&hasher.finalize());

    if expected != actual {
        return Err(format!(
            "sha256 mismatch for {}: expected {expected}, got {actual}",
            archive_path.display()
        ));
    }
    Ok(())
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn prebuilt_base_url() -> Result<String, String> {
    if let Some(value) = env::var_os(ENV_PREBUILT_BASE_URL) {
        return Ok(trim_trailing_slashes(&value.to_string_lossy()).to_owned());
    }

    let repository = env::var("CARGO_PKG_REPOSITORY").map_err(|_| {
        format!(
            "{ENV_PREBUILT_BASE_URL} is not set and Cargo package repository metadata is unavailable"
        )
    })?;
    if repository.trim().is_empty() {
        return Err(format!(
            "{ENV_PREBUILT_BASE_URL} is not set and Cargo package repository metadata is empty"
        ));
    }

    let tag = format!(
        "v{}",
        env::var("CARGO_PKG_VERSION").expect("missing CARGO_PKG_VERSION")
    );
    Ok(format!(
        "{}/releases/download/{}",
        trim_trailing_slashes(&repository),
        tag
    ))
}

fn trim_trailing_slashes(input: &str) -> &str {
    input.trim_end_matches('/')
}

fn cmake_command() -> Result<OsString, String> {
    if let Some(configured) = env::var_os(ENV_CMAKE) {
        if command_exists(&configured) {
            Ok(configured)
        } else {
            Err(format!(
                "configured {ENV_CMAKE} executable {:?} is not runnable",
                configured
            ))
        }
    } else {
        let default = OsString::from("cmake");
        if command_exists(&default) {
            Ok(default)
        } else {
            Err("cmake executable was not found on PATH".to_owned())
        }
    }
}

fn command_exists<S: AsRef<OsStr>>(program: S) -> bool {
    Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn configure_and_build(source_dir: &Path, build_dir: &Path, install_dir: &Path, cmake: &OsStr) {
    run(Command::new(cmake)
        .arg("-S")
        .arg(source_dir)
        .arg("-B")
        .arg(build_dir)
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install_dir.display()))
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("-DBUILD_SHARED_LIBS=OFF")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .arg("-DAVIF_CODEC_AOM=LOCAL")
        .arg("-DAVIF_CODEC_AOM_DECODE=ON")
        .arg("-DAVIF_CODEC_AOM_ENCODE=OFF")
        .arg("-DAVIF_CODEC_DAV1D=OFF")
        .arg("-DAVIF_CODEC_LIBGAV1=OFF")
        .arg("-DAVIF_CODEC_RAV1E=OFF")
        .arg("-DAVIF_CODEC_SVT=OFF")
        .arg("-DAVIF_CODEC_AVM=OFF")
        .arg("-DAVIF_LIBYUV=OFF")
        .arg("-DAVIF_LIBSHARPYUV=OFF")
        .arg("-DAVIF_LIBXML2=OFF")
        .arg("-DAVIF_ZLIBPNG=OFF")
        .arg("-DAVIF_JPEG=OFF")
        .arg("-DAVIF_BUILD_APPS=OFF")
        .arg("-DAVIF_BUILD_TESTS=OFF")
        .arg("-DAVIF_BUILD_EXAMPLES=OFF")
        .arg("-DAVIF_BUILD_MAN_PAGES=OFF"))
    .unwrap_or_else(|error| panic!("failed to configure libavif with cmake: {error}"));

    run(Command::new(cmake)
        .arg("--build")
        .arg(build_dir)
        .arg("--config")
        .arg("Release")
        .arg("--parallel"))
    .unwrap_or_else(|error| panic!("failed to build libavif with cmake: {error}"));

    run(Command::new(cmake)
        .arg("--install")
        .arg(build_dir)
        .arg("--config")
        .arg("Release"))
    .unwrap_or_else(|error| panic!("failed to install libavif with cmake: {error}"));
}

fn build_libaom(source_dir: &Path, build_dir: &Path, cmake: &OsStr) {
    recreate_dir(build_dir).expect("failed to prepare libaom build directory");

    let mut configure = Command::new(cmake);
    configure
        .arg("-S")
        .arg(source_dir)
        .arg("-B")
        .arg(build_dir)
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .arg("-DBUILD_SHARED_LIBS=OFF")
        .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
        .arg("-DCONFIG_AV1_DECODER=1")
        .arg("-DCONFIG_AV1_ENCODER=0")
        .arg("-DCONFIG_PIC=1")
        .arg("-DCONFIG_WEBM_IO=0")
        .arg("-DENABLE_DOCS=0")
        .arg("-DENABLE_EXAMPLES=0")
        .arg("-DENABLE_TESTDATA=0")
        .arg("-DENABLE_TESTS=0")
        .arg("-DENABLE_TOOLS=0");

    if let Some(cpu) = aom_target_cpu_override() {
        configure.arg(format!("-DAOM_TARGET_CPU={cpu}"));
    }

    run(&mut configure)
        .unwrap_or_else(|error| panic!("failed to configure libaom with cmake: {error}"));

    run(Command::new(cmake)
        .arg("--build")
        .arg(build_dir)
        .arg("--config")
        .arg("Release")
        .arg("--parallel"))
    .unwrap_or_else(|error| panic!("failed to build libaom with cmake: {error}"));
}

fn aom_target_cpu_override() -> Option<&'static str> {
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").ok()?;

    if target_arch == "aarch64" {
        return Some("arm64");
    }

    if target_arch == "x86_64" && !command_exists("yasm") && !command_exists("nasm") {
        return Some("generic");
    }

    None
}

fn run(command: &mut Command) -> Result<(), String> {
    let status = command
        .status()
        .map_err(|error| format!("failed to run {:?}: {error}", command))?;
    if !status.success() {
        return Err(format!("command {:?} failed with status {status}", command));
    }
    Ok(())
}

fn emit_rerun_rules(path: &Path) {
    println!("cargo:rerun-if-changed={}", path.display());
}

fn emit_rerun_env(name: &str) {
    println!("cargo:rerun-if-env-changed={name}");
}

fn recreate_dir(path: &Path) -> io::Result<()> {
    check_output_path(path)?;
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    fs::create_dir_all(path)
}

fn library_dir(install_dir: &Path) -> PathBuf {
    for candidate in ["lib64", "lib"] {
        let path = install_dir.join(candidate);
        if path.exists() {
            return path;
        }
    }
    panic!(
        "could not find installed library directory under {}",
        install_dir.display()
    );
}

fn read_versions(path: &Path) -> Versions {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    let mut libavif = None;
    let mut libaom = None;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("invalid versions entry: {line}"));
        let value = value.trim().trim_matches('"').to_owned();
        match key.trim() {
            "libavif" => libavif = Some(value),
            "libaom" => libaom = Some(value),
            other => panic!("unexpected versions key: {other}"),
        }
    }
    Versions {
        libavif: libavif.expect("missing libavif version"),
        libaom: libaom.expect("missing libaom version"),
    }
}

fn libavif_source_url(version: &str) -> String {
    format!("https://github.com/AOMediaCodec/libavif/archive/refs/tags/{version}.tar.gz")
}

fn libaom_source_url(version: &str) -> String {
    format!(
        "https://aom-releases.storage.googleapis.com/libaom-{}.tar.gz",
        version.trim_start_matches('v')
    )
}

fn libavif_source_dir_name(version: &str) -> String {
    format!("libavif-{}", version.trim_start_matches('v'))
}

fn libaom_source_dir_name(version: &str) -> String {
    format!("libaom-{}", version.trim_start_matches('v'))
}

struct Versions {
    libavif: String,
    libaom: String,
}

#[derive(Clone, Copy, Debug)]
enum BuildMode {
    PreferPrebuilt,
    PrebuiltOnly,
    SourceOnly,
}

impl BuildMode {
    fn from_env() -> Self {
        let prebuilt_only = env_flag(ENV_PREBUILT_ONLY);
        let no_prebuilt = env_flag(ENV_NO_PREBUILT);

        if prebuilt_only && no_prebuilt {
            panic!("{ENV_PREBUILT_ONLY} and {ENV_NO_PREBUILT} cannot both be enabled");
        }

        if prebuilt_only {
            Self::PrebuiltOnly
        } else if no_prebuilt {
            Self::SourceOnly
        } else {
            Self::PreferPrebuilt
        }
    }
}

fn env_flag(name: &str) -> bool {
    matches!(
        env::var(name)
            .ok()
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
        Some("1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON")
    )
}

fn check_output_path(path: &Path) -> io::Result<()> {
    let out = fs::canonicalize(env::var_os("OUT_DIR").ok_or_else(|| io::Error::other("missing OUT_DIR"))?)?;
    let resolved = if path.exists() { fs::canonicalize(path)? } else {
        let parent = path.parent().ok_or_else(|| io::Error::other("output path has no parent"))?;
        fs::canonicalize(parent)?.join(path.file_name().ok_or_else(|| io::Error::other("output path has no name"))?)
    };
    if resolved == out || !resolved.starts_with(&out) { return Err(io::Error::other("native build path is outside OUT_DIR")); }
    Ok(())
}

fn copy_source_tree(source: &Path, destination: &Path) -> io::Result<()> {
    check_output_path(source)?;
    if !destination.exists() { fs::create_dir_all(destination)?; }
    check_output_path(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() { copy_source_tree(&entry.path(), &target)?; }
        else { fs::copy(entry.path(), target)?; }
    }
    Ok(())
}
