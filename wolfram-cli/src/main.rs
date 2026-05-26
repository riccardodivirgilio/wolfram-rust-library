use anyhow::{Context, Result};
use cargo_metadata::Message;
use clap::{Parser, Subcommand};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::ffi::CStr;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use wolfram_app_discovery::{SystemID, WolframApp};
use wolfram_expr::{Expr, Symbol};

#[derive(Subcommand)]
enum WlScriptCmd {
    /// Build the crate then run test files through a Wolfram kernel using TestReport
    Test(TestArgs),
    /// Evaluate each file in a Wolfram kernel using Get
    Evaluate(EvaluateArgs),
}

fn dispatch_wl_script(cmd: WlScriptCmd) -> Result<()> {
    match cmd {
        WlScriptCmd::Test(args) => cmd_test(args),
        WlScriptCmd::Evaluate(args) => cmd_evaluate(args),
    }
}

// ── CLI structure ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
enum Cargo {
    Wl(WlArgs),
}

#[derive(Parser)]
#[command(
    name = "wl",
    about = "Build and package Wolfram LibraryLink crates"
)]
struct WlArgs {
    #[command(subcommand)]
    cmd: WlCmd,
}

#[derive(Subcommand)]
enum WlCmd {
    /// Build the crate and generate a WL loader alongside each cdylib
    Build(BuildArgs),
    /// Run a WL script command against the given files
    #[command(flatten)]
    Script(WlScriptCmd),
}

#[derive(Parser)]
struct BuildArgs {
    /// Destination folder for the package (default: <dylib_dir>/<stem>/)
    #[arg(long)]
    out: Option<PathBuf>,

    /// Empty the destination folder before writing
    #[arg(long)]
    cleanup: bool,

    /// Extra arguments forwarded verbatim to `cargo build`
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    cargo_args: Vec<String>,
}

#[derive(Parser)]
struct TestArgs {
    /// Test files (.wlt or .wl); defaults to all *.wl/*.wlt in the current directory
    files: Vec<String>,
}

#[derive(Parser)]
struct EvaluateArgs {
    /// Files to evaluate; defaults to all *.wl in the current directory
    files: Vec<String>,
}

// ── Manifest types ───────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct FunctionEntry {
    name: String,
    kind: String,
    // Native only
    #[serde(default)]
    params: Vec<String>,
    #[serde(default)]
    ret: String,
}

struct ParsedBuildArgs {
    cargo_args: Vec<String>,
    system_ids: Vec<SystemID>,
    out: Option<PathBuf>,
    cleanup: bool,
}

fn rust_target(id: SystemID) -> Result<&'static str> {
    match id {
        SystemID::MacOSX_x86_64 => Ok("x86_64-apple-darwin"),
        SystemID::MacOSX_ARM64 => Ok("aarch64-apple-darwin"),
        SystemID::Windows_x86_64 => Ok("x86_64-pc-windows-gnu"),
        SystemID::Linux_x86_64 => Ok("x86_64-unknown-linux-gnu"),
        SystemID::Linux_ARM64 => Ok("aarch64-unknown-linux-gnu"),
        SystemID::Linux_ARM => Ok("armv7-unknown-linux-gnueabihf"),
        other => anyhow::bail!(
            "SystemID {} is not supported by cargo wl build",
            other.as_str()
        ),
    }
}

// ── Entry point ──────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let Cargo::Wl(args) = Cargo::parse();
    match args.cmd {
        WlCmd::Build(args) => cmd_build(args),
        WlCmd::Script(script_cmd) => dispatch_wl_script(script_cmd),
    }
}

// ── build ────────────────────────────────────────────────────────────────────

fn cmd_build(args: BuildArgs) -> Result<()> {
    let parsed = parse_forwarded_args(args.cargo_args)?;
    let out_dir = parsed.out.as_deref().or(args.out.as_deref());
    let cleanup = args.cleanup || parsed.cleanup;
    let host_system_id = SystemID::try_current_rust_target()
        .map_err(|e| anyhow::anyhow!("unsupported host platform: {e}"))?;
    rust_target(host_system_id)?;
    let system_ids = target_system_ids(host_system_id, parsed.system_ids);

    let host_dylibs = run_cargo_build(&parsed.cargo_args, None)?;

    if host_dylibs.is_empty() {
        eprintln!("cargo wl: no cdylib artifacts found — nothing to generate");
        return Ok(());
    }

    let mut artifacts_by_system_id = Vec::new();
    artifacts_by_system_id.push((host_system_id, host_dylibs.clone()));

    for system_id in system_ids.iter().copied() {
        if system_id == host_system_id {
            continue;
        }

        let dylibs = run_cargo_build(&parsed.cargo_args, Some(rust_target(system_id)?))?;
        artifacts_by_system_id.push((system_id, dylibs));
    }

    let mut cleaned_folders = HashSet::new();
    for host_dylib in &host_dylibs {
        let entries = load_manifest(host_dylib)?;
        let package_folder = package_folder(host_dylib, out_dir);

        if cleanup
            && cleaned_folders.insert(package_folder.clone())
            && package_folder.exists()
        {
            std::fs::remove_dir_all(&package_folder).with_context(|| {
                format!("failed to clear {}", package_folder.display())
            })?;
        }
        std::fs::create_dir_all(&package_folder)
            .with_context(|| format!("failed to create {}", package_folder.display()))?;

        let library_folder_name = library_folder_name(host_dylib)?;
        let host_key = artifact_key(host_dylib);
        for (system_id, dylibs) in &artifacts_by_system_id {
            let Some(dylib) = dylibs.iter().find(|dylib| artifact_key(dylib) == host_key) else {
                anyhow::bail!(
                    "target build for {} did not produce an artifact matching {}",
                    system_id.as_str(),
                    host_dylib.display()
                );
            };
            let library_folder = package_folder
                .join(system_id.as_str())
                .join(&library_folder_name);
            generate_loader(dylib, &entries, &library_folder)?;
        }
    }

    Ok(())
}

fn run_cargo_build(
    cargo_args: &[String],
    rust_target: Option<&str>,
) -> Result<Vec<PathBuf>> {
    let cargo_bin = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let mut cargo = Command::new(cargo_bin);
    cargo
        .arg("build")
        .arg("--message-format=json-render-diagnostics")
        .stdout(Stdio::piped());

    if let Some(rust_target) = rust_target {
        cargo.arg("--target").arg(rust_target);
    }

    cargo.args(cargo_args);

    let mut child = cargo.spawn().context("failed to spawn cargo build")?;
    let stdout = child.stdout.take().unwrap();

    let mut dylibs: Vec<PathBuf> = Vec::new();

    for message in Message::parse_stream(BufReader::new(stdout)) {
        let Message::CompilerArtifact(artifact) =
            message.context("failed to parse cargo build JSON message")?
        else {
            continue;
        };

        let is_cdylib = artifact
            .target
            .crate_types
            .iter()
            .any(|crate_type| crate_type.to_string() == "cdylib");
        if !is_cdylib {
            continue;
        }

        for filename in artifact.filenames {
            let path = filename.as_std_path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "dylib" | "so" | "dll") {
                dylibs.push(path.to_owned());
            }
        }
    }

    let status = child.wait()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(dylibs)
}

// ── loader generation ────────────────────────────────────────────────────────

fn parse_forwarded_args(args: Vec<String>) -> Result<ParsedBuildArgs> {
    let mut cargo_args = Vec::new();
    let mut system_ids = Vec::new();
    let mut out = None;
    let mut cleanup = false;
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        if arg == "--system-id" {
            let value = iter
                .next()
                .context("--system-id requires a Wolfram SystemID value")?;
            system_ids.push(
                value
                    .parse::<SystemID>()
                    .map_err(|()| anyhow::anyhow!("unrecognized Wolfram SystemID: {value:?}"))?,
            );
        } else if let Some(value) = arg.strip_prefix("--system-id=") {
            system_ids.push(
                value
                    .parse::<SystemID>()
                    .map_err(|()| anyhow::anyhow!("unrecognized Wolfram SystemID: {value:?}"))?,
            );
        } else if arg == "--out" {
            let value = iter.next().context("--out requires a destination folder")?;
            out = Some(PathBuf::from(value));
        } else if let Some(value) = arg.strip_prefix("--out=") {
            out = Some(PathBuf::from(value));
        } else if arg == "--cleanup" {
            cleanup = true;
        } else if arg == "--target" || arg.starts_with("--target=") {
            anyhow::bail!(
                "use --system-id <SystemID> instead of forwarding Cargo --target"
            );
        } else {
            cargo_args.push(arg);
        }
    }

    Ok(ParsedBuildArgs {
        cargo_args,
        system_ids,
        out,
        cleanup,
    })
}

fn target_system_ids(
    host_system_id: SystemID,
    requested: Vec<SystemID>,
) -> Vec<SystemID> {
    let mut system_ids = vec![host_system_id];
    for system_id in requested {
        if !system_ids.contains(&system_id) {
            system_ids.push(system_id);
        }
    }
    system_ids
}

fn package_folder(host_dylib: &Path, out_dir: Option<&Path>) -> PathBuf {
    if let Some(dir) = out_dir {
        return dir.to_owned();
    }

    let stem = host_dylib.file_stem().unwrap();
    host_dylib.parent().unwrap_or(Path::new(".")).join(stem)
}

fn library_folder_name(host_dylib: &Path) -> Result<String> {
    Ok(host_dylib
        .file_stem()
        .and_then(|stem| stem.to_str())
        .context("dylib file name is not valid UTF-8")?
        .to_owned())
}

fn artifact_key(dylib: &Path) -> String {
    dylib
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .trim_start_matches("lib")
        .to_owned()
}

fn generate_loader(dylib: &Path, entries: &[FunctionEntry], folder: &Path) -> Result<()> {
    let dylib_bytes = std::fs::read(dylib)
        .with_context(|| format!("failed to read {}", dylib.display()))?;
    let hash = format!("{:x}", Sha256::digest(&dylib_bytes));

    let ext = dylib
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("dylib");
    let hashed_name = format!("{}.{}", hash, ext);

    std::fs::create_dir_all(folder)
        .with_context(|| format!("failed to create {}", folder.display()))?;

    let hashed_dylib = folder.join(&hashed_name);
    std::fs::copy(dylib, &hashed_dylib)
        .with_context(|| format!("failed to copy dylib to {}", hashed_dylib.display()))?;

    let wl = render_wl(&hashed_name, entries);
    let manifest_path = folder.join("manifest.wl");
    std::fs::write(&manifest_path, &wl)
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;

    println!("{}", manifest_path.display());
    Ok(())
}

fn load_manifest(dylib: &Path) -> Result<Vec<FunctionEntry>> {
    type ManifestFn = unsafe extern "C" fn() -> *const std::os::raw::c_char;

    let lib = unsafe { libloading::Library::new(dylib) }
        .with_context(|| format!("failed to dlopen {}", dylib.display()))?;

    let manifest_fn: libloading::Symbol<ManifestFn> =
        unsafe { lib.get(b"__wolfram_manifest_data__\0") }.context(
            "dylib does not export __wolfram_manifest_data__ \
             — was it built with a wolfram-export-* crate?",
        )?;

    let ptr = unsafe { manifest_fn() };
    anyhow::ensure!(!ptr.is_null(), "__wolfram_manifest_data__ returned null");

    let json = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .context("manifest JSON is not valid UTF-8")?;

    serde_json::from_str(json).context("failed to parse manifest JSON")
}

fn render_wl(dylib_name: &str, entries: &[FunctionEntry]) -> String {
    let mut out = String::new();

    out.push_str("(* Auto-generated by cargo wl build — do not edit *)\n\n");
    out.push_str(&format!(
        "With[{{$lib = FileNameJoin[{{DirectoryName[$InputFileName], \"{}\"}}]}},\n",
        dylib_name
    ));
    out.push_str("  <|\n");

    for (i, e) in entries.iter().enumerate() {
        let sep = if i + 1 < entries.len() { "," } else { "" };
        match e.kind.as_str() {
            "Native" => {
                let clean: Vec<String> =
                    e.params.iter().map(|p| p.replace("System`", "")).collect();
                let params = clean.join(", ");
                let ret = e.ret.replace("System`", "");
                out.push_str(&format!(
                    "    \"{}\" -> LibraryFunctionLoad[$lib, \"{}\", {{{}}}, {}]{}\n",
                    e.name, e.name, params, ret, sep
                ));
            },
            "Wstp" => {
                out.push_str(&format!(
                    "    \"{}\" -> With[{{$f = LibraryFunctionLoad[$lib, \"{}\", LinkObject, LinkObject]}}, \
                     Function[Block[{{$Context = \"RustLinkWSTPPrivateContext`\", $ContextPath = {{}}}}, $f[##1]]]]{}\n",
                    e.name, e.name, sep
                ));
            },
            "Wxf" => {
                let load = format!(
                    "LibraryFunctionLoad[$lib, \"{}\", \
                     {{{{ByteArray, \"Constant\"}}}}, \
                     {{ByteArray, Automatic}}]",
                    e.name
                );
                out.push_str(&format!(
                    "    \"{}\" -> Composition[BinaryDeserialize, \
                     {}, BinarySerialize, List]{}\n",
                    e.name, load, sep
                ));
            },
            other => {
                out.push_str(&format!(
                    "    (* unknown export kind {other}: {} *){}\n",
                    e.name, sep
                ));
            },
        }
    }

    out.push_str("  |>\n");
    out.push_str("]\n");

    out
}

// ── WL script commands (test, evaluate, …) ───────────────────────────────────

fn cmd_test(args: TestArgs) -> Result<()> {
    // Always build all examples in debug mode, then discover dylib directories.
    let dylibs = run_cargo_build(&["--examples".to_string()], None)?;
    let lib_dirs: Vec<PathBuf> = {
        let mut seen = HashSet::new();
        dylibs
            .iter()
            .filter_map(|p| p.parent().map(Path::to_owned))
            .filter(|d| seen.insert(d.clone()))
            .collect()
    };

    run_wl_script(include_str!("../commands/test.wl"), args.files, lib_dirs)
}

fn cmd_evaluate(args: EvaluateArgs) -> Result<()> {
    run_wl_script(include_str!("../commands/evaluate.wl"), args.files, vec![])
}

fn run_wl_script(content: &str, files: Vec<String>, lib_dirs: Vec<PathBuf>) -> Result<()> {
    let app = WolframApp::try_default().context("no Wolfram installation found")?;
    let kernel_path = app
        .kernel_executable_path()
        .context("could not locate WolframKernel")?;

    eprintln!("launching {}", kernel_path.display());

    let mut kernel = wstp::kernel::WolframKernelProcess::launch(&kernel_path)
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    let link = kernel.link();

    drain_packets(link)?;

    let fn_expr = Expr::normal(
        Symbol::new("System`ToExpression"),
        vec![Expr::string(content.trim()), Expr::from(Symbol::new("System`InputForm"))],
    );
    let files_list = Expr::normal(
        Symbol::new("System`List"),
        files.iter().map(|f| Expr::string(f.as_str())).collect(),
    );
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let cwd_str = cwd.to_str().context("current directory is not valid UTF-8")?;
    let lib_paths_list = Expr::normal(
        Symbol::new("System`List"),
        lib_dirs
            .iter()
            .map(|p| {
                p.to_str()
                    .map(Expr::string)
                    .with_context(|| format!("lib dir is not valid UTF-8: {}", p.display()))
            })
            .collect::<Result<_>>()?,
    );
    let assoc = Expr::normal(
        Symbol::new("System`Association"),
        vec![
            Expr::normal(Symbol::new("System`Rule"), vec![Expr::string("Files"), files_list]),
            Expr::normal(Symbol::new("System`Rule"), vec![Expr::string("Cwd"), Expr::string(cwd_str)]),
            Expr::normal(Symbol::new("System`Rule"), vec![Expr::string("LibPaths"), lib_paths_list]),
        ],
    );
    let call = Expr::normal(
        Symbol::new("System`ToString"),
        vec![
            Expr::normal(fn_expr, vec![assoc]),
            Expr::from(Symbol::new("System`InputForm")),
        ],
    );

    link.put_eval_packet(&call)
        .map_err(|e| anyhow::anyhow!("failed to send eval packet: {:?}", e))?;

    let result = read_return_packet(link)?;
    match result.kind() {
        wolfram_expr::ExprKind::String(s) => println!("{s}"),
        _ => println!("{result}"),
    }

    Ok(())
}

fn drain_packets(link: &mut wstp::Link) -> Result<()> {
    while link.is_ready() {
        link.raw_next_packet()
            .context("failed to read packet while draining")?;
        link.new_packet()
            .context("failed to advance past packet while draining")?;
    }
    Ok(())
}

fn read_return_packet(link: &mut wstp::Link) -> Result<Expr> {
    loop {
        let pkt = link
            .raw_next_packet()
            .context("failed to read packet from kernel")?;
        match pkt {
            p if p == wstp::sys::RETURNPKT => {
                let result = link.get_expr().context("failed to read return value")?;
                link.new_packet()
                    .context("failed to advance past ReturnPacket")?;
                return Ok(result);
            },
            p if p == wstp::sys::TEXTPKT => {
                // Print[] output
                let text = link.get_expr().context("failed to read TextPacket")?;
                link.new_packet()?;
                if let wolfram_expr::ExprKind::String(s) = text.kind() {
                    print!("{s}");
                }
            },
            p if p == wstp::sys::MESSAGEPKT => {
                // The kernel follows every MessagePacket with a TextPacket
                // containing the formatted message text — just drain this one.
                link.new_packet()?;
            },
            _ => {
                link.new_packet().context("failed to skip packet")?;
            },
        }
    }
}
