// use crate::instruments::{dirty_guitar, hit_comb_pipe, pluck_comb_string};
// use crate::patch_builder::*;
// use crate::patch_helpers::Adsr;
// use crate::{register_sound, SharedMidiState};
// use fundsp::prelude::{lowpass_hz, shape, AudioUnit};
// use fundsp::prelude64::{constant, sine_hz, Atan};
// use fundsp::shape::Tanh;
//
//
// pub fn harpsichord(state: &SharedMidiState) -> Box<dyn AudioUnit> {
//     state.adsr.configure(
//         0.005,
//         0.8,
//         0.0,
//         0.0,
//     );
//     let gate = state.control_var().clone();
//     let mix = (state.bent_pitch().clone() | gate | constant(0.0))
//         >> pluck_comb_string()
//         >> lowpass_hz(9000.0, 0.5);
//     state.assemble_pitched_sound(Box::new(mix), state.boxed_adsr())
// }
//
// pub fn plastic_pipe(state: &SharedMidiState) -> Box<dyn AudioUnit> {
//     let adsr = state.adsr.clone();
//     adsr.attack.set_value(12.3);
//     let gate = state.control_var().clone();
//     let mix = (state.bent_pitch().clone() | gate | constant(0.0))
//         >> hit_comb_pipe() * 5.0
//         >> shape(Tanh(1.0))
//         >> lowpass_hz(7000.0, 0.5);
//     state.assemble_pitched_sound(Box::new(mix), state.boxed_adsr())
// }
//
// pub fn chorused_dirty_guitar(state: &SharedMidiState) -> Box<dyn AudioUnit> {
//     state.adsr.configure(
//          0.005,
//          0.8,
//          1.0,
//          0.5,
//     );
//     let base_pitch = state.bent_pitch();
//     let lfo1 = sine_hz(3.0) * 0.0065;
//     let pitch1 = base_pitch.clone() * (constant(1.0) + lfo1);
//     let gate = state.control_var();
//     let dg = dirty_guitar();
//     state.assemble_pitched_sound(Box::new(dg(pitch1, gate.clone()) * 6.6 >> shape(Atan(5.0))), state.boxed_adsr())
// }
//
// register_sound!("chorused_dirty_guitar", chorused_dirty_guitar);
// register_sound!("plastic_pipe", plastic_pipe);

use crate::SoundBuilder;
use crate::patch_builder::{ParamDefault, ParamInfo, ParamType, Parameters, SoundEntry};
use crate::{SharedMidiState, register_sound};
use fundsp::audiounit::AudioUnit;
use fundsp::prelude64::*;
use toml::Table;

pub enum OscillatorType {
    Saw,
    Triangle,
    Sine,
    Pulse, // todo: add Pulse Width
    Square,
    None,
}

impl OscillatorType {
    pub fn to_audiounit(&self) -> Box<dyn AudioUnit> {
        match self {
            OscillatorType::Saw => Box::new(poly_saw()),
            OscillatorType::Triangle => Box::new(triangle()),
            OscillatorType::Sine => Box::new(sine()),
            OscillatorType::Pulse => Box::new(poly_pulse()),
            OscillatorType::Square => Box::new(poly_square()),
            OscillatorType::None => Box::new(sine() * constant(0.0)),
        }
    }
    pub fn from_string(s: &str) -> OscillatorType {
        match s {
            "saw" => OscillatorType::Saw,
            "triangle" => OscillatorType::Triangle,
            "sine" => OscillatorType::Sine,
            "pulse" => OscillatorType::Pulse,
            "square" => OscillatorType::Square,
            _ => OscillatorType::None,
        }
    }
    pub fn from_string_to_audiounit(s: &str) -> Box<dyn AudioUnit> {
        OscillatorType::from_string(s).to_audiounit()
    }
}

pub struct TwoOscMixParams {
    pub dummy: f64,
    pub oscillator_type_1: OscillatorType,
    pub oscillator_type_2: OscillatorType,
}

impl Parameters for TwoOscMixParams {
    fn from_table(table: &Table) -> Self {
        let osc1_str = table.get("osc1").and_then(|v| v.as_str()).unwrap_or("None");
        let osc2_str = table.get("osc2").and_then(|v| v.as_str()).unwrap_or("None");
        Self {
            dummy: table
                .get("volume")
                .and_then(|v| v.as_float())
                .unwrap_or(0.8),
            oscillator_type_1: OscillatorType::from_string(&osc1_str),
            oscillator_type_2: OscillatorType::from_string(&osc2_str),
        }
    }

    // todo: complete this properly

    fn param_info() -> &'static [ParamInfo] {
        &[ParamInfo {
            name: "volume",
            param_type: ParamType::Float,
            default: ParamDefault::Float(0.5),
        }]
    }
}

fn basic_pluck() -> Box<dyn AudioUnit> {
    Box::new((square() & saw()) >> lowpass_hz(3000.0, 0.5))
}

//todo: make this into a general synth: pro style...2 oscillators with shapes cascading (saw, trianle, pulse) - detune control,
// todo: this should be an engine with 2 oscilators with independent levels (pulse width modulation too?), detune and pitch shit of 1 octave up and down
pub fn saw_to_square(_params: &TwoOscMixParams, state: &SharedMidiState) -> Box<dyn AudioUnit> {
    let b_cc = state.get_sound_control_change(1);
    let synth = Box::new(
        (square() * (constant(1.0) - b_cc.clone()) & saw() * b_cc) * 2.0
            >> lowpass_hz(10000.0, 0.5),
    );
    state.assemble_unpitched_sound(synth, state.boxed_adsr())
}

register_sound!(
    name: "Square_saw_soft",    // display name & base for struct name
    params: TwoOscMixParams,
    factory: saw_to_square,
    cc_params: [("balance", 1)]   // CC param: name, default knob index, default value
);
