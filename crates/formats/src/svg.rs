//! SVG resources, drawn to a linear-light image. Parsing and drawing go through the shared
//! `craft-svg` (`tree` and `tree-render`): its preflight bounds the bytes (SVGZ included), entity
//! expansion, attributes and nesting before usvg sees the document, its image resolver never
//! reads linked files and bounds embedded images, and its draw check refuses a drawing whose
//! layers, filters or patterns would allocate past the budget instead of aborting.

use crate::{FormatError, Result};
use craft_svg::tree::{self, ImageResolver, Limits, draw, usvg};
use materialcraft_image::{Image, Kind, MAX_SIDE, srgb_to_linear};

/// Largest SVG file (unpacked when SVGZ).
const MAX_BYTES: usize = 16 << 20;
/// Nesting usvg may build, references followed. usvg recurses a few stack frames per level
/// (about 15 kB), so 64 levels stay well inside a 2 MB worker stack.
const MAX_TREE_DEPTH: usize = 64;
/// Bytes the RGBA buffers of one drawing may hold at once: the 8192² target plus its layers.
const DRAW_BUDGET: u64 = 1 << 30;

fn limits() -> Limits {
    Limits { bytes: MAX_BYTES, depth: MAX_TREE_DEPTH, tree_depth: MAX_TREE_DEPTH, ..Limits::default() }
}

pub(crate) fn decode(bytes: &[u8]) -> Result<Image> {
    let limits = limits();
    let resolver = ImageResolver::new(limits);
    let options = usvg::Options { image_href_resolver: resolver.resolver(), ..usvg::Options::default() };
    let svg = tree::text(bytes, &limits).map_err(svg_error)?;
    resolver.check_document(&svg).map_err(svg_error)?;
    let tree = tree::parse_text(&svg, &limits, &options).map_err(svg_error)?;
    let size = tree.size().to_int_size();
    let (width, height) = (size.width(), size.height());
    if width > MAX_SIDE || height > MAX_SIDE {
        return Err(FormatError::Image(format!("SVG exceeds {MAX_SIDE} pixels")));
    }

    let pixels = draw::render(&tree, usvg::Transform::identity(), width, height, 0, DRAW_BUDGET)
        .map_err(|refusal| FormatError::Image(format!("SVG not drawn: {}", refusal.message)))?;
    Ok(Image::generate(width, height, Kind::Color, |x, y| {
        let pixel = pixels.pixel(x, y).map(|pixel| pixel.demultiply());
        pixel.map_or([0.0; 4], |pixel| {
            [
                srgb_to_linear(f32::from(pixel.red()) / 255.0),
                srgb_to_linear(f32::from(pixel.green()) / 255.0),
                srgb_to_linear(f32::from(pixel.blue()) / 255.0),
                f32::from(pixel.alpha()) / 255.0,
            ]
        })
    }))
}

fn svg_error(error: tree::Error) -> FormatError {
    FormatError::Image(error.to_string())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::decode;

    fn svg(body: &str) -> Vec<u8> {
        format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="2">{body}</svg>"#).into_bytes()
    }

    #[test]
    fn draws_a_shape_in_linear_light() {
        let image = decode(&svg(r##"<rect width="2" height="2" fill="#ff0000"/>"##)).unwrap();
        assert_eq!((image.w, image.h), (4, 2));
        let left = image.get(0, 0, false);
        assert!((left[0] - 1.0).abs() < 1e-3 && left[1].abs() < 1e-3 && (left[3] - 1.0).abs() < 1e-3, "{left:?}");
        assert_eq!(image.get(3, 1, false)[3], 0.0);
    }

    #[test]
    fn hostile_documents_are_errors_not_crashes() {
        // Entities that expand past the shared bound (a "billion laughs").
        let mut laughs = String::from(r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa">"#);
        for level in 1..10 {
            let prev = char::from(b'a' + level - 1);
            let name = char::from(b'a' + level);
            laughs.push_str(&format!("<!ENTITY {name} \"{}\">", format!("&{prev};").repeat(10)));
        }
        laughs.push_str(r#"]><svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><text>&j;</text></svg>"#);
        assert!(decode(laughs.as_bytes()).is_err());
        // Nesting deeper than the tree depth.
        let deep = format!("{}{}", "<g>".repeat(200), "</g>".repeat(200));
        assert!(decode(&svg(&deep)).is_err());
        // Larger than the image side limit.
        let huge = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="1"/>"#;
        assert!(decode(huge).is_err());
        assert!(decode(b"<svg").is_err());
    }

    #[test]
    fn linked_files_are_not_read() {
        let linked = svg(r#"<image href="file:///etc/hostname" width="4" height="2"/>"#);
        let image = decode(&linked).unwrap();
        assert_eq!(image.get(0, 0, false)[3], 0.0);
    }

    #[test]
    fn svgz_is_unpacked() {
        use std::io::Write;
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&svg(r#"<rect width="4" height="2" fill="blue"/>"#)).unwrap();
        let image = decode(&gz.finish().unwrap()).unwrap();
        assert!(image.get(1, 1, false)[2] > 0.9);
    }
}
