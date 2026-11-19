//! Import referenced resource declarations without recursively importing their owner's graphs.
use super::{
    constant::{MAX_SBS_BYTES, MAX_XML_NODES, SELF_DEPENDENCY},
    sbs_reader::SbsReader,
    xml::{XmlNode, child_element, child_value},
};
use materialcraft_core::Package;
use std::collections::{BTreeMap, BTreeSet};

fn relative_to(owner: &str, file: &str) -> String {
    let file = file.replace('\\', "/");
    if file.contains(':') || file.starts_with('/') {
        return file;
    }

    let owner = owner.replace('\\', "/");
    owner.rsplit_once('/').map_or_else(|| file.clone(), |(folder, _)| format!("{folder}/{file}"))
}

fn collect_resources<'a>(el: XmlNode<'a>, parts: &mut Vec<&'a str>, out: &mut Vec<(String, XmlNode<'a>)>) {
    for child in el.elements() {
        if child.is("resource") {
            let mut path = parts.join("/");
            let id = child_value(child, "identifier").unwrap_or("");
            if !parts.is_empty() {
                path.push('/');
            }
            path.push_str(id);
            out.push((path, child));
        } else if child.is("group") {
            let id = child_value(child, "identifier");
            if let Some(id) = id {
                parts.push(id);
            }
            collect_resources(child, parts, out);
            if id.is_some() {
                parts.pop();
            }
        } else {
            collect_resources(child, parts, out);
        }
    }
}

pub(super) fn read(reader: &mut SbsReader<'_>, content: XmlNode<'_>, pkg: &mut Package, linked: &dyn Fn(&str) -> Option<Vec<u8>>) {
    const MAX_SOURCE_BYTES: usize = 64 * 1024 * 1024;
    const MAX_SOURCE_PACKAGES: usize = 64;
    const MAX_IMPORTED_RESOURCES: usize = 1024;

    let mut needed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for parameter in content
        .descendants()
        .filter(|node| node.is("parameter") && matches!(child_value(*node, "name"), Some("bitmapresourcepath" | "vectorresourcepath" | "fontdata")))
    {
        let Some(url) = child_element(parameter, "paramValue").and_then(|value| child_value(value, "constantValueString")) else { continue };
        let (path, dependency) = reader.resolve_reference(url);
        if let Some(file) = dependency.filter(|file| file != SELF_DEPENDENCY) {
            needed.entry(file).or_default().insert(path);
        }
    }

    // Dependency resources cannot trigger recursive graph loads or unbounded XML work
    let mut remaining = MAX_SOURCE_BYTES;
    let mut remaining_data = crate::MAX_RESOURCE as usize;
    if needed.len() > MAX_SOURCE_PACKAGES {
        reader.warnings.add("Only the first 64 resource dependencies were read");
    }

    for (file, paths) in needed.into_iter().take(MAX_SOURCE_PACKAGES) {
        let result = (|| -> Result<(), String> {
            let bytes = linked(&file).ok_or("source package is unavailable")?;
            remaining = remaining.checked_sub(bytes.len()).ok_or("linked resource XML exceeds 64 MiB")?;
            let options = craft_xml::Options {
                limits: craft_xml::Limits {
                    max_input: MAX_SBS_BYTES as usize,
                    max_nodes: MAX_XML_NODES as usize,
                    max_depth: 1024,
                    max_elements: usize::MAX,
                    max_attrs: usize::MAX,
                    max_attrs_total: usize::MAX,
                },
                doctype: craft_xml::Doctype::Reject,
                ..craft_xml::Options::default().strict()
            };
            let document = options.parse(&bytes).map_err(|_| "source package is not valid XML")?;
            if document.name != "package" {
                return Err("source is not an SBS package".into());
            }

            let mut resources = Vec::new();
            collect_resources(&document, &mut Vec::new(), &mut resources);
            for (path, resource) in resources {
                if !paths.contains(&path) || reader.external_resource_names.contains_key(&(file.clone(), path.clone())) {
                    continue;
                }

                if reader.external_resource_names.len() >= MAX_IMPORTED_RESOURCES {
                    return Err("linked resources exceed the 1024-resource limit".into());
                }

                let identifier = child_value(resource, "identifier").unwrap_or("resource");
                let name = reader.read_resource(resource, identifier, &path, pkg, &|reference| {
                    linked(&relative_to(&file, reference)).filter(|bytes| bytes.len() <= remaining_data)
                });
                if let Some(imported) = pkg.resources.last_mut() {
                    if !imported.source.is_empty() {
                        imported.source = relative_to(&file, &imported.source);
                    }

                    if let Some(left) = remaining_data.checked_sub(imported.bytes.len()) {
                        remaining_data = left;
                    } else {
                        imported.bytes = materialcraft_core::ResourceBytes::new(Vec::new());
                        reader.warnings.add("Linked resource data exceeds 256 MiB; remaining data is unavailable");
                    }
                }
                reader.external_resource_names.insert((file.clone(), path), name);
            }

            Ok(())
        })();
        if let Err(error) = result {
            reader.warnings.add(format!("Could not load resources from `{file}`: {error}"));
        }
    }
}
