use metal2vulkan::passes::Stage;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    if let Err(error) = run() {
        eprintln!("FALLBACK: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let input = args.next().ok_or_else(usage)?;
    let spv_path = args.next().ok_or_else(usage)?;
    let reflection_path = args.next().ok_or_else(usage)?;
    let requested_stage = args.next().unwrap_or_else(|| "auto".to_string());
    if args.next().is_some() {
        return Err(usage());
    }

    let scratch = ScratchDir::new("translate-reflected")?;
    let stage = match requested_stage.as_str() {
        "auto" => metal2vulkan::detect_stage(&input, scratch.path())?,
        "kernel" | "compute" => Stage::Kernel,
        "fragment" => Stage::Fragment,
        "vertex" => Stage::Vertex,
        other => return Err(format!("unsupported stage {other:?}; {}", usage())),
    };
    let (spv, reflection) = metal2vulkan::translate_reflected(&input, stage, scratch.path())?;
    let json = serde_json::to_vec_pretty(&reflection)
        .map_err(|error| format!("serialize reflection: {error}"))?;

    std::fs::write(&spv_path, &spv).map_err(|error| format!("write {spv_path}: {error}"))?;
    std::fs::write(&reflection_path, json)
        .map_err(|error| format!("write {reflection_path}: {error}"))?;
    println!(
        "PASS: wrote {} SPIR-V bytes and reflection schema v{}",
        spv.len(),
        reflection.reflection_version
    );
    Ok(())
}

fn usage() -> String {
    "usage: translate_reflected <in.air|in.ll> <out.spv> <out.json> \
     [auto|kernel|fragment|vertex]"
        .to_string()
}

struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(label: &str) -> Result<Self, String> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("clock error: {error}"))?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "metal2vulkan-{label}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path)
            .map_err(|error| format!("create scratch {}: {error}", path.display()))?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
