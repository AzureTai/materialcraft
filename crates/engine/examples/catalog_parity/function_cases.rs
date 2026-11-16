use super::{ProbeCase, Result};
use materialcraft_core::{FunctionGraph, Value};
use materialcraft_nodes::function::{FUNCTIONS, default_value};

fn operation(op: &str, arguments: &[Value], setting: Option<Value>) -> FunctionGraph {
    let mut graph = FunctionGraph::default();
    let sources: Vec<_> = arguments.iter().map(|value| graph.add_constant(value.clone())).collect();
    let id = graph.add(op, &sources, setting);
    graph.output = Some(if op == "passthrough" { graph.add("sequence", &[id, id], None) } else { id });
    graph
}

pub(super) fn cases() -> Result<Vec<ProbeCase>> {
    let mut result = Vec::new();
    for definition in FUNCTIONS {
        let op = definition.id;
        let mut setting = default_value(op);
        let mut arguments = vec![Value::Float(0.375); definition.inputs.len()];
        match op {
            "and" | "or" | "not" => arguments.fill(Value::Bool(true)),
            "atan2" | "samplecol" | "samplelum" => arguments = vec![Value::Float2([0.25, 0.75])],
            "dot" => arguments = vec![Value::Float2([0.25, 0.75]), Value::Float2([0.5, 0.125])],
            "vector3" => arguments = vec![Value::Float2([0.125, 0.25]), Value::Float(0.75)],
            "vector4" => arguments = vec![Value::Float2([0.125, 0.25]), Value::Float2([0.5, 0.75])],
            "ivector2" => arguments = vec![Value::Int(-3), Value::Int(7)],
            "ivector3" => arguments = vec![Value::Int2([-3, 7]), Value::Int(19)],
            "ivector4" => arguments = vec![Value::Int2([-3, 7]), Value::Int2([19, 23])],
            "tofloat" => arguments = vec![Value::Int(-7)],
            "tofloat2" => arguments = vec![Value::Int2([-7, 3])],
            "tofloat3" => arguments = vec![Value::Int3([-7, 3, 19])],
            "tofloat4" => arguments = vec![Value::Int4([-7, 3, 19, 23])],
            "toint1" => arguments = vec![Value::Float(-1.75)],
            "toint2" => arguments = vec![Value::Float2([-1.75, 2.75])],
            "toint3" => arguments = vec![Value::Float3([-1.75, 2.75, 0.0])],
            "toint4" => arguments = vec![Value::Float4([-1.75, 2.75, 0.0, -0.0])],
            "ifelse" => arguments = vec![Value::Bool(false), Value::Float(0.125), Value::Float(0.875)],
            "while" => arguments = vec![Value::Float(0.125), Value::Bool(false), Value::Float(0.875)],
            _ if op.starts_with("iswizzle") => arguments = vec![Value::Int4([-3, 7, 19, 23])],
            _ if op.starts_with("swizzle") => arguments = vec![Value::Float4([0.125, 0.25, 0.5, 0.75])],
            _ => {}
        }
        if op.starts_with("get_") || matches!(op, "set" | "isdefined") {
            setting = Some(Value::Text("audit_variable".into()));
        }
        let mut function = operation(op, &arguments, setting.clone());
        if op.starts_with("get_") || op == "isdefined" {
            let value = match op {
                "get_bool" => Value::Bool(true),
                "get_string" => Value::Text("original probe".into()),
                "get_integer1" => Value::Int(16_777_217),
                "get_integer2" => Value::Int2([-3, 7]),
                "get_integer3" => Value::Int3([-3, 7, 19]),
                "get_integer4" => Value::Int4([-3, 7, 19, 23]),
                "get_float2" => Value::Float2([0.125, 0.25]),
                "get_float3" => Value::Float3([0.125, 0.25, 0.5]),
                "get_float4" => Value::Float4([0.125, 0.25, 0.5, 0.75]),
                _ => Value::Float(0.375),
            };
            let value = function.add_constant(value);
            let set = function.add("set", &[value], setting.clone());
            function.output = Some(function.add("sequence", &[set, 1], None));
        }
        let mut case = ProbeCase::function(&format!("function_{op}_nominal"), function, false)?;
        if matches!(op, "samplecol" | "samplelum") {
            case.image_input(materialcraft_core::NodeId(1), "input0", op == "samplelum")?;
        }
        result.push(case);

        let mut function = operation(op, &[], setting);
        if let Some(node) = function.nodes.first_mut() {
            node.inputs.resize(definition.inputs.len(), None);
        }
        result.push(ProbeCase::function(&format!("function_{op}_unconnected"), function, false)?);
    }
    for op in ["add", "sub", "mul", "div", "mod", "min", "max", "pow", "dot"] {
        for (index, (a, b)) in [(-2.75, 2.0), (0.0, 0.0), (1.0, 0.0), (f32::MAX, 2.0), (1e-30, 1e-20)].into_iter().enumerate() {
            result.push(ProbeCase::function(&format!("function_{op}_boundary{index}"), operation(op, &[Value::Float(a), Value::Float(b)], None), false)?);
        }
        for (name, a, b) in [
            ("float2", Value::Float2([-0.25, 2.5]), Value::Float2([0.5, -0.75])),
            ("float3", Value::Float3([-0.25, 2.5, 4.0]), Value::Float3([0.5, -0.75, 8.0])),
            ("float4", Value::Float4([-0.25, 2.5, 4.0, 0.0]), Value::Float4([0.5, -0.75, 8.0, 1.0])),
            ("integer", Value::Int(-7), Value::Int(3)),
            ("integer2", Value::Int2([-7, 8]), Value::Int2([3, -3])),
            ("integer3", Value::Int3([-7, 8, 16_777_217]), Value::Int3([3, -3, 1])),
            ("integer4", Value::Int4([-7, 8, 16_777_217, 0]), Value::Int4([3, -3, 1, 1])),
        ] {
            result.push(ProbeCase::function(&format!("function_{op}_{name}"), operation(op, &[a, b], None), false)?);
        }
    }
    for op in ["neg", "abs", "floor", "ceil", "sqrt", "log", "log2", "exp", "pow2", "sin", "cos", "tan"] {
        for (index, a) in [-1.5, -0.0, 0.0, 0.5, 1.0, 2.0, f32::MAX, f32::MIN_POSITIVE].into_iter().enumerate() {
            result.push(ProbeCase::function(&format!("function_{op}_boundary{index}"), operation(op, &[Value::Float(a)], None), false)?);
        }
        result.push(ProbeCase::function(&format!("function_{op}_float4"), operation(op, &[Value::Float4([-1.5, 0.0, 0.5, 2.0])], None), false)?);
    }
    for op in ["add", "sub", "mul", "div", "mod", "abs", "neg"] {
        for (index, a, b) in [(0, i32::MAX, 1), (1, i32::MIN, -1), (2, 1, 0)] {
            let args = if matches!(op, "abs" | "neg") { vec![Value::Int(a)] } else { vec![Value::Int(a), Value::Int(b)] };
            result.push(ProbeCase::function(&format!("function_{op}_integer_boundary{index}"), operation(op, &args, None), false)?);
        }
    }
    for op in ["eq", "noteq", "gt", "gteq", "lr", "lreq"] {
        for (index, a, b) in [
            (0, Value::Int(16_777_217), Value::Int(16_777_216)),
            (1, Value::Bool(true), Value::Bool(false)),
            (2, Value::Text("original probe".into()), Value::Text("original probe".into())),
            (3, Value::Float2([1.0, 2.0]), Value::Float2([1.0, 3.0])),
            (4, Value::Int(1), Value::Float(1.0)),
        ] {
            result.push(ProbeCase::function(&format!("function_{op}_types{index}"), operation(op, &[a, b], None), false)?);
        }
    }
    for op in ["and", "or"] {
        for (index, a, b) in [(0, false, false), (1, false, true), (2, true, false)] {
            result.push(ProbeCase::function(&format!("function_{op}_bool{index}"), operation(op, &[Value::Bool(a), Value::Bool(b)], None), false)?);
        }
    }
    for op in ["toint1", "tofloat"] {
        for (index, value) in [Value::Float(0.0), Value::Float(-0.5), Value::Float(2147483648.0), Value::Int(i32::MIN), Value::Int(i32::MAX), Value::Bool(true)]
            .into_iter()
            .enumerate()
        {
            result.push(ProbeCase::function(&format!("function_{op}_conversion{index}"), operation(op, &[value], None), false)?);
        }
    }
    Ok(result)
}
