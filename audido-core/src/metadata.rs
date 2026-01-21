use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    Mono,
    Stereo,
    Unsupported,
}

impl Display for ChannelLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ChannelLayout::Mono => "Mono",
            ChannelLayout::Stereo => "Stereo",
            ChannelLayout::Unsupported => "Unsupported",
        };
        write!(f, "{}", label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicalSongKey {
    CMaj,
    CMin,
    CSharpMaj,
    CSharpMin,
    DMaj,
    DMin,
    DSharpMaj,
    DSharpMin,
    EMaj,
    EMin,
    FMaj,
    FMin,
    FSharpMaj,
    FSharpMin,
    GMaj,
    GMin,
    GSharpMaj,
    GSharpMin,
    AMaj,
    AMin,
    ASharpMaj,
    ASharpMin,
    BMaj,
    BMin,
}

impl Display for MusicalSongKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            MusicalSongKey::CMaj => "C",
            MusicalSongKey::CMin => "Cm",
            MusicalSongKey::CSharpMaj => "C#",
            MusicalSongKey::CSharpMin => "C#m",
            MusicalSongKey::DMaj => "D",
            MusicalSongKey::DMin => "Dm",
            MusicalSongKey::DSharpMaj => "D#",
            MusicalSongKey::DSharpMin => "D#m",
            MusicalSongKey::EMaj => "E",
            MusicalSongKey::EMin => "Em",
            MusicalSongKey::FMaj => "F",
            MusicalSongKey::FMin => "Fm",
            MusicalSongKey::FSharpMaj => "F#",
            MusicalSongKey::FSharpMin => "F#m",
            MusicalSongKey::GMaj => "G",
            MusicalSongKey::GMin => "Gm",
            MusicalSongKey::GSharpMaj => "G#",
            MusicalSongKey::GSharpMin => "G#m",
            MusicalSongKey::AMaj => "A",
            MusicalSongKey::AMin => "Am",
            MusicalSongKey::ASharpMaj => "A#",
            MusicalSongKey::ASharpMin => "A#m",
            MusicalSongKey::BMaj => "B",
            MusicalSongKey::BMin => "Bm",
        };

        write!(f, "{}", label)
    }
}

#[derive(Debug, Clone)]
pub struct AudioMetadata {
    /// Audio format (mp3, flac, wav, ogg, etc)
    pub format: String,
    /// sample rate / sampling frequency (f_s)
    pub sample_rate: u32,
    /// number of audio channels
    pub num_channels: u16,
    /// Channel layout (Mono or Stereo)
    pub channel_layout: ChannelLayout,
    /// Path to the audio sound file
    pub full_file_path: String,
    /// Audio title (if any)
    pub title: Option<String>,
    /// Audio author (if any, if many then separate each author by semicolon)
    pub author: Option<String>,
    /// Genre of the audio (if any)
    pub genre: Option<String>,
    /// Audio's tempo in Beat-per-minute (BPM) (if any)
    pub bpm: Option<f32>,
    /// Audio base key (will be computed internally using DSP)
    pub key: Option<MusicalSongKey>,
    /// Audio's duration in seconds
    pub duration: f32,
    /// Album of the music (if provided any)
    pub album: Option<String>,
    /// Audio danceability (computed internally)
    pub danceability: Option<f32>,
    /// Audio acousticness (computed internally)
    pub acousticness: Option<f32>,

    pub electronicness: Option<f32>,
    // Add more in the future (optional)
    // pub lyric: Option<LyricData> // LyricData store lyrics and each part's timestamp
}

impl Default for AudioMetadata {
    fn default() -> Self {
        Self {
            format: String::new(),
            sample_rate: 0,
            num_channels: 0,
            channel_layout: ChannelLayout::Unsupported,
            full_file_path: String::new(),
            title: None,
            author: None,
            genre: None,
            bpm: None,
            key: None,
            duration: 0.0,
            album: None,
            danceability: None,
            acousticness: None,
            electronicness: None,
        }
    }
}

impl Display for AudioMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mins = (self.duration / 60.0).floor() as u64;
        let secs = (self.duration % 60.0).floor() as u64;
        let title = self.title.as_deref().unwrap_or("Unknown Title");
        let author = self.author.as_deref().unwrap_or("Unknown Artist");
        let album = self.album.as_deref().unwrap_or("Unknown Album");

        writeln!(f, "Track:  {} - {}", title, author)?;
        writeln!(f, "Album:  {}", album)?;
        writeln!(f, "Length: {:02}:{:02}", mins, secs)?;

        writeln!(
            f,
            "Format: {} ({:.1} kHz, {})",
            self.format.to_uppercase(),
            self.sample_rate / 1000,
            self.channel_layout
        )?;

        if let Some(bpm) = self.bpm {
            write!(f, "BPM:    {:.1}", bpm)?;
            if let Some(key) = &self.key {
                write!(f, " | Key: {}", key)?;
            }
            writeln!(f)?;
        } else if let Some(key) = &self.key {
            // If only Key is available but not BPM
            writeln!(f, "Key:    {}", key)?;
        }

        Ok(())
    }
}
