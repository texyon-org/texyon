//! Color-aware raster differential metrics for preview fidelity gates.

/// Per-page color-aware raster comparison.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterDiff {
    /// Whether page raster dimensions and component layouts match.
    pub dimensions_match: bool,
    /// Fraction of expected participating pixels matched within tolerance.
    pub agreement: f64,
    /// Expected participating pixels with no nearby color match.
    pub missing_pixels: usize,
    /// Actual participating pixels with no nearby expected color match.
    pub spurious_pixels: usize,
}

/// Compare tightly packed RGB/RGBA rasters with one-pixel spatial tolerance.
pub fn compare_rasters(
    expected: &[u8],
    actual: &[u8],
    width: usize,
    height: usize,
    components: usize,
    max_rgb_distance: u16,
) -> RasterDiff {
    let length = width.saturating_mul(height).saturating_mul(components);
    if components < 3 || expected.len() != length || actual.len() != length {
        return RasterDiff {
            dimensions_match: false,
            agreement: 0.0,
            missing_pixels: width.saturating_mul(height),
            spurious_pixels: width.saturating_mul(height),
        };
    }
    let mut expected_count = 0;
    let mut matched = 0;
    let mut missing = 0;
    for y in 0..height {
        for x in 0..width {
            let color = rgb(expected, width, components, x, y);
            if !participates(color) {
                continue;
            }
            expected_count += 1;
            if neighborhood_matches(
                actual,
                (width, height),
                components,
                (x, y),
                color,
                max_rgb_distance,
            ) {
                matched += 1;
            } else {
                missing += 1;
            }
        }
    }
    let mut spurious = 0;
    for y in 0..height {
        for x in 0..width {
            let color = rgb(actual, width, components, x, y);
            if participates(color)
                && !neighborhood_matches(
                    expected,
                    (width, height),
                    components,
                    (x, y),
                    color,
                    max_rgb_distance,
                )
            {
                spurious += 1;
            }
        }
    }
    RasterDiff {
        dimensions_match: true,
        agreement: if expected_count == 0 {
            f64::from(spurious == 0)
        } else {
            matched as f64 / expected_count as f64
        },
        missing_pixels: missing,
        spurious_pixels: spurious,
    }
}

fn neighborhood_matches(
    samples: &[u8],
    size: (usize, usize),
    components: usize,
    point: (usize, usize),
    color: [u8; 3],
    max_distance: u16,
) -> bool {
    let (width, height) = size;
    let (x, y) = point;
    let threshold = u32::from(max_distance).pow(2);
    let x0 = x.saturating_sub(1);
    let y0 = y.saturating_sub(1);
    for nearby_y in y0..=(y + 1).min(height - 1) {
        for nearby_x in x0..=(x + 1).min(width - 1) {
            let candidate = rgb(samples, width, components, nearby_x, nearby_y);
            let distance = color
                .into_iter()
                .zip(candidate)
                .map(|(left, right)| i32::from(left) - i32::from(right))
                .map(|delta| delta.unsigned_abs().pow(2))
                .sum::<u32>();
            if distance <= threshold {
                return true;
            }
        }
    }
    false
}

fn rgb(samples: &[u8], width: usize, components: usize, x: usize, y: usize) -> [u8; 3] {
    let offset = (y * width + x) * components;
    [samples[offset], samples[offset + 1], samples[offset + 2]]
}

fn participates(color: [u8; 3]) -> bool {
    color != [255, 255, 255]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerates_one_pixel_motion_but_not_wrong_color() {
        let mut expected = vec![255; 3 * 3 * 3];
        expected[12..15].copy_from_slice(&[255, 0, 0]);
        let mut shifted = vec![255; 3 * 3 * 3];
        shifted[15..18].copy_from_slice(&[255, 0, 0]);
        assert_eq!(
            compare_rasters(&expected, &shifted, 3, 3, 3, 8).agreement,
            1.0
        );
        shifted[15..18].copy_from_slice(&[0, 0, 255]);
        let diff = compare_rasters(&expected, &shifted, 3, 3, 3, 8);
        assert_eq!(diff.agreement, 0.0);
        assert_eq!(diff.missing_pixels, 1);
        assert_eq!(diff.spurious_pixels, 1);
    }
}
