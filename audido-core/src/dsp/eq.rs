
pub const MAX_EQ_FILTERS: usize = 8;

pub enum FilterType {
    Peaking,
    LowPass,
    HighPass,
    LowShelf,
    HighShelf
}

pub struct FilterNode {
    pub id: i16,
    pub filter_type: FilterType,
    pub freq: f32, // in Hz
    pub gain: f32, // in dB
}

pub enum EqPreset {
    Flat,
    Acoustic,
    Dance,
    EDM,
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
            EqPreset::EDM => vec![],
            EqPreset::BassBoosted => vec![],
            EqPreset::Custom => vec![],
        }
    }
}

pub struct Equalizer {
    pub sample_rate: u32,
    pub preset: EqPreset,
    pub filters: Vec<FilterNode>
}

impl Equalizer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            preset: EqPreset::Flat,
            filters: Vec::with_capacity(MAX_EQ_FILTERS),
        }
    }
}