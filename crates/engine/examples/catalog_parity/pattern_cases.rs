use super::{ProbeCase, Result};
use materialcraft_core::{FunctionGraph, OutputFormat, PatternGraph, PatternNodeKind, Resource, ResourceBytes, Value};

fn square(graph: &mut PatternGraph, depth: usize) -> Result<()> {
    let node = graph.nodes.get_mut(depth).ok_or("missing pattern")?;
    node.parameters.insert("patterntype".into(), Value::Int(2));
    node.parameters.insert("opacity".into(), Value::Float4([0.375, 0.5, 0.75, 1.0]));
    Ok(())
}

fn pattern_case(name: &str, graph: PatternGraph, color: bool, size: [u32; 2], roughness: f32, seed: i32) -> Result<ProbeCase> {
    let mut case = ProbeCase::new(name);
    case.graph.size = size;
    case.graph.seed = seed;
    let source = case.graph.add_node("fx_map", [0.0, 0.0]);
    let node = case.graph.node_mut(source)?;
    node.params.insert("pattern".into(), Value::PatternGraph(Box::new(graph)));
    node.params.insert("color_mode".into(), Value::Bool(color));
    node.params.insert("roughness".into(), Value::Float(roughness));
    case.finish(source)?;
    Ok(case)
}

pub(super) fn cases() -> Result<Vec<ProbeCase>> {
    let mut cases = Vec::new();
    for shape in 2..=18 {
        for color in [false, true] {
            for size in [[4, 4], [5, 4]] {
                let mut graph = PatternGraph::with_root(PatternNodeKind::Quadrant);
                square(&mut graph, 0)?;
                let node = graph.nodes.first_mut().ok_or("missing quadrant")?;
                node.parameters.insert("patterntype".into(), Value::Int(shape));
                node.parameters.insert("patternsize".into(), Value::Float2([0.625, 0.75]));
                node.parameters.insert("patternrotation".into(), Value::Float(0.125));
                node.parameters.insert("patternsuppl".into(), Value::Float(0.5));
                cases.push(pattern_case(&format!("fx_shape{shape}_color{color}_{}x{}", size[0], size[1]), graph, color, size, 1.0, 0)?);
            }
        }
    }
    for depth in 0..=2 {
        for roughness in [0.0, 0.5, 1.0] {
            for seed in [0, 7] {
                let mut graph = PatternGraph::with_root(PatternNodeKind::Quadrant);
                for index in 0..depth {
                    let child = graph.add(PatternNodeKind::Quadrant)?;
                    graph.nodes.get_mut(index).ok_or("missing ancestor")?.children.fill(Some(child));
                }
                square(&mut graph, depth)?;
                cases.push(pattern_case(&format!("fx_depth{depth}_rough{roughness}_seed{seed}"), graph, false, [5, 4], roughness, seed)?);
            }
        }
    }
    for seed in [0, 1, 7, -1] {
        let mut graph = PatternGraph::with_root(PatternNodeKind::Quadrant);
        square(&mut graph, 0)?;
        let mut random = FunctionGraph::default();
        let one = random.add_constant(Value::Float(1.0));
        random.output = Some(random.add("rand", &[one], None));
        graph.nodes.first_mut().ok_or("missing quadrant")?.parameters.insert("opacity".into(), Value::Function(Box::new(random)));
        cases.push(pattern_case(&format!("fx_random_seed{seed}"), graph, false, [4, 4], 1.0, seed)?);
    }
    for variable in materialcraft_nodes::function::SystemVariable::ALL {
        let mut graph = PatternGraph::with_root(PatternNodeKind::Quadrant);
        square(&mut graph, 0)?;
        let mut function = FunctionGraph::input(variable.name(), variable.value_type());
        let output = function.output.ok_or("missing variable")?;
        function.output = Some(match variable.value_type() {
            materialcraft_core::ValueType::Float2 | materialcraft_core::ValueType::Float3 => function.add("swizzle1", &[output], Some(Value::Int(0))),
            materialcraft_core::ValueType::Integer => function.add("tofloat", &[output], None),
            materialcraft_core::ValueType::Integer2 => {
                let first = function.add("iswizzle1", &[output], Some(Value::Int(0)));
                function.add("tofloat", &[first], None)
            }
            _ => output,
        });
        graph.nodes.first_mut().ok_or("missing quadrant")?.parameters.insert("opacity".into(), Value::Function(Box::new(function)));
        cases.push(pattern_case(&format!("fx_scope_{}", variable.name().trim_start_matches('$')), graph, false, [5, 4], 1.0, 0)?);
    }
    for branch in [false, true] {
        let mut graph = PatternGraph::with_root(PatternNodeKind::Switch);
        let left = graph.add(PatternNodeKind::Quadrant)?;
        let right = graph.add(PatternNodeKind::Quadrant)?;
        graph.nodes.first_mut().ok_or("missing switch")?.children = vec![Some(left), Some(right)];
        graph.nodes.first_mut().ok_or("missing switch")?.parameters.insert("switch".into(), Value::Bool(branch));
        square(&mut graph, 1)?;
        square(&mut graph, 2)?;
        graph.nodes.get_mut(2).ok_or("missing right branch")?.parameters.insert("opacity".into(), Value::Float4([0.75; 4]));
        cases.push(pattern_case(&format!("fx_switch_{branch}"), graph, false, [4, 4], 1.0, 0)?);
    }
    for count in [0, 1, 4] {
        let mut graph = PatternGraph::with_root(PatternNodeKind::Iterate);
        let child = graph.add(PatternNodeKind::Quadrant)?;
        graph.nodes.first_mut().ok_or("missing iterate")?.children = vec![None, Some(child)];
        graph.nodes.first_mut().ok_or("missing iterate")?.parameters.insert("numberadded".into(), Value::Int(count));
        square(&mut graph, 1)?;
        let mut function = FunctionGraph::default();
        let current = function.add("get_float1", &[], Some(Value::Text("audit_counter".into())));
        let step = function.add_constant(Value::Float(0.125));
        let next = function.add("add", &[current, step], None);
        function.output = Some(function.add("set", &[next], Some(Value::Text("audit_counter".into()))));
        graph.nodes.get_mut(1).ok_or("missing leaf")?.parameters.insert("opacity".into(), Value::Function(Box::new(function)));
        cases.push(pattern_case(&format!("fx_iterate_assignments_{count}"), graph, false, [4, 4], 1.0, 0)?);
    }
    let mut graph = PatternGraph::with_root(PatternNodeKind::Quadrant);
    square(&mut graph, 0)?;
    graph.nodes.first_mut().ok_or("missing quadrant")?.parameters.insert("patterntype".into(), Value::Int(1));
    let mut case = pattern_case("fx_nested_pixel_input", graph, false, [5, 4], 1.0, 0)?;
    let mut function = FunctionGraph::default();
    let position = function.add("get_float2", &[], Some(Value::Text("$pos".into())));
    function.output = Some(function.add("swizzle1", &[position], Some(Value::Int(0))));
    let pixel = case.graph.add_node("pixel_processor", [-200.0, 0.0]);
    case.graph.node_mut(pixel)?.params.insert("function".into(), Value::Function(Box::new(function)));
    case.graph.node_mut(pixel)?.params.insert("color_mode".into(), Value::Bool(false));
    case.graph.connect(pixel, "output", materialcraft_core::NodeId(1), "pattern0")?;
    cases.push(case);
    for (id, file, bytes) in [
        ("bitmap", "original.pgm", b"P2\n4 4\n255\n0 85 170 255\n255 170 85 0\n0 255 0 255\n255 0 255 0\n".to_vec()),
        (
            "svg",
            "original.svg",
            b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\"><rect x=\"2\" y=\"3\" width=\"8\" height=\"10\" fill=\"#4080c0\"/></svg>"
                .to_vec(),
        ),
    ] {
        for size in [[4, 4], [5, 4], [6, 6]] {
            let mut case = ProbeCase::new(&format!("resource_{id}_{}x{}", size[0], size[1]));
            case.graph.size = size;
            let source = case.graph.add_node(id, [0.0, 0.0]);
            case.graph.node_mut(source)?.params.insert("resource".into(), Value::Text("original".into()));
            case.resources.push(Resource::new("original", file, ResourceBytes::new(bytes.clone())));
            case.finish(source)?;
            cases.push(case);
        }
    }
    for format in [OutputFormat::U8, OutputFormat::U16, OutputFormat::F16] {
        let mut graph = PatternGraph::with_root(PatternNodeKind::Quadrant);
        square(&mut graph, 0)?;
        let mut case = pattern_case(&format!("fx_precision_{format:?}"), graph, false, [4, 4], 1.0, 0)?;
        case.graph.format = format;
        cases.push(case);
    }
    Ok(cases)
}
