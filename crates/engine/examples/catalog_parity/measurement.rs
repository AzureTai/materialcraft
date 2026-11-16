use super::{ProbeCase, Result};
use materialcraft_eval::{Cache, Options, cook};
use serde_json::{Value, json};
use std::{path::Path, sync::Mutex};

pub(super) fn measure(case: &ProbeCase, root: &Path, tools: Option<&Path>) -> Result<Value> {
    let directory = root.join(&case.name);
    std::fs::create_dir_all(&directory)?;
    let package = case.package();
    let source = directory.join("probe.sbs");
    let bytes = materialcraft_formats::sbs::write_sbs(&package)?;
    std::fs::write(&source, &bytes)?;
    let resource = |key: &str| {
        let (name, linear) = materialcraft_image::split_resource_key(key);
        package.resource(name).and_then(|r| materialcraft_formats::decode(r.bytes.as_slice(), linear).ok())
    };
    let mut options = Options::new(&resource);
    options.inputs.clone_from(&case.inputs);
    let cooked = cook(&package, 0, &options, &Mutex::new(Cache::default()));
    let errors: Vec<_> = cooked.nodes.values().filter_map(|node| node.error.as_ref()).collect();
    let mut report = json!({"case": case.name, "status": "untested", "local_errors": errors,
        "local_value": cooked.value(case.output, 0)});
    let Some(tools) = tools else { return Ok(report) };

    let measured = super::reference_run::measure(case, &directory, tools, bytes)?;
    report["transport_header_version"] = super::reference_run::REFERENCE_FORMAT_VERSION.into();
    report["reference_engine"] = measured.engine.into();
    report["cached_reference"] = measured.cached.into();
    if measured.status != "measured" {
        report["status"] = measured.status.into();
        return Ok(report);
    }

    let path = directory.join("result.exr");
    if path.exists()
        && let Some(local) = cooked.image(case.output, 0)
    {
        let (reference, reference_channels) = read_exr(&path)?;
        report["reference_channels"] = json!(reference_channels);
        report["local_size"] = json!([local.w, local.h]);
        report["reference_size"] = json!([reference.w, reference.h]);
        if (local.w, local.h) != (reference.w, reference.h) {
            report["status"] = "size_mismatch".into();
            return Ok(report);
        }

        if !errors.is_empty() {
            report["status"] = "local_error".into();
            return Ok(report);
        }

        report["kind_match"] = (local.kind == reference.kind).into();
        if local.kind != reference.kind {
            report["status"] = "kind_mismatch".into();
            return Ok(report);
        }

        let channels: Vec<usize> = if local.is_gray() || reference.is_gray() {
            vec![0]
        } else {
            ["R", "G", "B", "A"]
                .iter()
                .enumerate()
                .filter_map(|(index, name)| reference_channels.iter().any(|channel| channel == name).then_some(index))
                .collect()
        };
        if channels.is_empty() {
            return Err("reference EXR has no comparable channels".into());
        }

        let mut mismatches = 0_u64;
        let mut pixel_mismatches = 0_u64;
        let mut max_error = 0.0_f64;
        let mut non_finite_mismatches = 0_u64;
        for y in 0..local.h {
            for x in 0..local.w {
                let a = local.get(i64::from(x), i64::from(y), false);
                let b = reference.get(i64::from(x), i64::from(y), false);
                let mut changed = false;
                for (_, (a, b)) in a.into_iter().zip(b).enumerate().filter(|(index, _)| channels.contains(index)) {
                    if a.to_bits() != b.to_bits() {
                        changed = true;
                        mismatches += 1;
                        if a.is_finite() && b.is_finite() {
                            max_error = max_error.max((f64::from(a) - f64::from(b)).abs());
                        } else {
                            non_finite_mismatches += 1;
                        }
                    }
                }
                pixel_mismatches += u64::from(changed);
            }
        }
        report["channels"] = channels.len().into();
        report["samples"] = (u64::from(local.w) * u64::from(local.h) * channels.len() as u64).into();
        report["mismatches"] = mismatches.into();
        report["pixel_mismatches"] = pixel_mismatches.into();
        report["max_finite_error"] = max_error.into();
        report["max_error"] = if non_finite_mismatches == 0 { max_error.into() } else { Value::Null };
        report["non_finite_mismatches"] = non_finite_mismatches.into();
        report["unmeasured_alpha"] = (!local.is_gray() && !reference_channels.iter().any(|channel| channel == "A")).into();
        report["status"] = if mismatches == 0 && local.kind == reference.kind {
            if !local.is_gray() && channels.len() < 4 { "exact_available_channels" } else { "exact" }
        } else {
            "pixel_mismatch"
        }
        .into();
        report["local_first"] = json!(local.get(0, 0, false));
        report["reference_first"] = json!(reference.get(0, 0, false));
    } else {
        let text = String::from_utf8_lossy(&measured.stdout);
        let start = text.find("[\r\n    {").or_else(|| text.find("[\n    {"));
        let data = start.and_then(|start| serde_json::from_str::<Value>(&text[start..]).ok());
        let output = data.as_ref().and_then(|data| data.get(0)).and_then(|graph| graph.get("outputs")).and_then(|outputs| outputs.get(0));
        if let (Some(local), Some(output)) = (cooked.value(case.output, 0), output) {
            let metrics = super::comparison::typed(local, output);
            if let Some(metrics) = metrics.as_object() {
                for (key, value) in metrics {
                    report[key] = value.clone();
                }
            }
            report["reference_value"] = output.clone();
        } else {
            report["reference_value"] = output.cloned().unwrap_or(Value::Null);
            report["status"] = if errors.is_empty() { "reference_value_unread" } else { "local_error" }.into();
        }
    }
    Ok(report)
}

fn read_exr(path: &Path) -> Result<(materialcraft_image::Image, Vec<String>)> {
    use materialcraft_image::{Image, Kind};
    let image = exr::prelude::read_first_flat_layer_from_file(path)?;
    let layer = image.layer_data;
    let channels: std::collections::BTreeMap<_, Vec<_>> =
        layer.channel_data.list.iter().map(|channel| (channel.name.to_string(), channel.sample_data.values_as_f32().collect())).collect();
    let (w, h) = (u32::try_from(layer.size.0)?, u32::try_from(layer.size.1)?);
    let names: Vec<_> = channels.keys().cloned().collect();
    if channels.len() == 1 {
        let values = channels.values().next().ok_or("empty EXR")?;
        return Ok((Image { w, h, kind: Kind::Gray, integer_sampling: false, data: std::sync::Arc::new(values.clone()) }, names));
    }
    let image = Image::generate(w, h, Kind::Color, |x, y| {
        std::array::from_fn(|channel| {
            let name = ["R", "G", "B", "A"].get(channel).copied().unwrap_or("");
            channels.get(name).and_then(|values| values.get((y as usize) * (w as usize) + (x as usize))).copied().unwrap_or(if channel == 3 {
                1.0
            } else {
                0.0
            })
        })
    });
    Ok((image, names))
}
