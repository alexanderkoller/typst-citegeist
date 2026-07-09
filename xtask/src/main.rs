use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BINARYEN_VERSION: &str = "version_130";
const PACKAGE: &str = "citegeist";
const WASM_OPT_FLAGS: &[&str] = &["--enable-bulk-memory-opt", "-Oz"];

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("build-wasm") => build_wasm(),
        Some("-h") | Some("--help") | None => {
            print_help();
            Ok(())
        }
        Some(cmd) => Err(format!("unknown command: {cmd}")),
    }
}

fn print_help() {
    println!("Usage:");
    println!("  cargo build-wasm");
    println!();
    println!("Builds plugin/citegeist/plugin for wasm32-unknown-unknown, then optimizes");
    println!("the resulting citegeist.wasm with Binaryen wasm-opt -Oz.");
}

fn build_wasm() -> Result<(), String> {
    let repo = repo_root()?;
    let plugin_dir = repo.join("plugin").join(PACKAGE).join("plugin");
    let wasm = plugin_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{PACKAGE}.wasm"));
    let profiling_wasm = repo.join("profiling").join(format!("{PACKAGE}.wasm"));

    run_cmd(
        Command::new("cargo")
            .arg("build")
            .arg("--target")
            .arg("wasm32-unknown-unknown")
            .arg("--release")
            .current_dir(&plugin_dir),
    )?;

    let wasm_opt = find_or_install_wasm_opt(&repo)?;
    optimize_wasm(&wasm_opt, &wasm)?;

    println!("optimized {}", wasm.display());
    if profiling_wasm.exists() {
        fs::copy(&wasm, &profiling_wasm).map_err(|e| {
            format!(
                "could not update profiling copy {}: {e}",
                profiling_wasm.display()
            )
        })?;
        println!("updated {}", profiling_wasm.display());
    }
    Ok(())
}

fn repo_root() -> Result<PathBuf, String> {
    let mut dir =
        env::current_dir().map_err(|e| format!("could not get current directory: {e}"))?;
    loop {
        if dir.join("typst-template.toml").is_file() && dir.join("plugin").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("could not find repository root".into());
        }
    }
}

fn find_or_install_wasm_opt(repo: &Path) -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("CITEGEIST_WASM_OPT") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "CITEGEIST_WASM_OPT points to a missing file: {}",
            path.display()
        ));
    }

    if let Some(path) = find_on_path("wasm-opt") {
        return Ok(path);
    }

    let local = local_wasm_opt_path(repo)?;
    if local.is_file() {
        return Ok(local);
    }

    if env::var_os("CITEGEIST_NO_BINARYEN_DOWNLOAD").is_some() {
        return Err(binaryen_install_hint());
    }

    install_binaryen(repo)?;

    let local = local_wasm_opt_path(repo)?;
    if local.is_file() {
        Ok(local)
    } else {
        Err(format!(
            "Binaryen download finished, but wasm-opt was not found at {}",
            local.display()
        ))
    }
}

fn optimize_wasm(wasm_opt: &Path, wasm: &Path) -> Result<(), String> {
    let tmp = wasm.with_extension("wasm.tmp");
    let mut command = Command::new(wasm_opt);
    command.args(WASM_OPT_FLAGS).arg(wasm).arg("-o").arg(&tmp);
    run_cmd(&mut command)?;
    fs::rename(&tmp, wasm).map_err(|e| {
        format!(
            "could not replace {} with optimized output {}: {e}",
            wasm.display(),
            tmp.display()
        )
    })?;
    Ok(())
}

fn install_binaryen(repo: &Path) -> Result<(), String> {
    let asset = binaryen_asset_name()?;
    let base_url = format!(
        "https://github.com/WebAssembly/binaryen/releases/download/{BINARYEN_VERSION}/{asset}"
    );

    let cache_dir = repo.join(".tools").join("downloads");
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("could not create {}: {e}", cache_dir.display()))?;

    let archive = cache_dir.join(&asset);
    let checksum = cache_dir.join(format!("{asset}.sha256"));

    println!("wasm-opt not found; downloading Binaryen {BINARYEN_VERSION}...");
    download(&base_url, &archive)?;
    download(&format!("{base_url}.sha256"), &checksum)?;
    verify_sha256(&archive, &checksum)?;

    let install_dir = local_binaryen_dir(repo)?;
    fs::create_dir_all(&install_dir)
        .map_err(|e| format!("could not create {}: {e}", install_dir.display()))?;

    run_cmd(
        Command::new("tar")
            .arg("-xzf")
            .arg(&archive)
            .arg("--strip-components=1")
            .arg("-C")
            .arg(&install_dir),
    )?;

    Ok(())
}

fn download(url: &str, dest: &Path) -> Result<(), String> {
    run_cmd(
        Command::new("curl")
            .arg("-L")
            .arg("--fail")
            .arg("--silent")
            .arg("--show-error")
            .arg("-o")
            .arg(dest)
            .arg(url),
    )
}

fn verify_sha256(archive: &Path, checksum: &Path) -> Result<(), String> {
    let expected_line = fs::read_to_string(checksum)
        .map_err(|e| format!("could not read {}: {e}", checksum.display()))?;
    let expected = expected_line
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("empty checksum file: {}", checksum.display()))?;

    let output = Command::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(archive)
        .output()
        .map_err(|e| format!("could not run shasum: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "shasum failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let actual_stdout = String::from_utf8_lossy(&output.stdout);
    let actual = actual_stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| "shasum produced no output".to_string())?;

    if expected == actual {
        Ok(())
    } else {
        Err(format!(
            "Binaryen checksum mismatch: expected {expected}, got {actual}"
        ))
    }
}

fn binaryen_asset_name() -> Result<String, String> {
    let os = env::consts::OS;
    let arch = env::consts::ARCH;

    let platform = match (os, arch) {
        ("macos", "aarch64") => "arm64-macos",
        ("macos", "x86_64") => "x86_64-macos",
        ("linux", "aarch64") => "aarch64-linux",
        ("linux", "x86_64") => "x86_64-linux",
        ("windows", "aarch64") => "arm64-windows",
        ("windows", "x86_64") => "x86_64-windows",
        _ => {
            return Err(format!(
                "no prebuilt Binaryen archive configured for {arch}-{os}; {}",
                binaryen_install_hint()
            ))
        }
    };

    Ok(format!("binaryen-{BINARYEN_VERSION}-{platform}.tar.gz"))
}

fn local_binaryen_dir(repo: &Path) -> Result<PathBuf, String> {
    let asset = binaryen_asset_name()?;
    let platform = asset
        .strip_prefix(&format!("binaryen-{BINARYEN_VERSION}-"))
        .and_then(|s| s.strip_suffix(".tar.gz"))
        .ok_or_else(|| format!("could not parse Binaryen asset name: {asset}"))?;
    Ok(repo
        .join(".tools")
        .join("binaryen")
        .join(BINARYEN_VERSION)
        .join(platform))
}

fn local_wasm_opt_path(repo: &Path) -> Result<PathBuf, String> {
    let exe = if env::consts::OS == "windows" {
        "wasm-opt.exe"
    } else {
        "wasm-opt"
    };
    Ok(local_binaryen_dir(repo)?.join("bin").join(exe))
}

fn binaryen_install_hint() -> String {
    "install Binaryen with `brew install binaryen`, or unset CITEGEIST_NO_BINARYEN_DOWNLOAD to let `cargo build-wasm` download a prebuilt release".into()
}

fn find_on_path(exe: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for path in env::split_paths(&paths) {
        let candidate = path.join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn run_cmd(command: &mut Command) -> Result<(), String> {
    let display = display_command(command);
    let status = command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("failed to run {display}: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{display} exited with {status}"))
    }
}

fn display_command(command: &Command) -> String {
    let mut parts: Vec<OsString> = Vec::new();
    parts.push(command.get_program().to_os_string());
    parts.extend(command.get_args().map(|arg| arg.to_os_string()));
    parts
        .iter()
        .map(|part| part.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}
