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

use crate::SharedMidiState;
use crate::common_definitions::helpers::quantize_01_step;
use crate::common_definitions::params::{CcParam, ParamType, Parameterized};
use crate::sound_engine::sound_building::{SOUNDS, SoundFactory};
use fundsp::audiounit::AudioUnit;
use fundsp::prelude64::*;
use linkme::distributed_slice;
use std::borrow::Cow;

// todo: make this into morph2: 2 osc with custom morphing and leveling - 2 morph cc 2 volume cc for each osc
pub fn morph2(state: &SharedMidiState, params: &Parameterized) -> Box<dyn AudioUnit> {
    let osc_1a = params.get_osc_type("osc_1a").unwrap().get_osc();
    let osc_1b = params.get_osc_type("osc_1b").unwrap().get_osc();
    let osc_2a = params.get_osc_type("osc_2a").unwrap().get_osc();
    let osc_2b = params.get_osc_type("osc_2b").unwrap().get_osc();

    // todo: how to accept default as int but control cc with 0.0-1.0?
    let fm_ratio = *0.25 >> quantize_01_step(); // non cc controlled option not on by default steps of 0.5 up to 10??
    // if value is lower than 1.0 apply *10 and leave steps of 0.1
    let fm_amount_1 = *10; // modulation index cap to 10 cc option by default
    let fm_amount_2 = *10; // same

    let balance_1 = params.get_cc_param("balance_1").unwrap();
    let b1_cc = state.get_sound_an_or_default(balance_1);

    let balance_2 = params.get_cc_param("balance_2").unwrap();
    let b2_cc = state.get_sound_an_or_default(balance_2);

    // FM: osc(f * ratio) * (f * depth) + f >> sine()
    let osc_1b = ((state.bent_pitch() * fm_ratio) >> osc_1a.clone())
        * (state.bent_pitch() * fm_amount_1)
        + state.bent_pitch()
        >> osc_1b;
    let osc_2b = ((state.bent_pitch() * fm_ratio) >> osc_2a.clone())
        * (state.bent_pitch() * fm_amount_2)
        + state.bent_pitch()
        >> osc_2b;

    let morph1 = (osc_1a * (constant(1.0) - b1_cc.clone()) & osc_1b * b1_cc.clone()) * 2.0;
    let morph2 = (osc_2a * (constant(1.0) - b2_cc.clone()) & osc_2b * b2_cc) * 2.0;
    let synth = Box::new(morph1 + morph2);
    state.assemble_unpitched_sound(synth, state.boxed_adsr())
}

#[distributed_slice(SOUNDS)]
static MORPH2: SoundFactory = SoundFactory {
    builder: morph2,
    params: Parameterized {
        name: "morph2",
        cc_params: Some(Cow::Borrowed(&[
            CcParam {
                value: ParamType::ZeroOneFloat(0.5),
                cc_index: 1,
                name: "balance_1",
            },
            CcParam {
                value: ParamType::ZeroOneFloat(0.5),
                cc_index: 2,
                name: "balance_2",
            },
            CcParam {
                value: ParamType::ZeroOneFloat(0.5),
                cc_index: 0, // static value by default
                name: "balance_1",
            },
        ])),
        non_cc_params: None,
    },
};

// register_sound!(
//     name: "Square_saw_soft",
//     params: TwoOscMorphParams,
//     factory: saw_to_square,
//     cc_params: [("balance", 1)]
// );

//todo: add a general synth: pro6 style...2 oscillators with shapes cascading (saw, trianle, pulse) - detune control,
// todo: this should be an engine with 2 oscilators with independent levels (pulse width modulation too?), detune and pitch shit of 1 octave up and down
