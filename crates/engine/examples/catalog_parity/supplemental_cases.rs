use super::{ProbeCase, Result};
use materialcraft_core::{FunctionGraph, Graph, GraphInstance, NodeId, OutputFormat, ParameterInheritance, Value, ValueType};
use std::sync::Arc;

pub(super) fn cases() -> Result<Vec<ProbeCase>> {
    let mut cases = Vec::new();
    for color in [false, true] {
        for (kind, value) in [
            ("float", Value::Float(0.375)),
            ("float2", Value::Float2([0.375, 0.625])),
            ("float3", Value::Float3([0.375, 0.625, 0.875])),
            ("float4", Value::Float4([0.375, 0.625, 0.875, 0.125])),
            ("int", Value::Int(17)),
            ("int2", Value::Int2([17, -3])),
            ("int3", Value::Int3([17, -3, 7])),
            ("int4", Value::Int4([17, -3, 7, 1])),
            ("bool", Value::Bool(true)),
        ] {
            let mut case = ProbeCase::function(&format!("pixel_output_{kind}_color{color}"), FunctionGraph::constant(value), true)?;
            case.graph.node_mut(NodeId(1))?.params.insert("color_mode".into(), Value::Bool(color));
            cases.push(case);
        }
    }
    for format in [OutputFormat::U8, OutputFormat::U16, OutputFormat::F16, OutputFormat::F32] {
        for color in [false, true] {
            for (index, value) in [Value::Float(0.1), Value::Float(-0.25), Value::Float(1.25), Value::Float(65520.0)].into_iter().enumerate() {
                let value = if color {
                    let scalar = value.as_f32().ok_or("missing scalar fixture")?;
                    Value::Float4([scalar, 0.25, 0.75, 0.125])
                } else {
                    value
                };
                let mut case = ProbeCase::function(&format!("precision_{format:?}_color{color}_case{index}"), FunctionGraph::constant(value), true)?;
                case.graph.format = format;
                case.graph.node_mut(NodeId(1))?.params.insert("color_mode".into(), Value::Bool(color));
                cases.push(case);
            }
            let mut function = FunctionGraph::input("$pos", ValueType::Float2);
            let position = function.output.ok_or("missing position")?;
            let x = function.add("swizzle1", &[position], Some(Value::Int(0)));
            function.output = Some(if color {
                let remaining = function.add_constant(Value::Float3([0.25, 0.75, 0.125]));
                function.add("vector4", &[x, remaining], None)
            } else {
                x
            });
            let mut case = ProbeCase::function(&format!("precision_position_256_{format:?}_color{color}"), function, true)?;
            case.graph.size = [8, 8];
            case.graph.format = format;
            case.graph.node_mut(NodeId(1))?.params.insert("color_mode".into(), Value::Bool(color));
            cases.push(case);
        }
    }
    for seed in [0, 1, 7, -1] {
        for vector in [false, true] {
            let mut graph = FunctionGraph::default();
            let range = graph.add_constant(if vector { Value::Float4([1.0; 4]) } else { Value::Float(1.0) });
            graph.output = Some(graph.add("rand", &[range], None));
            let mut case = ProbeCase::function(&format!("value_random_seed{seed}_vector{vector}"), graph, false)?;
            case.graph.seed = seed;
            cases.push(case);
        }
    }
    for tile in [[false, false], [true, true], [true, false], [false, true]] {
        let function = FunctionGraph::input("$tiling", ValueType::Integer);
        let mut case = ProbeCase::function(&format!("inheritance_tiling_{}{}", u8::from(tile[0]), u8::from(tile[1])), function, false)?;
        case.graph.tiling = tile;
        case.graph.tiling_mode = ParameterInheritance::RelativeToInput;
        cases.push(case);
    }
    for pixel in [false, true] {
        for assignment in [false, true] {
            let mut body = FunctionGraph::constant(Value::Float(0.375));
            if assignment {
                body = FunctionGraph::default();
                let current = body.add("get_float1", &[], Some(Value::Text("nested_counter".into())));
                let step = body.add_constant(Value::Float(0.125));
                let next = body.add("add", &[current, step], None);
                body.output = Some(body.add("set", &[next], Some(Value::Text("nested_counter".into()))));
            }
            let mut source = Graph::new("original_callee");
            source.function = Some(body.clone());
            source.function_type = ValueType::Float as u32;
            let (inputs, outputs) = materialcraft_nodes::function::graph_ports(&source)?;
            let mut function = FunctionGraph::default();
            let mut calls = Vec::new();
            for _ in 0..if assignment { 2 } else { 1 } {
                let id = function.add("instance", &[], None);
                let node = function.nodes.last_mut().ok_or("missing call node")?;
                node.instance = Some(GraphInstance {
                    label: String::new(),
                    package: String::new(),
                    graph: source.name.clone(),
                    inputs: inputs.clone(),
                    outputs: outputs.clone(),
                    defaults: Default::default(),
                });
                node.source = Some(Ok(Arc::new(body.clone())));
                calls.push(id);
            }
            let mut output = *calls.first().ok_or("missing call")?;
            if assignment {
                let second = *calls.get(1).ok_or("missing second call")?;
                let both = function.add("sequence", &[output, second], None);
                let read = function.add("get_float1", &[], Some(Value::Text("nested_counter".into())));
                output = function.add("sequence", &[both, read], None);
            }
            function.output = Some(output);
            let mut case = ProbeCase::function(&format!("nested_function_pixel{pixel}_assignment{assignment}"), function, pixel)?;
            case.children.push(source);
            cases.push(case);
        }
    }
    Ok(cases)
}
