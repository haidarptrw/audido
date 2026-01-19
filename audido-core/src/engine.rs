use anyhow::Context;
use rodio::{
    DeviceTrait, OutputStream, OutputStreamBuilder, Sink,
    cpal::{self, traits::HostTrait},
};

pub struct AudioEngine {
    stream: OutputStream,
    sink: Sink,
    device_name: String,
}

impl AudioEngine {
    fn try_new_default() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default output device found")?;

        let device_name = device.name().unwrap_or_else(|_| "(unknown)".to_string());

        let stream_builder = OutputStreamBuilder::from_device(device)
            .context("cannot create output stream builder from file")?;

        let stream = stream_builder.open_stream().context("Cannot create stream output")?;

        let sink = Sink::connect_new(&stream.mixer());

        Ok(AudioEngine { stream, sink, device_name })
    }
}
