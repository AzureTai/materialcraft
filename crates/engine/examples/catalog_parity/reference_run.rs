use super::{ProbeCase, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const REFERENCE_TIMEOUT: Duration = Duration::from_secs(30);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(20);

// Measured from an empty package saved by the reference app
pub(super) const REFERENCE_FORMAT_VERSION: &str = "1.1.0.202502";

pub(super) fn run(command: &mut Command, directory: &Path, stage: &str) -> Result<(bool, Vec<u8>)> {
    let stdout = directory.join(format!("{stage}.stdout.txt"));
    let stderr = directory.join(format!("{stage}.stderr.txt"));
    let mut child = command.stdout(Stdio::from(File::create(&stdout)?)).stderr(Stdio::from(File::create(stderr)?)).spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            std::fs::write(
                directory.join(format!("{stage}.exit.json")),
                serde_json::to_vec(&serde_json::json!({"success": status.success(), "code": status.code()}))?,
            )?;
            return Ok((status.success(), std::fs::read(stdout)?));
        }
        if start.elapsed() > REFERENCE_TIMEOUT {
            child.kill()?;
            child.wait()?;
            return Err(format!("reference {stage} exceeded {} seconds", REFERENCE_TIMEOUT.as_secs()).into());
        }
        std::thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

pub(super) struct ReferenceRun {
    pub status: String,
    pub stdout: Vec<u8>,
    pub cached: bool,
    pub engine: String,
}

pub(super) fn measure(case: &ProbeCase, directory: &Path, tools: &Path, bytes: Vec<u8>) -> Result<ReferenceRun> {
    let engine = std::env::var("MC_PARITY_REFERENCE_ENGINE").unwrap_or_else(|_| "sse2".into());
    let mut digest = Sha256::new();
    digest.update(b"original-probe-v2-rgba");
    digest.update(&bytes);
    digest.update(engine.as_bytes());
    for (name, image) in &case.inputs {
        digest.update(name.as_bytes());
        digest.update(image.w.to_le_bytes());
        digest.update(image.h.to_le_bytes());
        for value in image.data.iter() {
            digest.update(value.to_bits().to_le_bytes());
        }
    }
    let key = format!("{:x}", digest.finalize());
    let key_path = directory.join("reference-key.txt");
    let status_path = directory.join("reference-status.txt");
    if std::fs::read_to_string(&key_path).is_ok_and(|previous| previous == key)
        && let Ok(status) = std::fs::read_to_string(&status_path)
    {
        let stdout = std::fs::read(directory.join("render.stdout.txt")).unwrap_or_default();
        return Ok(ReferenceRun { status, stdout, cached: true, engine });
    }

    // Native readers reject the application release in the SBS version header
    // Keep the original; only the transport copy uses the measured empty-package header
    let xml = String::from_utf8(bytes)?
        .replace(&format!("<formatVersion v=\"{}\"/>", env!("CARGO_PKG_VERSION")), &format!("<formatVersion v=\"{REFERENCE_FORMAT_VERSION}\"/>"))
        .replace(&format!("<updaterVersion v=\"{}\"/>", env!("CARGO_PKG_VERSION")), &format!("<updaterVersion v=\"{REFERENCE_FORMAT_VERSION}\"/>"));
    let source = directory.join("reference.sbs");
    std::fs::write(&source, xml)?;
    let mut cooker = Command::new(tools.join("sbscooker.exe"));
    cooker.arg("--inputs").arg(&source).arg("--output-path").arg(directory).args([
        "--output-name",
        "probe",
        "--expose-output-size",
        "1",
        "--expose-random-seed",
        "1",
    ]);
    let (success, _) = run(&mut cooker, directory, "cook")?;
    let (status, stdout) = if success {
        let output = directory.join("result.exr");
        if output.exists() {
            std::fs::remove_file(output)?;
        }

        let mut render = Command::new(tools.join("sbsrender.exe"));
        render
            .arg("render")
            .arg("--inputs")
            .arg(directory.join("probe.sbsar"))
            .arg("--output-path")
            .arg(directory)
            .args(["--output-name", "result", "--output-format", "exr", "--output-bit-depth", "32f", "--exr-format-use-float", "--engine"])
            .arg(&engine)
            .arg("--set-value")
            .arg(format!("$outputsize@{},{}", case.graph.size[0], case.graph.size[1]));
        for (name, image) in &case.inputs {
            let path = directory.join(format!("{name}.exr"));

            // Independent RGBA writer: the app's RGB-only exporter would drop alpha
            exr::prelude::write_rgba_file(&path, image.w as usize, image.h as usize, |x, y| {
                let [r, g, b, a] = image.get(x as i64, y as i64, false);
                (r, g, b, a)
            })?;
            render.arg("--set-entry").arg(format!("{name}@{}", path.display()));
        }
        let (success, stdout) = run(&mut render, directory, "render")?;
        (if success { "measured" } else { "reference_render_error" }, stdout)
    } else {
        ("reference_cook_error", Vec::new())
    };
    std::fs::write(key_path, key)?;
    std::fs::write(status_path, status)?;
    Ok(ReferenceRun { status: status.into(), stdout, cached: false, engine })
}
