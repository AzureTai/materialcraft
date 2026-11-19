pub(super) type XmlNode<'a> = &'a craft_xml::El;

pub(super) fn parse_floats(text: &str) -> Vec<f32> {
    text.split_whitespace().take(16).map(|t| t.parse::<f32>().ok().filter(|v| v.is_finite()).unwrap_or(0.0)).collect()
}
pub(super) fn parse_ints(text: &str) -> Vec<i32> {
    let parse = |t: &str| t.parse::<i64>().map(|v| v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32).or_else(|_| t.parse::<f64>().map(|v| v as i32));
    text.split_whitespace().take(16).map(|t| parse(t).unwrap_or(0)).collect()
}
pub(super) fn format_float(v: f32) -> String {
    if v.is_finite() { format!("{v}") } else { "0".into() }
}

pub(super) fn child_element<'a>(node: XmlNode<'a>, tag: &str) -> Option<XmlNode<'a>> {
    node.child(tag)
}

pub(super) fn children_named<'a>(node: XmlNode<'a>, tag: &'a str) -> impl Iterator<Item = XmlNode<'a>> + 'a {
    node.children(tag)
}
pub(super) fn child_value<'a>(node: XmlNode<'a>, tag: &str) -> Option<&'a str> {
    node.child_attr(tag, "v")
}
pub(super) fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // SBS historically escapes apostrophes too, although attributes are double quoted.
    // Keep that byte-level policy while sharing XML validity/attribute escaping.
    for (i, part) in text.split('\'').enumerate() {
        if i != 0 {
            out.push_str("&apos;");
        }
        craft_xml::push_esc_attr(&mut out, part);
    }
    out
}
pub(super) fn is_parameters_element(xml: &str) -> bool {
    let options = craft_xml::Options {
        limits: craft_xml::Limits {
            max_input: (16 << 20) - 1,
            max_nodes: 1_000_000,
            max_depth: 1024,
            max_elements: usize::MAX,
            max_attrs: usize::MAX,
            max_attrs_total: usize::MAX,
        },
        doctype: craft_xml::Doctype::Reject,
        ..craft_xml::Options::default().strict()
    };
    options.parse(xml.as_bytes()).is_ok_and(|root| root.name == "parameters")
}

#[cfg(test)]
mod shared_escape_tests {
    use super::xml_escape;

    #[test]
    fn sbs_attribute_wire_spelling_is_preserved() {
        assert_eq!(xml_escape("&<>\"'\t\n\r"), "&amp;&lt;&gt;&quot;&apos;&#9;&#10;&#13;");
        assert_eq!(xml_escape("\0\u{1f}\u{fffe}\u{ffff}é😀"), "é😀");
        assert_eq!(xml_escape("''"), "&apos;&apos;");
        assert_eq!(xml_escape(""), "");
    }

    #[test]
    fn written_attribute_roundtrips_valid_unicode_and_whitespace() {
        let value = "élan 😀 ' \" & < >\t\n\r";
        let xml = format!("<value v=\"{}\"/>", xml_escape(value));
        let doc = roxmltree::Document::parse(&xml).unwrap();
        assert_eq!(doc.root_element().attribute("v"), Some(value));
    }
}
