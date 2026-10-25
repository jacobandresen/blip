use std::path::{Path, PathBuf};
use std::process::Command;

const GAMES: [&str; 9] = [
    "serpent", "bouncer", "defender", "rally", "meteors", "raider", "brawler", "bubbler", "adder",
];

fn rustup_path(tool: &str) -> Option<PathBuf> {
    let output = Command::new("rustup").args(["which", tool]).output().ok()?;
    if !output.status.success() { return None; }
    Some(PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()))
}

fn run(command: &Path, args: &[String], root: &Path, rustc: Option<&Path>) {
    let mut process = Command::new(command);
    process.args(args).current_dir(root);
    if let Some(rustc) = rustc { process.env("RUSTC", rustc); }
    let status = process.status().unwrap_or_else(|error| panic!("could not run {}: {error}", command.display()));
    assert!(status.success(), "{} failed with {status}", command.display());
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).expect("workspace root");
    let cargo = rustup_path("cargo").unwrap_or_else(|| PathBuf::from("cargo"));
    let rustc = rustup_path("rustc");
    let mut args = vec!["build".into(), "--release".into(), "--target".into(), "wasm32-unknown-unknown".into()];
    for game in GAMES { args.extend(["-p".into(), game.into()]); }
    args.extend(["-p".into(), "blip_basic".into()]);
    println!("[build] cargo {}", args.join(" "));
    run(&cargo, &args, root, rustc.as_deref());

    let target = root.join("target/wasm32-unknown-unknown/release");
    for game in GAMES {
        let output = root.join("web").join(game);
        std::fs::create_dir_all(&output).expect("create game web directory");
        std::fs::copy(target.join(format!("{game}.wasm")), output.join("index.wasm"))
            .unwrap_or_else(|error| panic!("copy {game}.wasm: {error}"));
        std::fs::copy(root.join("web/shell.html"), output.join("index.html"))
            .unwrap_or_else(|error| panic!("copy shell.html: {error}"));
        let bytes = std::fs::metadata(output.join("index.wasm")).expect("wasm metadata").len();
        println!("[ok] {game} -> web/{game}/index.wasm ({bytes} bytes)");
    }
    std::fs::copy(target.join("blip_basic.wasm"), root.join("web/basic-runtime.wasm"))
        .expect("copy blip_basic.wasm");
    println!("[ok] blip_basic -> web/basic-runtime.wasm");
}
