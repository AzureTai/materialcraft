use materialcraft_image::{Image, MAX_SIDE};
use rayon::prelude::*;
use std::sync::Arc;

pub(super) fn axis(input: &Image, radius: f32, horizontal: bool, tile: bool) -> Image {
    if !radius.is_finite() || radius <= 0.5 {
        return input.clone();
    }

    // Bound work for hostile strengths to the largest supported image dimension
    let radius = radius.min(MAX_SIDE as f32);
    let extent = (radius + 0.5).floor() as i64;
    let weight = 1.0 / (2.0 * radius);
    let edge_weight = (0.5 - (extent as f32 - 0.5) * weight).max(0.0);
    let (length, lines) = if horizontal { (input.w, input.h) } else { (input.h, input.w) };
    let channels = input.channels();
    let results: Vec<Vec<f32>> = (0..lines)
        .into_par_iter()
        .map(|line| {
            let get = |position: i64| {
                if horizontal { input.get(position, i64::from(line), tile) } else { input.get(i64::from(line), position, tile) }
            };
            let mut output = vec![0.0; length as usize * channels];
            for channel in 0..channels {
                let mut sum = 0.0_f64;
                let mut non_finite = [0_i64; 3];
                for position in (1 - extent)..extent {
                    accumulate(&mut sum, &mut non_finite, get(position)[channel], 1);
                }
                for (position, pixel) in output.chunks_exact_mut(channels).enumerate() {
                    let position = position as i64;
                    let mut value = total(sum, non_finite) * weight;
                    if edge_weight > 0.0 {
                        value += (get(position - extent)[channel] + get(position + extent)[channel]) * edge_weight;
                    }
                    pixel[channel] = value;
                    accumulate(&mut sum, &mut non_finite, get(position + extent)[channel], 1);
                    accumulate(&mut sum, &mut non_finite, get(position - extent + 1)[channel], -1);
                }
            }
            output
        })
        .collect();

    let data = if horizontal {
        results.into_iter().flatten().collect()
    } else {
        let mut data = Vec::with_capacity(input.w as usize * input.h as usize * channels);
        for y in 0..input.h as usize {
            for line in &results {
                data.extend_from_slice(line.get(y * channels..(y + 1) * channels).unwrap_or(&[]));
            }
        }
        data
    };
    Image { w: input.w, h: input.h, kind: input.kind, integer_sampling: false, data: Arc::new(data) }
}

fn accumulate(sum: &mut f64, non_finite: &mut [i64; 3], value: f32, sign: i64) {
    if value.is_finite() {
        *sum += f64::from(value) * sign as f64;
    } else {
        let index = if value.is_nan() {
            0
        } else if value.is_sign_positive() {
            1
        } else {
            2
        };
        non_finite[index] += sign;
    }
}

fn total(sum: f64, non_finite: [i64; 3]) -> f32 {
    // A non-finite sample must stop affecting the running sum once it leaves the window
    if non_finite[0] > 0 || (non_finite[1] > 0 && non_finite[2] > 0) {
        f32::NAN
    } else if non_finite[1] > 0 {
        f32::INFINITY
    } else if non_finite[2] > 0 {
        f32::NEG_INFINITY
    } else {
        sum as f32
    }
}
