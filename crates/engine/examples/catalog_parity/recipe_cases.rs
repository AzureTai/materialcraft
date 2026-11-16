use super::{ProbeCase, Result};
use materialcraft_core::{OutputFormat, ParameterInheritance, Value};
use materialcraft_image::{Image, Kind};
use serde_json::Value as Json;
use std::{collections::BTreeMap, io::Read, path::Path};

// Original, bounded recipes allow numerical experiments without recompiling the runner
const MAX_CASES: usize = 2048;
const MAX_SIZE_LOG2: u32 = 10;
const MAX_RECIPE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_INPUT_COMPONENTS: u64 = 32 * 1024 * 1024;

pub(super) fn read(path: &Path) -> Result<Vec<ProbeCase>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?.take(MAX_RECIPE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_RECIPE_BYTES {
        return Err("probe recipe file exceeds the byte limit".into());
    }

    let recipes: Vec<Json> = serde_json::from_slice(&bytes)?;
    if recipes.len() > MAX_CASES {
        return Err("too many probe recipes".into());
    }

    // Bound all resident input images before allocating any probe graphs
    let mut components = 0_u64;
    for recipe in &recipes {
        let size: [u32; 2] = field(recipe, "size", [4, 4])?;
        if size.iter().any(|side| *side > MAX_SIZE_LOG2) {
            return Err("recipe exceeds the image size limit".into());
        }

        for input in recipe["inputs"].as_array().into_iter().flatten() {
            let channels = if field(input, "gray", true)? { 1 } else { 4 };
            components = components.saturating_add((1_u64 << (size[0] + size[1])) * channels);
            if components > MAX_INPUT_COMPONENTS {
                return Err("probe inputs exceed the total component limit".into());
            }
        }
    }

    recipes.iter().map(case).collect()
}

fn case(recipe: &Json) -> Result<ProbeCase> {
    let name = recipe["name"].as_str().ok_or("missing recipe name")?;
    if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'.')) || name.contains("..") {
        return Err("recipe name must be a plain file name".into());
    }

    let operation = recipe["operation"].as_str().ok_or("missing operation")?;
    let mut case = ProbeCase::new(name);
    case.graph.size = field(recipe, "size", [4, 4])?;
    if case.graph.size.iter().any(|size| *size > MAX_SIZE_LOG2) {
        return Err("recipe exceeds the image size limit".into());
    }

    case.graph.format = field(recipe, "format", OutputFormat::F32)?;
    case.graph.tiling = field(recipe, "tile", [true; 2])?;
    case.graph.pixel_ratio = field(recipe, "pixel_ratio", 1)?;
    case.graph.pixel_ratio_mode = ParameterInheritance::Absolute;
    let source = case.graph.add_node(operation, [0.0; 2]);
    let params: BTreeMap<String, Value> = field(recipe, "params", BTreeMap::new())?;
    case.graph.node_mut(source)?.params.extend(params);
    if let Some(format) = recipe.get("node_format") {
        let node = case.graph.node_mut(source)?;
        node.base.format = serde_json::from_value(format.clone())?;
        node.base.format_mode = ParameterInheritance::Absolute;
    }
    if let Some(size) = recipe.get("pixel_size") {
        let node = case.graph.node_mut(source)?;
        node.base.pixel_size = serde_json::from_value(size.clone())?;
        node.base.pixel_size_mode = ParameterInheritance::Absolute;
    }
    if let Some(inputs) = recipe["inputs"].as_array() {
        for input in inputs {
            let port = input["port"].as_str().ok_or("missing input port")?;
            let gray = field(input, "gray", true)?;
            let name = format!("image{}", case.inputs.len());
            case.image_input(source, port, gray)?;
            let original = case.inputs.get(&name).ok_or("missing generated input")?;
            let image = input_image(input, original)?;
            case.inputs.insert(name, image);
        }
    }
    if let Some(format) = recipe.get("input_format") {
        for node in &mut case.graph.nodes {
            if matches!(node.def.as_str(), "input_color" | "input_grayscale") {
                node.base.format = serde_json::from_value(format.clone())?;
                node.base.format_mode = ParameterInheritance::Absolute;
            }
        }
    }
    case.finish(source)?;
    Ok(case)
}

fn input_image(input: &Json, original: &Image) -> Result<Image> {
    let (w, h) = (original.w, original.h);
    let pattern = input["pattern"].as_str().unwrap_or("catalog");
    if pattern == "catalog" {
        return Ok(original.clone());
    }
    let background: [f32; 4] = field(input, "background", [0.0; 4])?;
    let value: [f32; 4] = field(input, "value", [1.0; 4])?;
    let position: [u32; 2] = field(input, "position", [w / 2, h / 2])?;
    let pixels: Vec<f32> = field(input, "pixels", Vec::new())?;
    if pattern == "pixels" && pixels.len() != w as usize * h as usize * original.channels() {
        return Err("recipe pixel count does not match its size and kind".into());
    }

    let image = Image::generate(w, h, original.kind, |x, y| match pattern {
        "constant" => value,
        "impulse" => {
            if [x, y] == position {
                value
            } else {
                background
            }
        }
        "grid" => {
            let v = ((x * 13 + y * 7) % 97) as f32 / 256.0;
            [v, 0.5 - v, v * 2.0, 0.25 + v]
        }
        "ramp_x" | "ramp_y" => {
            let v = if pattern == "ramp_x" { x as f32 / (w - 1).max(1) as f32 } else { y as f32 / (h - 1).max(1) as f32 };
            [v; 4]
        }
        "pixels" => {
            let start = (y as usize * w as usize + x as usize) * original.channels();
            if original.kind == Kind::Gray {
                [pixels.get(start).copied().unwrap_or(0.0); 4]
            } else {
                std::array::from_fn(|c| pixels.get(start + c).copied().unwrap_or(0.0))
            }
        }
        _ => background,
    });
    if !matches!(pattern, "constant" | "impulse" | "grid" | "ramp_x" | "ramp_y" | "pixels") {
        return Err(format!("unknown recipe pattern: {pattern}").into());
    }

    Ok(image)
}

fn field<T: serde::de::DeserializeOwned>(value: &Json, key: &str, default: T) -> Result<T> {
    match value.get(key) {
        Some(value) => Ok(serde_json::from_value(value.clone())?),
        None => Ok(default),
    }
}
