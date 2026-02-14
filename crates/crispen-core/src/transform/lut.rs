//! 3D LUT baking, application, and `.cube` file I/O.
//!
//! The 3D LUT maps input RGB values to graded output RGB values using
//! trilinear interpolation. Typical sizes are 33³ or 65³ entries.
//!
//! # `.cube` File Format (Iridas/Resolve)
//! ```text
//! TITLE "My LUT"
//! DOMAIN_MIN 0.0 0.0 0.0
//! DOMAIN_MAX 1.0 1.0 1.0
//! LUT_3D_SIZE 33
//! 0.000000 0.000000 0.000000
//! 0.031250 0.000000 0.000000
//! ...
//! ```

use std::io::{BufRead, Write as IoWrite};
use std::path::Path;

use crate::transform::evaluate::evaluate_transform;
use crate::transform::params::GradingParams;

/// A 3D lookup table for fast color transform application.
///
/// The LUT maps input RGB values to graded output RGB values using
/// trilinear interpolation. Typical sizes are 33³ or 65³ entries.
#[derive(Debug, Clone)]
pub struct Lut3D {
    /// Grid size per axis (typically 33 or 65).
    pub size: u32,
    /// LUT entries as RGBA values. Length = size³.
    pub data: Vec<[f32; 4]>,
    /// Minimum domain values per channel.
    pub domain_min: [f32; 3],
    /// Maximum domain values per channel.
    pub domain_max: [f32; 3],
}

impl Lut3D {
    /// Create a new LUT with the given size, initialized to zero.
    pub fn new(size: u32) -> Self {
        let total = (size as usize).pow(3);
        Self {
            size,
            data: vec![[0.0, 0.0, 0.0, 1.0]; total],
            domain_min: [0.0, 0.0, 0.0],
            domain_max: [1.0, 1.0, 1.0],
        }
    }

    /// Bake the full grading transform into this 3D LUT.
    ///
    /// Iterates over the size³ grid, evaluating the complete transform chain
    /// at each grid point using [`evaluate_transform`].
    pub fn bake(&mut self, params: &GradingParams) {
        let size = self.size;
        let size_f = (size - 1) as f32;

        for bi in 0..size {
            for gi in 0..size {
                for ri in 0..size {
                    let r = self.domain_min[0]
                        + (ri as f32 / size_f) * (self.domain_max[0] - self.domain_min[0]);
                    let g = self.domain_min[1]
                        + (gi as f32 / size_f) * (self.domain_max[1] - self.domain_min[1]);
                    let b = self.domain_min[2]
                        + (bi as f32 / size_f) * (self.domain_max[2] - self.domain_min[2]);

                    let result = evaluate_transform([r, g, b], params);
                    let idx = (bi * size * size + gi * size + ri) as usize;
                    self.data[idx] = [result[0], result[1], result[2], 1.0];
                }
            }
        }
    }

    /// Apply this LUT to an RGB pixel using trilinear interpolation.
    ///
    /// ```text
    /// Grid cube containing the lookup point:
    ///
    ///        b=1
    ///         C──────G
    ///        /│     /│
    ///       D──────H │
    ///       │ B────│─F
    ///       │/     │/
    ///       A──────E
    ///        b=0
    ///
    /// Interpolate within (A, E, B, F, C, G, D, H) using fractional [r, g, b].
    /// ```
    pub fn apply(&self, rgb: [f32; 3]) -> [f32; 3] {
        let size = self.size;
        let size_m1 = (size - 1) as f32;

        // Normalize input to [0, size-1] range
        let r_norm = ((rgb[0] - self.domain_min[0]) / (self.domain_max[0] - self.domain_min[0]))
            .clamp(0.0, 1.0)
            * size_m1;
        let g_norm = ((rgb[1] - self.domain_min[1]) / (self.domain_max[1] - self.domain_min[1]))
            .clamp(0.0, 1.0)
            * size_m1;
        let b_norm = ((rgb[2] - self.domain_min[2]) / (self.domain_max[2] - self.domain_min[2]))
            .clamp(0.0, 1.0)
            * size_m1;

        // Integer grid coordinates
        let r0 = (r_norm.floor() as u32).min(size - 2);
        let g0 = (g_norm.floor() as u32).min(size - 2);
        let b0 = (b_norm.floor() as u32).min(size - 2);
        let r1 = r0 + 1;
        let g1 = g0 + 1;
        let b1 = b0 + 1;

        // Fractional part
        let fr = r_norm - r0 as f32;
        let fg = g_norm - g0 as f32;
        let fb = b_norm - b0 as f32;

        // Fetch 8 corner values
        let c000 = self.get(r0, g0, b0);
        let c100 = self.get(r1, g0, b0);
        let c010 = self.get(r0, g1, b0);
        let c110 = self.get(r1, g1, b0);
        let c001 = self.get(r0, g0, b1);
        let c101 = self.get(r1, g0, b1);
        let c011 = self.get(r0, g1, b1);
        let c111 = self.get(r1, g1, b1);

        // Trilinear interpolation
        let mut out = [0.0_f32; 3];
        for i in 0..3 {
            let c00 = c000[i] * (1.0 - fr) + c100[i] * fr;
            let c01 = c001[i] * (1.0 - fr) + c101[i] * fr;
            let c10 = c010[i] * (1.0 - fr) + c110[i] * fr;
            let c11 = c011[i] * (1.0 - fr) + c111[i] * fr;

            let c0 = c00 * (1.0 - fg) + c10 * fg;
            let c1 = c01 * (1.0 - fg) + c11 * fg;

            out[i] = c0 * (1.0 - fb) + c1 * fb;
        }
        out
    }

    /// Get a LUT entry by grid indices (r, g, b).
    #[inline]
    fn get(&self, ri: u32, gi: u32, bi: u32) -> [f32; 4] {
        let idx = (bi * self.size * self.size + gi * self.size + ri) as usize;
        self.data[idx]
    }

    /// Load a 3D LUT from an Iridas `.cube` file.
    ///
    /// Parses: `TITLE`, `DOMAIN_MIN`, `DOMAIN_MAX`, `LUT_3D_SIZE`, and data lines.
    pub fn load_cube(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);

        let mut size: u32 = 0;
        let mut domain_min = [0.0_f32; 3];
        let mut domain_max = [1.0_f32; 3];
        let mut data = Vec::new();

        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("TITLE") {
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("DOMAIN_MIN") {
                let vals: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if vals.len() == 3 {
                    domain_min = [vals[0], vals[1], vals[2]];
                }
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("DOMAIN_MAX") {
                let vals: Vec<f32> = rest
                    .split_whitespace()
                    .filter_map(|s| s.parse().ok())
                    .collect();
                if vals.len() == 3 {
                    domain_max = [vals[0], vals[1], vals[2]];
                }
                continue;
            }

            if let Some(rest) = trimmed.strip_prefix("LUT_3D_SIZE") {
                if let Some(s) = rest.split_whitespace().next()
                    && let Ok(v) = s.parse::<u32>()
                {
                    size = v;
                }
                continue;
            }

            // Skip other keywords
            if trimmed.starts_with(|c: char| c.is_ascii_alphabetic()) {
                continue;
            }

            // Data line: three floats
            let vals: Vec<f32> = trimmed
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if vals.len() >= 3 {
                data.push([vals[0], vals[1], vals[2], 1.0]);
            }
        }

        if size == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Missing LUT_3D_SIZE in .cube file",
            ));
        }

        let expected = (size as usize).pow(3);
        if data.len() != expected {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Expected {expected} entries for size {size}, got {}",
                    data.len()
                ),
            ));
        }

        Ok(Self {
            size,
            data,
            domain_min,
            domain_max,
        })
    }

    /// Save this 3D LUT to an Iridas `.cube` file.
    pub fn save_cube(&self, path: &Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        writeln!(writer, "TITLE \"Crispen LUT\"")?;
        writeln!(
            writer,
            "DOMAIN_MIN {:.6} {:.6} {:.6}",
            self.domain_min[0], self.domain_min[1], self.domain_min[2]
        )?;
        writeln!(
            writer,
            "DOMAIN_MAX {:.6} {:.6} {:.6}",
            self.domain_max[0], self.domain_max[1], self.domain_max[2]
        )?;
        writeln!(writer, "LUT_3D_SIZE {}", self.size)?;

        for entry in &self.data {
            writeln!(writer, "{:.6} {:.6} {:.6}", entry[0], entry[1], entry[2])?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transform::params::{ColorManagementConfig, ColorSpaceId, DisplayOetf};

    const EPSILON: f32 = 1e-5;

    fn identity_params() -> GradingParams {
        GradingParams {
            color_management: ColorManagementConfig {
                input_space: ColorSpaceId::AcesCg,
                working_space: ColorSpaceId::AcesCg,
                output_space: ColorSpaceId::AcesCg,
                display_oetf: DisplayOetf::Srgb,
            },
            ..GradingParams::default()
        }
    }

    #[test]
    fn test_lut_bake_matches_evaluate() {
        let params = identity_params();
        let size = 17;
        let mut lut = Lut3D::new(size);
        lut.bake(&params);

        let size_f = (size - 1) as f32;
        for bi in 0..size {
            for gi in 0..size {
                for ri in 0..size {
                    let r = ri as f32 / size_f;
                    let g = gi as f32 / size_f;
                    let b = bi as f32 / size_f;

                    let expected = evaluate_transform([r, g, b], &params);
                    let idx = (bi * size * size + gi * size + ri) as usize;
                    let actual = lut.data[idx];

                    for c in 0..3 {
                        assert!(
                            (actual[c] - expected[c]).abs() < EPSILON,
                            "Mismatch at ({ri},{gi},{bi}) ch{c}: {:.8} vs {:.8}",
                            actual[c],
                            expected[c]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_lut_trilinear_interpolation_is_smooth() {
        let params = identity_params();
        let mut lut = Lut3D::new(9);
        lut.bake(&params);

        let v1 = lut.apply([0.25, 0.25, 0.25]);
        let v2 = lut.apply([0.26, 0.25, 0.25]);
        assert!(
            (v1[0] - v2[0]).abs() < 0.05,
            "interpolation should be smooth"
        );
    }

    #[test]
    fn test_lut_apply_at_grid_point_matches_bake() {
        let params = identity_params();
        let size = 9;
        let mut lut = Lut3D::new(size);
        lut.bake(&params);

        let result = lut.apply([0.0, 0.0, 0.0]);
        for channel in result.iter().take(3) {
            assert!(channel.abs() < EPSILON);
        }
    }

    #[test]
    fn test_cube_file_roundtrip_data_matches() {
        let params = identity_params();
        let mut lut = Lut3D::new(5);
        lut.bake(&params);

        let dir = std::env::temp_dir();
        let path = dir.join("crispen_test_lut_roundtrip.cube");

        lut.save_cube(&path).expect("save should succeed");
        let loaded = Lut3D::load_cube(&path).expect("load should succeed");

        assert_eq!(lut.size, loaded.size);
        assert_eq!(lut.data.len(), loaded.data.len());

        for (i, (a, b)) in lut.data.iter().zip(loaded.data.iter()).enumerate() {
            for c in 0..3 {
                assert!(
                    (a[c] - b[c]).abs() < 1e-6,
                    "entry {i} ch{c}: {:.8} vs {:.8}",
                    a[c],
                    b[c]
                );
            }
        }

        let _ = std::fs::remove_file(&path);
    }
}
