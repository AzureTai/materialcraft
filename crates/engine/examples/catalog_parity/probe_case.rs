use super::Result;
use materialcraft_core::{Graph, OutputFormat, Package, Value};

pub(super) struct ProbeCase {
    pub(super) name: String,
    pub(super) graph: Graph,
    pub(super) output: materialcraft_core::NodeId,
    pub(super) inputs: std::collections::BTreeMap<String, materialcraft_image::Image>,
    pub(super) children: Vec<Graph>,
    pub(super) resources: Vec<materialcraft_core::Resource>,
}

impl ProbeCase {
    pub(super) fn new(name: &str) -> Self {
        let mut graph = Graph::new("probe");
        graph.size = [4, 4];
        graph.format = OutputFormat::F32;
        graph.tiling_mode = materialcraft_core::ParameterInheritance::Absolute;

        Self { name: name.into(), graph, output: materialcraft_core::NodeId(0), inputs: Default::default(), children: Vec::new(), resources: Vec::new() }
    }

    pub(super) fn finish(&mut self, source: materialcraft_core::NodeId) -> Result<()> {
        self.output = self.graph.add_node("output", [400.0, 0.0]);
        self.graph.node_mut(self.output)?.params.insert("identifier".into(), Value::Text("result".into()));
        self.graph.node_mut(self.output)?.params.insert("usage".into(), Value::Enum(9));
        self.graph.connect(source, "output", self.output, "input")?;

        Ok(())
    }

    pub(super) fn function(name: &str, function: materialcraft_core::FunctionGraph, pixel: bool) -> Result<Self> {
        let mut case = Self::new(name);
        let source = case.graph.add_node(if pixel { "pixel_processor" } else { "value_processor" }, [0.0, 0.0]);
        case.graph.node_mut(source)?.params.insert("function".into(), Value::Function(Box::new(function)));
        if pixel {
            case.graph.node_mut(source)?.params.insert("color_mode".into(), Value::Bool(false));
        }

        case.finish(source)?;

        Ok(case)
    }

    pub(super) fn image_input(&mut self, target: materialcraft_core::NodeId, port: &str, gray: bool) -> Result<()> {
        use materialcraft_image::{Image, Kind};
        let name = format!("image{}", self.inputs.len());
        let input = self.graph.add_node(if gray { "input_grayscale" } else { "input_color" }, [-200.0, self.inputs.len() as f32 * 120.0]);
        self.graph.node_mut(input)?.params.insert("identifier".into(), Value::Text(name.clone()));

        let w = 1_u32.checked_shl(self.graph.size[0]).ok_or("invalid width")?;
        let h = 1_u32.checked_shl(self.graph.size[1]).ok_or("invalid height")?;
        let image = Image::generate(w, h, if gray { Kind::Gray } else { Kind::Color }, |x, y| {
            let u = x as f32 / (w - 1).max(1) as f32;
            let v = y as f32 / (h - 1).max(1) as f32;
            [u, v, if (x / 3 + y / 5) % 2 == 0 { 0.125 } else { 0.875 }, 0.25 + 0.75 * v]
        });
        self.inputs.insert(name, image);
        self.graph.connect(input, "output", target, port)?;

        Ok(())
    }

    pub(super) fn package(&self) -> Package {
        let mut package = Package::new("Original numerical probe");
        package.graphs.push(self.graph.clone());
        package.graphs.extend(self.children.clone());
        package.resources.clone_from(&self.resources);

        package
    }
}
