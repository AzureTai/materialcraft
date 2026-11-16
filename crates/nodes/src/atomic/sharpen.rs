use super::ATOMIC;
use crate::{CookContext, CookResult, NodeDefinition, node};
use materialcraft_image::{Image, Kind};

pub(super) fn definition() -> NodeDefinition {
    node("sharpen", "Sharpen", ATOMIC)
        .desc("Sharpens local contrast with a four-neighbor Laplacian.")
        .atomic()
        .any_in("input", "Input")
        .float("intensity", "Intensity", 1.0, 0.0, 10.0)
        .any_out()
        .cook(sharpen)
}

fn sharpen(context: &CookContext) -> CookResult {
    let img = context.any_or(0, Kind::Gray, [0.0; 4]);
    let intensity = context.f("intensity");
    let tile = context.tiling_axes();
    let short_side = context.w.min(context.h) as f32;
    let horizontal = intensity * short_side / context.w.max(1) as f32;
    let vertical = intensity * short_side / context.h.max(1) as f32;
    Ok(vec![Image::generate(context.w, context.h, img.kind, |x, y| {
        let (x, y) = (i64::from(x), i64::from(y));
        let center = img.get(x, y, false);
        let left = img.get_axes(x - 1, y, tile);
        let right = img.get_axes(x + 1, y, tile);
        let top = img.get_axes(x, y - 1, tile);
        let bottom = img.get_axes(x, y + 1, tile);
        std::array::from_fn(|channel| {
            // Keep the axial differences separate to preserve Float32 cancellation order
            let twice_center = 2.0 * center[channel];
            let across = twice_center - (left[channel] + right[channel]);
            let down = twice_center - (top[channel] + bottom[channel]);

            // Float outputs retain HDR values but discard negative sharpening lobes
            (center[channel] + (across * horizontal + down * vertical)).max(0.0)
        })
    })])
}
