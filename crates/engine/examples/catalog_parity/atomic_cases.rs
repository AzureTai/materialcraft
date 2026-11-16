use super::{ProbeCase, Result};
use materialcraft_core::Value;
use materialcraft_nodes::PortKind;

fn connected(id: &str, suffix: &str, size: [u32; 2], gray: bool, tile: [bool; 2]) -> Result<(ProbeCase, materialcraft_core::NodeId)> {
    let mut case = ProbeCase::new(&format!("atomic_{id}_{suffix}"));
    case.graph.size = size;
    case.graph.tiling = tile;
    let source = case.graph.add_node(id, [0.0, 0.0]);
    if id == "text" {
        case.graph.node_mut(source)?.params.insert("text".into(), Value::Text(String::new()));
    }
    let definition = materialcraft_nodes::find(id).ok_or("missing definition")?;
    for port in &definition.inputs {
        if port.kind != PortKind::Value {
            case.image_input(source, port.id, port.kind == PortKind::Gray || (gray && port.kind != PortKind::Color))?;
        }
    }
    case.finish(source)?;
    Ok((case, source))
}

pub(super) fn cases() -> Result<Vec<ProbeCase>> {
    let mut result = Vec::new();
    for definition in materialcraft_nodes::catalog() {
        if matches!(definition.id, "input_color" | "input_grayscale" | "input_value" | "output" | "value_processor" | "pixel_processor" | "fx_map") {
            continue;
        }
        for size in [[4, 4], [5, 4], [6, 6]] {
            for gray in [false, true] {
                for tile in [[false, false], [true, true], [true, false], [false, true]] {
                    let suffix =
                        format!("connected_{}x{}_{}_tile{}{}", size[0], size[1], if gray { "gray" } else { "color" }, u8::from(tile[0]), u8::from(tile[1]));
                    result.push(connected(definition.id, &suffix, size, gray, tile)?.0);
                }
            }
        }
        for parameter in &definition.params {
            let values: Vec<Value> = match &parameter.default {
                Value::Float(_) => vec![Value::Float(parameter.min), Value::Float(parameter.max)],
                Value::Int(_) => vec![Value::Int(parameter.min as i32), Value::Int(parameter.max as i32)],
                Value::Float2(_) => vec![Value::Float2([parameter.min; 2]), Value::Float2([parameter.max; 2])],
                Value::Float4(_) => vec![Value::Float4([parameter.min; 4]), Value::Float4([parameter.max; 4])],
                Value::Color(_) => vec![Value::Color([0.0; 4]), Value::Color([1.0; 4])],
                Value::Bool(_) => vec![Value::Bool(false), Value::Bool(true)],
                Value::Enum(_) => {
                    // Retain the original selector-9 probe after removing unsupported catalog choices
                    let upper = if definition.id == "channels_shuffle" { 9 } else { parameter.options.len().saturating_sub(1) as u32 };
                    vec![Value::Enum(0), Value::Enum(upper)]
                }
                _ => Vec::new(),
            };
            for (index, value) in values.into_iter().enumerate() {
                let (mut case, source) = connected(definition.id, &format!("param_{}_boundary{index}", parameter.id), [5, 4], false, [true, true])?;
                case.graph.node_mut(source)?.params.insert(parameter.id.into(), value);
                result.push(case);
            }
        }
        if definition.id == "blend" {
            let modes = definition.param("mode").ok_or("missing blending modes")?;
            for index in 0..modes.options.len() {
                let (mut case, source) = connected("blend", &format!("mode{index}"), [5, 4], false, [true, true])?;
                case.graph.node_mut(source)?.params.insert("mode".into(), Value::Enum(index as u32));
                result.push(case);
            }
        }
        if definition.id == "normal" {
            for opengl in [false, true] {
                let (mut case, source) = connected("normal", &format!("vertical_opengl{opengl}"), [4, 4], true, [false, false])?;
                for image in case.inputs.values_mut() {
                    *image = materialcraft_image::Image::gray(16, 16, |_, y| y as f32 / 15.0);
                }
                case.graph.node_mut(source)?.params.insert("opengl".into(), Value::Bool(opengl));
                result.push(case);
            }
        }
    }
    Ok(result)
}
