use super::{ProbeCase, Result};
use materialcraft_core::{OutputFormat, Value};
use materialcraft_image::{Image, Kind};

pub(super) fn cases() -> Result<Vec<ProbeCase>> {
    let mut cases = Vec::new();
    for operation in ["blur", "directional_blur", "sharpen"] {
        for size in [4, 6, 8] {
            for intensity in [0.0, 1.0, 4.0, 10.0] {
                let mut case = ProbeCase::new(&format!("kernel_{operation}_size{size}_intensity{intensity}"));
                case.graph.size = [size, size];
                let source = case.graph.add_node(operation, [0.0; 2]);
                case.graph.node_mut(source)?.params.insert("intensity".into(), Value::Float(intensity));
                case.image_input(source, "input", true)?;
                let side = 1 << size;
                for image in case.inputs.values_mut() {
                    *image = Image::gray(side, side, |x, y| f32::from(x == side / 2 && y == side / 2));
                }
                case.finish(source)?;
                cases.push(case);
            }
        }
    }
    for channel in 0..10 {
        let mut case = ProbeCase::new(&format!("kernel_shuffle_gray_channel{channel}"));
        let source = case.graph.add_node("channels_shuffle", [0.0; 2]);
        for name in ["red", "green", "blue", "alpha"] {
            case.graph.node_mut(source)?.params.insert(name.into(), Value::Enum(channel));
        }
        case.image_input(source, "input1", true)?;
        case.image_input(source, "input2", true)?;
        for (index, image) in case.inputs.values_mut().enumerate() {
            *image = Image::filled(16, 16, Kind::Gray, [if index == 0 { 0.25 } else { 0.75 }; 4]);
        }
        case.finish(source)?;
        cases.push(case);
    }
    for operation in ["text", "svg"] {
        for color in [false, true] {
            for gray_input in [false, true] {
                let mut case = ProbeCase::new(&format!("kernel_{operation}_background_color{color}_gray{gray_input}"));
                let source = case.graph.add_node(operation, [0.0; 2]);
                case.graph.node_mut(source)?.params.insert("color_mode".into(), Value::Bool(color));
                if operation == "text" {
                    case.graph.node_mut(source)?.params.insert("text".into(), Value::Text(String::new()));
                }
                case.image_input(source, "background", gray_input)?;
                for image in case.inputs.values_mut() {
                    *image = Image::filled(16, 16, if gray_input { Kind::Gray } else { Kind::Color }, [0.25, 0.5, 0.75, 0.125]);
                }
                case.finish(source)?;
                cases.push(case);
            }
        }
    }
    cases.extend(detail_cases()?);
    cases.extend(boundary_cases()?);
    Ok(cases)
}

fn boundary_cases() -> Result<Vec<ProbeCase>> {
    let mut cases = Vec::new();
    for operation in ["blur", "directional_blur", "sharpen", "channels_shuffle"] {
        for size in [[5, 4], [4, 5]] {
            for format in [OutputFormat::U8, OutputFormat::U16, OutputFormat::F16, OutputFormat::F32] {
                let mut case = ProbeCase::new(&format!("kernel_boundary_{operation}_{}x{}_{format:?}", size[0], size[1]));
                case.graph.size = size;
                case.graph.format = format;
                let source = case.graph.add_node(operation, [0.0; 2]);
                if operation != "channels_shuffle" {
                    case.graph.node_mut(source)?.params.insert("intensity".into(), Value::Float(10.0));
                }
                case.image_input(source, if operation == "channels_shuffle" { "input1" } else { "input" }, false)?;
                let (w, h) = (1 << size[0], 1 << size[1]);
                for image in case.inputs.values_mut() {
                    *image = Image::generate(w, h, Kind::Color, |x, y| if [x, y] == [w / 2, h / 2] { [1.0, 0.0, 2.0, 0.25] } else { [0.0, 0.5, -0.5, 0.75] });
                }
                case.finish(source)?;
                cases.push(case);
            }
        }
        for (label, value) in [("nan", f32::NAN), ("positive_infinity", f32::INFINITY), ("negative_infinity", f32::NEG_INFINITY)] {
            let mut case = ProbeCase::new(&format!("kernel_nonfinite_{operation}_{label}"));
            let source = case.graph.add_node(operation, [0.0; 2]);
            case.image_input(source, if operation == "channels_shuffle" { "input1" } else { "input" }, true)?;
            for image in case.inputs.values_mut() {
                *image = Image::gray(16, 16, |x, y| if x == 8 && y == 8 { value } else { 0.25 });
            }
            case.finish(source)?;
            cases.push(case);
        }
    }
    Ok(cases)
}

fn detail_cases() -> Result<Vec<ProbeCase>> {
    let mut cases = Vec::new();
    for operation in ["blur", "directional_blur", "sharpen"] {
        for size in [[4, 4], [5, 4], [8, 8]] {
            for tile in [false, true] {
                for edge in [false, true] {
                    for color in [false, true] {
                        let mut case = ProbeCase::new(&format!("kernel_detail_{operation}_{}x{}_tile{tile}_edge{edge}_color{color}", size[0], size[1]));
                        case.graph.size = size;
                        case.graph.tiling = [tile; 2];
                        let source = case.graph.add_node(operation, [0.0; 2]);
                        case.graph.node_mut(source)?.params.insert("intensity".into(), Value::Float(1.0));
                        case.image_input(source, "input", !color)?;
                        let (w, h) = (1 << size[0], 1 << size[1]);
                        let center = if edge { [0, 0] } else { [w / 2, h / 2] };
                        for image in case.inputs.values_mut() {
                            *image = Image::generate(w, h, if color { Kind::Color } else { Kind::Gray }, |x, y| {
                                let impulse = if [x, y] == center { 0.0625 } else { 0.0 };
                                [0.5 + impulse, 0.25 - impulse, 2.0 + impulse, 0.5 + impulse]
                            });
                        }
                        case.finish(source)?;
                        cases.push(case);
                    }
                }
            }
        }
    }
    for intensity in [0.25, 0.5, 0.75, 1.5, 2.75, 10.0] {
        for angle in [0.0, 0.125, 0.25, 0.333] {
            let mut case = ProbeCase::new(&format!("kernel_directional_angle{angle}_intensity{intensity}"));
            case.graph.size = [8, 8];
            let source = case.graph.add_node("directional_blur", [0.0; 2]);
            case.graph.node_mut(source)?.params.insert("intensity".into(), Value::Float(intensity));
            case.graph.node_mut(source)?.params.insert("angle".into(), Value::Float(angle));
            case.image_input(source, "input", true)?;
            for image in case.inputs.values_mut() {
                *image = Image::gray(256, 256, |x, y| f32::from(x == 128 && y == 128));
            }
            case.finish(source)?;
            cases.push(case);
        }
    }
    for operation in ["text", "svg"] {
        for color in [false, true] {
            for gray_input in [false, true] {
                let mut case = ProbeCase::new(&format!("kernel_{operation}_fallback_color{color}_gray{gray_input}"));
                let source = case.graph.add_node(operation, [0.0; 2]);
                case.graph.node_mut(source)?.params.insert("color_mode".into(), Value::Bool(color));
                case.graph.node_mut(source)?.params.insert("background".into(), Value::Color([0.125, 0.25, 0.5, 0.75]));
                case.image_input(source, "background", gray_input)?;
                for image in case.inputs.values_mut() {
                    *image = Image::filled(16, 16, if gray_input { Kind::Gray } else { Kind::Color }, [0.25, 0.5, 0.75, 0.0]);
                }
                case.finish(source)?;
                cases.push(case);
            }
        }
    }
    Ok(cases)
}
