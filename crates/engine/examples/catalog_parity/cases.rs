use super::{ProbeCase, Result};
use materialcraft_core::{FunctionGraph, Value};

pub(super) fn cases() -> Result<Vec<ProbeCase>> {
    let mut result = Vec::new();
    for definition in materialcraft_nodes::catalog() {
        let mut case = ProbeCase::new(&format!("atomic_{}_unconnected", definition.id));
        let source = case.graph.add_node(definition.id, [0.0, 0.0]);
        // Empty text exercises compositing without depending on an installed font
        if definition.id == "text" {
            case.graph.node_mut(source)?.params.insert("text".into(), Value::Text(String::new()));
        }
        if definition.id == "output" {
            case.output = source;
            case.graph.node_mut(source)?.params.insert("identifier".into(), Value::Text("result".into()));
        } else {
            case.finish(source)?;
        }
        result.push(case);
    }
    let mut case = ProbeCase::new("value_constant");
    let source = case.graph.add_node("value_processor", [0.0, 0.0]);
    case.graph.node_mut(source)?.params.insert("function".into(), Value::Function(Box::new(FunctionGraph::constant(Value::Float(0.375)))));
    case.finish(source)?;
    result.push(case);
    result.extend(super::function_cases::cases()?);
    result.extend(super::processor_cases::cases()?);
    result.extend(super::atomic_cases::cases()?);
    result.extend(super::control_cases::cases()?);
    result.extend(super::pattern_cases::cases()?);
    result.extend(super::supplemental_cases::cases()?);
    result.extend(super::kernel_cases::cases()?);
    Ok(result)
}
