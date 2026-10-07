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
    let path = args.next().ok_or_else(|| {
        "usage: translate_native <in.ll> [kernel|fragment|vertex] [out.spv]".to_string()
    })?;
    let stage = match args.next().as_deref() {
        Some("fragment") => Stage::Fragment,
        Some("vertex") => Stage::Vertex,
        Some("kernel") | None => Stage::Kernel,
        Some(other) => return Err(format!("unsupported stage {other:?}")),
    };
    let out = args.next();
    if args.next().is_some() {
        return Err("too many arguments".to_string());
    }

    let san_ll = std::fs::read_to_string(&path).map_err(|error| format!("read {path}: {error}"))?;
    let scratch = ScratchDir::new("translate-native")?;
    let spv = metal2vulkan::translate_sanitized_native(&san_ll, stage, scratch.path())?;
    if let Some(out) = &out {
        std::fs::write(out, &spv).map_err(|error| format!("write {out}: {error}"))?;
    }
    println!("PASS spirv-val ({} bytes)", spv.len());
    Ok(())
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
