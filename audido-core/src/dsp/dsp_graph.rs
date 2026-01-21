use crate::dsp::eq::Equalizer;

pub enum DspNode {
    Equalizer {
        on: bool,
        instance: Equalizer
    }
}

pub struct DspGraph {
    nodes: Vec<DspNode>
}