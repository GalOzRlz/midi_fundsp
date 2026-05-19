use crate::patch_helpers::Adsr;
use crate::patch_builder::SoundEntry;
use crate::{register_sound, SharedMidiState};
use fundsp::audiounit::AudioUnit;
use fundsp::prelude64::*;

fn basic_pluck() -> Box<dyn AudioUnit> {
    Box::new((square() & saw()) >> lowpass_hz(3000.0, 0.5))
}

//todo: make this into a general synth: pro style...waveshaper with 2 shapes, 3 shapes, 2, with some detune control,
// todo: this should be an engine with 2 oscilators with independent levels (pulse width modulation too?), detune and pitch shit of 1 octave up and down
pub fn saw_square_soft(state: &SharedMidiState) -> Box<dyn AudioUnit> {
    state.assemble_unpitched_sound(basic_pluck(), state.boxed_adsr())
}

register_sound!("Square_saw_soft", saw_square_soft);