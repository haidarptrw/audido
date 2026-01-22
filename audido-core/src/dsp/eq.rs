// Implementation for Parametric EQ
// The algorithm is based on RBJ Audio EQ Cookbook

use std::f32::consts::PI;

pub const MAX_EQ_FILTERS: usize = 8;

/// Filter type: Use Direct Form II Biquad Filter
#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    #[default]
    Peaking,
    LowPass,
    HighPass,
    LowShelf,
    HighShelf,
    BandPass,
    Notch,
}

pub struct FilterNode {
    pub id: i16,
    pub filter_type: FilterType,
    /// cutoff frequence of the filter (in Hz)
    pub freq: f32,
    /// Filter gain in dB
    pub gain: f32,
    /// Filter Q Factor/resonance
    pub q: f32,
    /// Filter order (1 = 6dB/oct, 2 = 12dB/oct, 4 = 24dB/oct, etc)
    pub order: u8,
}

impl FilterNode {
    pub fn new(id: i16, freq: f32) -> Self {
        Self {
            id,
            filter_type: FilterType::Peaking,
            freq,
            gain: 0.0,
            q: 0.707,
            order: 2,
        }
    }
}

impl Default for FilterNode {
    fn default() -> Self {
        Self {
            id: 0,
            filter_type: FilterType::Peaking,
            freq: 1000.0,
            gain: 0.0,
            q: 0.707,
            order: 2,
        }
    }
}

/// implement Biquad Filter (Direct Form II Transposed)
/// $$ y[n] = frac{b0/a0}x[n] + frac{b1/a0}x[n-1] + frac{b2/a0}x[n-2] - frac{a1/a0}y[n-1] - frac{a2/a0}y[n-2] $$
#[derive(Clone, Default)]
struct Biquad {
    // Coefficients
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,
    // Previous State
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn process(&mut self, sample: f32) -> f32 {
        // Direct Form II Transposed difference equation
        // y[n] = b0*x[n] + z1[n-1]
        // z1[n] = b1*x[n] - a1*y[n] + z2[n-1]
        // z2[n] = b2*x[n] - a2*y[n]

        let out = self.b0 * sample + self.z1;
        self.z1 = self.b1 * sample - self.a1 * out + self.z2;
        self.z2 = self.b2 * sample - self.a2 * out;

        out
    }

    /// Recalculate coefficients
    fn update(&mut self, filter: &FilterNode, sample_rate: f32) {
        let w0 = 2.0 * PI * filter.freq / sample_rate;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * filter.q);
        
        // amplitude in linear scale (converted from dB)
        // A = 10^(Adb / 40.0)
        let a = 10.0f32.powf(filter.gain / 40.0);

        let (b0, b1, b2, a0, a1, a2) =
            Self::calculate_coefficients(cos_w0, alpha, a, filter.filter_type);

        // Normalize coefficients by a0
        let inv_a0 = 1.0 / a0;
        self.b0 = b0 * inv_a0;
        self.b1 = b1 * inv_a0;
        self.b2 = b2 * inv_a0;
        self.a1 = a1 * inv_a0;
        self.a2 = a2 * inv_a0;
    }

    /// Helper to calculate (b0, b1, b2, a0, a1, a2)
    fn calculate_coefficients(
        cos_w0: f32,
        alpha: f32,
        a: f32,
        filter_type: FilterType,
    ) -> (f32, f32, f32, f32, f32, f32) {
        match filter_type {
            FilterType::Peaking => (
                1.0 + alpha * a,
                -2.0 * cos_w0,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cos_w0,
                1.0 - alpha / a,
            ),
            FilterType::LowPass => (
                (1.0 - cos_w0) / 2.0,
                1.0 - cos_w0,
                (1.0 - cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::HighPass => (
                (1.0 + cos_w0) / 2.0,
                -(1.0 + cos_w0),
                (1.0 + cos_w0) / 2.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
            FilterType::LowShelf => {
                let sqrt_a = a.sqrt();
                (
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha),
                    2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
                    a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha),
                    (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha,
                    -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
                    (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha,
                )
            }
            FilterType::HighShelf => {
                let sqrt_a = a.sqrt();
                (
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
                    a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha),
                    (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
                    (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha,
                )
            }
            FilterType::BandPass => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha),
            FilterType::Notch => (
                1.0,
                -2.0 * cos_w0,
                1.0,
                1.0 + alpha,
                -2.0 * cos_w0,
                1.0 - alpha,
            ),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EqPreset {
    Flat,
    Acoustic,
    Dance,
    Electronic,
    BassBoosted,
    Custom,
    // ...
}

impl EqPreset {
    pub fn set_filters(&self) -> Vec<FilterNode> {
        match self {
            EqPreset::Flat => vec![],
            EqPreset::Acoustic => vec![],
            EqPreset::Dance => vec![],
            EqPreset::Electronic => vec![],
            EqPreset::BassBoosted => vec![FilterNode {
                id: 1,
                filter_type: FilterType::LowShelf,
                freq: 100.0,
                gain: 6.0,
                q: 0.707,
                order: 2,
            }],
            EqPreset::Custom => vec![],
        }
    }
}

pub struct Equalizer {
    pub sample_rate: u32,
    pub preset: EqPreset,
    pub filters: Vec<FilterNode>,
    /// Internal DSP state (vector of vector because one node can have multiple biquads for high order)
    processors: Vec<Vec<Biquad>>,
    pub master_gain: f32,
}

impl Equalizer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            preset: EqPreset::Flat,
            filters: Vec::with_capacity(MAX_EQ_FILTERS),
            processors: Vec::with_capacity(MAX_EQ_FILTERS),
            master_gain: 1.0,
        }
    }

    pub fn process_frame(&mut self, frame: &mut [f32]) {
        if (self.master_gain - 1.0).abs() > f32::EPSILON {
            for sample in frame.iter_mut() {
                *sample *= self.master_gain;
            }
        }

        for sample in frame.iter_mut() {
            let mut s = *sample;

            // Run through every filter chain
            for chain in &mut self.processors {
                for biquad in chain {
                    s = biquad.process(s);
                }
            }

            *sample = s;
        }
    }

    pub fn update_preset(&mut self, preset: EqPreset) {
        if preset != self.preset {
            self.preset = preset;
            let config = self.preset.set_filters();
            self.filters = config;

            // Rebuild the DSP processors to reflect the new config
            self.rebuild_processors();
        }
    }

    fn rebuild_processors(&mut self) {
        self.processors.clear();

        for filter_node in &self.filters {
            // A standard Biquad is 2nd order (12dB/oct).
            // For order 4 (24dB/oct), we need 2 biquads.
            // For order 1 (6dB/oct), we technically need 0.5 biquads, but we treat it as order 2 with reduced slope logic

            let num_biquads = (filter_node.order as f32 / 2.0).ceil() as usize;
            let count = if num_biquads == 0 { 1 } else { num_biquads };

            let mut chain = Vec::with_capacity(count);
            for _ in 0..count {
                let mut bq = Biquad::default();
                bq.update(filter_node, self.sample_rate as f32);
                chain.push(bq);
            }
            self.processors.push(chain);
        }
    }

    pub fn parameters_changed(&mut self) {
        // Only update coefficients, try to preserve state (z1, z2) to avoid popping
        // This requires matching filters by ID or index.
        // For simplicity here, we do a full rebuild which might click slightly.
        // A better production approach matches existing Biquads and calls .update() on them.

        if self.processors.len() != self.filters.len() {
            self.rebuild_processors();
            return;
        }

        for (i, filter_node) in self.filters.iter().enumerate() {
            let chain = &mut self.processors[i];

            // If order changed, we must rebuild this specific chain
            let required_biquads = (filter_node.order as f32 / 2.0).ceil() as usize;
            let count = if required_biquads == 0 {
                1
            } else {
                required_biquads
            };

            if chain.len() < count {
                // Order increased: Add new biquads (zero state)
                // Existing biquads are untouched
                chain.resize_with(count, Biquad::default);
            } else if chain.len() > count {
                // Order decreased: Remove extra biquads
                // The remaining ones keep their state
                chain.truncate(count);
            }

            // Update coefficients for all biquads in the chain
            for biquad in chain.iter_mut() {
                biquad.update(filter_node, self.sample_rate as f32);
            }
        }
    }
}
