pub enum Audiocommand {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    Seek(f32),
    SetVolume(f32),
    SetSpeed(f32),
    SetPitch(f32),
    SetEqualizer(f32),
}