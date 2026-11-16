use super::{ProbeCase, Result};
use materialcraft_core::{FunctionGraph, Value, ValueType};

pub(super) fn cases() -> Result<Vec<ProbeCase>> {
    let mut cases = Vec::new();
    for pixel in [false, true] {
        for branch in [false, true] {
            let mut graph = FunctionGraph::default();
            let a = graph.add_constant(Value::Float(0.25));
            let b = graph.add_constant(Value::Float(0.75));
            let set_a = graph.add("set", &[a], Some(Value::Text("counter".into())));
            let set_b = graph.add("set", &[b], Some(Value::Text("counter".into())));
            let condition = graph.add_constant(Value::Bool(branch));
            let choose = graph.add("ifelse", &[condition, set_a, set_b], None);
            let read = graph.add("get_float1", &[], Some(Value::Text("counter".into())));
            graph.output = Some(graph.add("sequence", &[choose, read], None));
            cases.push(ProbeCase::function(&format!("control_lazy_branch_pixel{pixel}_{branch}"), graph, pixel)?);
        }
        let mut graph = FunctionGraph::default();
        let get = graph.add("get_float1", &[], Some(Value::Text("counter".into())));
        let a = graph.add_constant(Value::Float(0.75));
        let set = graph.add("set", &[a], Some(Value::Text("counter".into())));
        let after = graph.add("sequence", &[set, get], None);
        graph.output = Some(graph.add("sequence", &[get, after], None));
        cases.push(ProbeCase::function(&format!("control_shared_get_pixel{pixel}"), graph, pixel)?);
        let mut graph = FunctionGraph::default();
        let get = graph.add("get_float1", &[], Some(Value::Text("counter".into())));
        let snapshot = graph.add("set", &[get], Some(Value::Text("snapshot".into())));
        let value = graph.add_constant(Value::Float(0.75));
        let set = graph.add("set", &[value], Some(Value::Text("counter".into())));
        let read_snapshot = graph.add("get_float1", &[], Some(Value::Text("snapshot".into())));
        let sum = graph.add("add", &[read_snapshot, get], None);
        let after_set = graph.add("sequence", &[set, sum], None);
        graph.output = Some(graph.add("sequence", &[snapshot, after_set], None));
        cases.push(ProbeCase::function(&format!("control_observable_snapshot_pixel{pixel}"), graph, pixel)?);
        for condition in [false, true] {
            let mut graph = FunctionGraph::default();
            let initial = graph.add_constant(Value::Float(0.25));
            let condition_node = graph.add_constant(Value::Bool(condition));
            let body = graph.add_constant(Value::Float(0.75));
            graph.output = Some(graph.add("while", &[initial, condition_node, body], Some(Value::Int(3))));
            cases.push(ProbeCase::function(&format!("control_constant_loop_pixel{pixel}_condition{condition}"), graph, pixel)?);
        }
        for limit in [0, 1, 3, -1] {
            let mut graph = FunctionGraph::default();
            let zero = graph.add_constant(Value::Float(0.0));
            let init = graph.add("set", &[zero], Some(Value::Text("counter".into())));
            let read_cond = graph.add("get_float1", &[], Some(Value::Text("counter".into())));
            let three = graph.add_constant(Value::Float(3.0));
            let condition = graph.add("gteq", &[read_cond, three], None);
            let read_body = graph.add("get_float1", &[], Some(Value::Text("counter".into())));
            let step = graph.add_constant(Value::Float(1.0));
            let next = graph.add("add", &[read_body, step], None);
            let body = graph.add("set", &[next], Some(Value::Text("counter".into())));
            graph.output = Some(graph.add("while", &[init, condition, body], Some(Value::Int(limit))));
            cases.push(ProbeCase::function(&format!("control_loop_pixel{pixel}_limit{limit}"), graph, pixel)?);
        }
        for (index, (op, a, b)) in [("div", 0.0, 0.0), ("div", 1.0, 0.0), ("sqrt", -1.0, 0.0)].into_iter().enumerate() {
            let mut graph = FunctionGraph::default();
            let a = graph.add_constant(Value::Float(a));
            let b = graph.add_constant(Value::Float(b));
            graph.output = Some(if op == "sqrt" { graph.add(op, &[a], None) } else { graph.add(op, &[a, b], None) });
            cases.push(ProbeCase::function(&format!("nonfinite_{op}_pixel{pixel}_case{index}"), graph, pixel)?);
        }
    }
    for op in ["const_string", "get_string", "set"] {
        let mut graph = FunctionGraph::constant(Value::Text("original probe".into()));
        let text = graph.output.ok_or("missing string")?;
        let value = if op == "const_string" {
            text
        } else {
            let set = graph.add("set", &[text], Some(Value::Text("local_text".into())));
            if op == "set" {
                set
            } else {
                let get = graph.add("get_string", &[], Some(Value::Text("local_text".into())));
                graph.add("sequence", &[set, get], None)
            }
        };
        graph.output = Some(graph.add("eq", &[value, text], None));
        cases.push(ProbeCase::function(&format!("string_{op}_equality"), graph, false)?);
    }
    for operation in ["vector2", "vector3", "vector4", "ivector2", "ivector3", "ivector4"] {
        for missing in [0, 1] {
            let mut graph = FunctionGraph::default();
            let constant = graph.add_constant(if operation.starts_with('i') { Value::Int(7) } else { Value::Float(0.75) });
            let output = graph.add(operation, &[], None);
            if let Some(node) = graph.nodes.last_mut() {
                node.inputs = if missing == 0 { vec![None, Some(constant)] } else { vec![Some(constant), None] };
            }
            graph.output = Some(output);
            cases.push(ProbeCase::function(&format!("partial_{operation}_missing{missing}"), graph, false)?);
        }
    }
    let mut graph = FunctionGraph::default();
    let read = graph.add("get_float1", &[], Some(Value::Text("signal".into())));
    let two = graph.add_constant(Value::Float(2.0));
    graph.output = Some(graph.add("mul", &[read, two], None));
    let mut child = ProbeCase::function("nested_value_to_pixel", graph, true)?;
    let pixel = materialcraft_core::NodeId(1);
    child.graph.node_mut(pixel)?.input_values.insert("signal".into(), ValueType::Float);
    let value = child.graph.add_node("value_processor", [-200.0, 0.0]);
    child.graph.node_mut(value)?.params.insert("function".into(), Value::Function(Box::new(FunctionGraph::constant(Value::Float(0.375)))));
    child.graph.connect(value, "output", pixel, "signal")?;
    cases.push(child);
    let mut outer = ProbeCase::new("nested_material_processors");
    let mut child = cases.last().ok_or("missing processor graph")?.graph.clone();
    child.name = "child_processors".into();
    let instance = outer.graph.add_node("graph_instance", [0.0, 0.0]);
    outer.graph.node_mut(instance)?.params.insert("graph".into(), Value::Text(child.name.clone()));
    outer.output = outer.graph.add_node("output", [200.0, 0.0]);
    outer.graph.node_mut(outer.output)?.params.insert("identifier".into(), Value::Text("result".into()));
    outer.graph.connect(instance, "result", outer.output, "input")?;
    outer.children.push(child);
    cases.push(outer);
    Ok(cases)
}
