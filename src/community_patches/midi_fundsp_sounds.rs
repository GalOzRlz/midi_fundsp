use crate::SoundBuilder;
use crate::patch_builder::{CcMap, ParamDefault, ParamInfo, ParamType, SoundEntry, SoundParams};
use crate::patch_helpers::Adsr;
use crate::{SharedMidiState, SynthFunc, register_sound};
use fundsp::audiounit::AudioUnit;
use fundsp::prelude64::*;
use std::collections::HashMap;
use std::sync::Arc;
use toml::Table;

pub struct TwoOscMixParams {
    pub dummy: f64,
}

impl SoundParams for TwoOscMixParams {
    fn from_table(table: &Table) -> Self {
        Self {
            dummy: table.get("volume").and_then(|v| v.as_float()).unwrap_or(0.8),
        }
    }

    fn param_info() -> &'static [ParamInfo] {
        &[
            ParamInfo {
                name: "volume",
                param_type: ParamType::Float,
                default: ParamDefault::Float(0.5),
            },
        ]
    }
}

fn basic_pluck() -> Box<dyn AudioUnit> {
    Box::new((square() & saw()) >> lowpass_hz(3000.0, 0.5))
}

//todo: make this into a general synth: pro style...2 oscillators with shapes cascading (saw, trianle, pulse) - detune control,
// todo: this should be an engine with 2 oscilators with independent levels (pulse width modulation too?), detune and pitch shit of 1 octave up and down
pub fn saw_to_square(_params: &TwoOscMixParams, state: &SharedMidiState) -> Box<dyn AudioUnit> {
    let b_cc = state.get_sound_control_change(1);
    let synth = (square() * (constant(1.0) - b_cc.clone()) & saw() * b_cc) * 2.0 >> lowpass_hz(8000.0, 0.5);
    state.assemble_unpitched_sound(basic_pluck(), state.boxed_adsr())
}

register_sound!(
    name: "Square_saw_soft",    // display name & base for struct name
    params: TwoOscMixParams,
    factory: saw_to_square,
    cc_params: [("balance", 1)]   // CC param: name, default knob index, default value
);

