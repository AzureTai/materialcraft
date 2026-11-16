use super::{ProbeCase, Result};
use materialcraft_core::{FunctionGraph, OutputFormat, PatternGraph, PatternNodeKind, Value};

pub(super) fn cases() -> Result<Vec<ProbeCase>> {
    let mut result = Vec::new();
    for size in [[4, 4], [5, 4], [6, 6]] {
        for format in [OutputFormat::U8, OutputFormat::U16, OutputFormat::F16, OutputFormat::F32] {
            for seed in [0, 7, -1] {
                let mut graph = FunctionGraph::default();
                let position = graph.add("get_float2", &[], Some(Value::Text("$pos".into())));
                graph.output = Some(graph.add("swizzle1", &[position], Some(Value::Int(0))));
                let mut case = ProbeCase::function(&format!("pixel_position_{}x{}_{format:?}_seed{seed}", size[0], size[1]), graph, true)?;
                case.graph.size = size;
                case.graph.format = format;
                case.graph.seed = seed;
                result.push(case);
                let mut graph = FunctionGraph::default();
                let one = graph.add_constant(Value::Float(1.0));
                graph.output = Some(graph.add("rand", &[one], None));
                let mut case = ProbeCase::function(&format!("pixel_random_{}x{}_{format:?}_seed{seed}", size[0], size[1]), graph, true)?;
                case.graph.size = size;
                case.graph.format = format;
                case.graph.seed = seed;
                result.push(case);
            }
        }
    }
    for variable in materialcraft_nodes::function::SystemVariable::ALL {
        for pixel in [false, true] {
            let value_type = variable.value_type();
            let graph = FunctionGraph::input(variable.name(), value_type);
            let name = format!("scope_{}_{}", if pixel { "pixel" } else { "value" }, variable.name().trim_start_matches('$'));
            if pixel {
                let mut graph = graph;
                let output = graph.output.ok_or("missing variable output")?;
                let scalar = match value_type {
                    materialcraft_core::ValueType::Float2 | materialcraft_core::ValueType::Float3 => graph.add("swizzle1", &[output], Some(Value::Int(0))),
                    materialcraft_core::ValueType::Integer => graph.add("tofloat", &[output], None),
                    materialcraft_core::ValueType::Integer2 => {
                        let first = graph.add("iswizzle1", &[output], Some(Value::Int(0)));
                        graph.add("tofloat", &[first], None)
                    }
                    _ => output,
                };
                graph.output = Some(scalar);
                result.push(ProbeCase::function(&name, graph, true)?);
            } else {
                result.push(ProbeCase::function(&name, graph, false)?);
            }
        }
    }
    for filter in [0, 1] {
        for tile in [[false, false], [true, true], [true, false], [false, true]] {
            for position in [[0.0, 0.0], [1.0, 1.0], [-0.125, 1.125], [0.125, 0.375]] {
                for gray in [false, true] {
                    let mut graph = FunctionGraph::default();
                    let pos = graph.add_constant(Value::Float2(position));
                    graph.output = Some(graph.add(if gray { "samplelum" } else { "samplecol" }, &[pos], Some(Value::Int2([0, filter]))));
                    let mut case = ProbeCase::function(
                        &format!(
                            "sample_{}_filter{filter}_tile{}{}_pos{}_{}",
                            if gray { "gray" } else { "color" },
                            u8::from(tile[0]),
                            u8::from(tile[1]),
                            position[0],
                            position[1]
                        ),
                        graph.clone(),
                        false,
                    )?;
                    case.graph.tiling = tile;
                    case.image_input(materialcraft_core::NodeId(1), "input0", gray)?;
                    let mut pixel_case = ProbeCase::function(&format!("pixel_{}", case.name), graph, true)?;
                    pixel_case.graph.tiling = tile;
                    pixel_case.graph.node_mut(materialcraft_core::NodeId(1))?.params.insert("color_mode".into(), Value::Bool(!gray));
                    pixel_case.image_input(materialcraft_core::NodeId(1), "input0", gray)?;
                    result.push(case);
                    result.push(pixel_case);
                }
            }
        }
    }
    for kind in [PatternNodeKind::Quadrant, PatternNodeKind::Switch, PatternNodeKind::Iterate, PatternNodeKind::Dot] {
        for size in [[4, 4], [5, 4], [6, 6]] {
            let mut case = ProbeCase::new(&format!("pattern_{}_{}x{}", kind.native_id(), size[0], size[1]));
            case.graph.size = size;
            let source = case.graph.add_node("fx_map", [0.0, 0.0]);
            let mut graph = PatternGraph::with_root(kind);
            if kind != PatternNodeKind::Quadrant {
                let child = graph.add(PatternNodeKind::Quadrant)?;
                if let Some(root) = graph.nodes.first_mut() {
                    root.children.fill(Some(child));
                    if kind == PatternNodeKind::Iterate {
                        root.parameters.insert("numberadded".into(), Value::Int(3));
                    }
                }
            }
            for node in &mut graph.nodes {
                if node.kind == PatternNodeKind::Quadrant {
                    node.parameters.insert("patterntype".into(), Value::Int(2));
                    node.parameters.insert("patternsize".into(), Value::Float2([0.5, 0.75]));
                    node.parameters.insert("opacity".into(), Value::Float4([0.375, 0.375, 0.375, 1.0]));
                }
            }
            case.graph.node_mut(source)?.params.insert("pattern".into(), Value::PatternGraph(Box::new(graph)));
            case.finish(source)?;
            result.push(case);
        }
    }
    Ok(result)
}
