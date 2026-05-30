use crate::SharedMidiState;
use crate::common::helpers::quantize_u8_to_01;
use crate::config_builder::{ConfigurableMapping, MAX_KNOBS_PER_GROUP};
use crate::helpers::fundsp::to_net;
use anyhow::anyhow;
use fundsp::audionode::Pipe;
use fundsp::follow::Follow;
use fundsp::prelude::{join, pulse};
use fundsp::prelude32::Var;
use fundsp::prelude64::{An, Net, U1, U2, Unit, pass, poly_saw, poly_square, sine, triangle, unit};
use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;
use toml::Value;

pub type CcAudioNode = An<Pipe<Var, Follow<f64>>>;

impl ToNet for CcAudioNode {
    fn to_net(self) -> Net {
        to_net(self)
    }
}

pub trait ToNet {
    fn to_net(self) -> Net;
}
pub trait CcInit {
    fn get_initial_cc(&self) -> [f32; MAX_KNOBS_PER_GROUP];
}

#[derive(Debug, Clone)]
pub enum ParamType {
    U8(u8),
    String(Cow<'static, str>),
    ZeroOneFloat(f32),
    ZeroHundredFloat(f32),
}

impl ParamType {
    pub fn as_zero_to_one_f32(&self) -> Option<f32> {
        match &self {
            ParamType::U8(v) => Some(quantize_u8_to_01(*v.clamp(&0, &127))),
            ParamType::String(_) => None,
            ParamType::ZeroOneFloat(v) => Some(v.clamp(0.0, 1.0)),
            ParamType::ZeroHundredFloat(v) => Some((*v / 100.0).clamp(0.0, 1.0)),
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        match &self {
            ParamType::U8(v) => Some(*v as f32),
            ParamType::String(_) => None,
            ParamType::ZeroOneFloat(v) => Some(v.clamp(0.0, 1.0)),
            ParamType::ZeroHundredFloat(v) => Some(*v),
        }
    }

    pub fn as_oscillator_type(&self) -> Result<OscillatorType, &'static str> {
        match self {
            ParamType::String(s) => OscillatorType::from_str(s),
            _ => Err("parameter is not a string, cannot convert to oscillator type"),
        }
    }
}

impl std::fmt::Display for ParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamType::U8(v) => write!(f, "{}", v),
            ParamType::String(s) => write!(f, "{}", s),
            ParamType::ZeroOneFloat(v) => write!(f, "{}", v),
            ParamType::ZeroHundredFloat(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CcParam {
    pub value: ParamType,
    pub cc_index: usize,
    pub name: &'static str,
    pub description: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub struct NonCcParam {
    pub value: ParamType,
    pub name: &'static str,
    pub description: Option<&'static str>,
}

#[derive(Clone)]
pub struct Parameterized {
    pub name: &'static str,
    pub cc_params: Option<Cow<'static, [CcParam]>>,
    pub non_cc_params: Option<Cow<'static, [NonCcParam]>>,
}

impl CcInit for Parameterized {
    fn get_initial_cc(&self) -> [f32; MAX_KNOBS_PER_GROUP] {
        let mut cc_array = [0_f32; MAX_KNOBS_PER_GROUP];
        if let Some(cc_params_cow) = &self.cc_params {
            for cc_param in cc_params_cow.iter() {
                cc_array[cc_param.cc_index] = cc_param.value.as_zero_to_one_f32().unwrap()
            }
        }
        cc_array
    }
}

impl Parameterized {
    pub fn apply_toml_overrides<T>(&mut self, toml_config: &T)
    where
        T: ConfigurableMapping,
    {
        if let Some(cfg) = toml_config.get_config() {
            if let Some(mut_cc_params) = self.cc_params.as_mut() {
                apply_toml_values_overrides(mut_cc_params.to_mut(), &cfg);
            }
            if let Some(mut_non_cc_params) = self.non_cc_params.as_mut() {
                apply_toml_values_overrides(mut_non_cc_params.to_mut(), &cfg);
            }
        }
        if let Some(user_mappings) = toml_config.get_mapping() {
            apply_toml_mapping(self, user_mappings);
        }
    }

    pub fn get_cc_param(&self, name: &str) -> anyhow::Result<&CcParam> {
        if let Some(vec) = &self.cc_params {
            for i in vec.iter() {
                if i.name == name {
                    return Ok(i);
                }
            }
        }
        Err(anyhow!(format!("CC-Parameter {} not found", name)))
    }

    pub fn cc_sound_or_default(&self, name: &str, shared: &SharedMidiState) -> CcAudioNode {
        shared.get_sound_an_or_default(&self.get_cc_param(name).unwrap())
    }

    pub fn cc_fx_or_default(&self, name: &str, shared: &SharedMidiState) -> CcAudioNode {
        shared.get_fx_an_or_default(&self.get_cc_param(name).unwrap())
    }

    pub fn get_non_cc_param(&self, name: &str) -> anyhow::Result<&NonCcParam> {
        if let Some(vec) = &self.non_cc_params {
            for i in vec.iter() {
                if i.name == name {
                    return Ok(i);
                }
            }
        }
        Err(anyhow!(format!("Non-CC-Parameter {} not found", name)))
    }
    pub fn get_osc_node_type(&self, name: &str) -> anyhow::Result<OscillatorType> {
        let param = self
            .get_non_cc_param(name)
            .map_err(|_| anyhow::anyhow!("parameter not found"))?;
        param
            .value
            .as_oscillator_type()
            .map_err(|e| anyhow::anyhow!(e))
    }
}

pub trait ValuedParam {
    fn get_mut(&mut self) -> &mut ParamType;

    fn get_name(&self) -> &str;
}

impl ValuedParam for CcParam {
    fn get_mut(&mut self) -> &mut ParamType {
        &mut self.value
    }

    fn get_name(&self) -> &str {
        self.name
    }
}

impl ValuedParam for NonCcParam {
    fn get_mut(&mut self) -> &mut ParamType {
        &mut self.value
    }

    fn get_name(&self) -> &str {
        self.name
    }
}

pub fn apply_toml_values_overrides<T>(params: &mut [T], toml_overrides: &HashMap<String, Value>)
where
    T: ValuedParam,
{
    for param in params {
        if let Some(toml_value) = toml_overrides.get(param.get_name()) {
            match param.get_mut() {
                ParamType::ZeroOneFloat(v) | ParamType::ZeroHundredFloat(v) => {
                    if let Some(num) = toml_value.as_float() {
                        *v = num as f32;
                    }
                }
                ParamType::U8(v) => {
                    if let Some(num) = toml_value.as_integer() {
                        *v = num as u8;
                    } else if let Some(num) = toml_value.as_float() {
                        *v = num as u8;
                    }
                }
                ParamType::String(s) => {
                    if let Some(str_val) = toml_value.as_str() {
                        *s = osc_string_to_cow(str_val);
                    }
                }
            }
        }
    }
}

pub fn apply_toml_mapping(params: &mut Parameterized, toml_mapping: &HashMap<String, Value>) {
    if let Some(ref mut cc_params) = params.cc_params {
        let params_mut = cc_params.to_mut();
        for param in params_mut.iter_mut() {
            if let Some(val) = toml_mapping.get(param.name).and_then(|v| v.as_integer()) {
                param.cc_index = val as usize
            }
        }
    }
}

pub enum Polarity {
    Positive,
    Negative,
}

impl Polarity {
    pub(crate) fn to_float(&self) -> f32 {
        match self {
            Polarity::Positive => 1.0,
            Polarity::Negative => -1.0,
        }
    }
}

#[derive(serde::Deserialize)]
pub enum OscillatorType {
    Saw,
    Triangle,
    Sine,
    Pulse,
    Square,
    WhiteNoise,
    BrownNoise,
    PinkNoise,
    WaveTable(String), // todo: how to implement?!
    None,
}

impl FromStr for OscillatorType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        type Err = &'static str;
        let lower = s.to_lowercase();
        match lower.as_str() {
            "saw" => Ok(OscillatorType::Saw),
            "triangle" => Ok(OscillatorType::Triangle),
            "sine" => Ok(OscillatorType::Sine),
            "pulse" => Ok(OscillatorType::Pulse),
            "square" => Ok(OscillatorType::Square),
            "none" => Ok(OscillatorType::None),
            // we assume it's a file path
            file_path => Ok(OscillatorType::WaveTable(file_path.parse().unwrap())), // todo: add other tables!
        }
    }
}
fn osc_string_to_cow(s: &str) -> Cow<'static, str> {
    match s {
        "saw" => Cow::Borrowed("saw"),
        "triangle" => Cow::Borrowed("triangle"),
        "sine" => Cow::Borrowed("sine"),
        "pulse" => Cow::Borrowed("pulse"),
        "square" => Cow::Borrowed("square"),
        "none" => Cow::Borrowed("none"),
        // Any other string (file path, custom name) – take ownership
        other => Cow::Owned(other.to_string()),
    }
}

impl OscillatorType {
    pub fn get_osc_node(&self) -> An<Unit<U1, U1>> {
        match self {
            OscillatorType::Saw => unit::<U1, U1>(Box::new(poly_saw())),
            OscillatorType::Triangle => unit::<U1, U1>(Box::new(triangle())),
            OscillatorType::Sine => unit::<U1, U1>(Box::new(sine())),
            OscillatorType::Pulse => unit::<U1, U1>(Box::new(poly_square())),
            OscillatorType::Square => unit::<U1, U1>(Box::new(poly_square())),
            OscillatorType::None => unit::<U1, U1>(Box::new(sine() * 0.0)),
            _ => todo!(), // todo: load wavetable with path!
        }
    }
    pub fn get_osc_node_pw(&self) -> An<Unit<U2, U1>> {
        // nullify the second value for osc that don't support pulse width
        let sinker = (pass() | pass() * 0.0) >> join::<U2>();
        match self {
            OscillatorType::Saw => unit::<U2, U1>(Box::new(sinker >> self.get_osc_node())),
            OscillatorType::Triangle => unit::<U2, U1>(Box::new(sinker >> self.get_osc_node())),
            OscillatorType::Sine => unit::<U2, U1>(Box::new(sinker >> self.get_osc_node())),
            OscillatorType::Pulse => unit::<U2, U1>(Box::new(pulse())),
            OscillatorType::Square => unit::<U2, U1>(Box::new(sinker >> self.get_osc_node())),
            OscillatorType::None => unit::<U2, U1>(Box::new(sinker >> sine() * 0.0)),
            _ => todo!(),
        }
    }
}
