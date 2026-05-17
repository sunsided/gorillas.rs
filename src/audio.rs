use std::time::Instant;

use anyhow::Result;
use synthie::params::AudioEvent;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundCue {
    Intro,
    GorillaIntro,
    Throw,
    BuildingExplosion,
    GorillaExplosion,
    VictoryDance,
}

#[allow(dead_code)]
pub(crate) struct AudioScheduler {
    _stream: cpal::Stream,
    tx: crossbeam_channel::Sender<AudioEvent>,
    queue: Vec<(Instant, AudioEvent)>,
}

#[allow(dead_code)]
pub(crate) fn cue_to_events(_cue: SoundCue, _start: Instant) -> Vec<(Instant, AudioEvent)> {
    todo!()
}

#[allow(dead_code)]
impl AudioScheduler {
    pub(crate) fn new() -> Result<Self> {
        todo!()
    }

    pub(crate) fn play(&mut self, _cue: SoundCue) {
        todo!()
    }

    pub(crate) fn tick(&mut self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_cue_all_variants_can_be_constructed() {
        let _ = [
            SoundCue::Intro,
            SoundCue::GorillaIntro,
            SoundCue::Throw,
            SoundCue::BuildingExplosion,
            SoundCue::GorillaExplosion,
            SoundCue::VictoryDance,
        ];
    }
}
