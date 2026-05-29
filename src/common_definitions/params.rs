use crate::config_builder::{ConfigurableMapping, MAX_KNOBS_PER_GROUP};
use anyhow::anyhow;
use fundsp::prelude64::{An, U1, Unit, poly_pulse, poly_saw, poly_square, sine, triangle, unit};
use serde::{Deserialize, Deserializer};
use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;
use toml::Value;

pub trait CcInit {
    fn get_initial_cc(&self) -> [f32; MAX_KNOBS_PER_GROUP];
}

#[derive(Debug, Clone)]
pub enum ParamType {
    Float(f32),
    Int(usize),
    String(String),
    ZeroToOneFloat(f32),
}

impl ParamType {
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            ParamType::Float(v) => Some(*v),
            ParamType::Int(v) => Some(*v as f32),
            ParamType::String(_) => None,
            ParamType::ZeroToOneFloat(v) => (Some(v.clamp(0.0, 1.0))),
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
            ParamType::Float(v) => write!(f, "{}", v),
            ParamType::Int(v) => write!(f, "{}", v),
            ParamType::String(s) => write!(f, "{}", s),
            ParamType::ZeroToOneFloat(v) => write!(f, "{}", v),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CcParam {
    pub value: ParamType,
    pub cc_index: usize,
    pub name: &'static str,
}

#[derive(Debug, Clone)]
pub struct NonCcParam {
    pub value: ParamType,
    pub name: &'static str,
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
        for cc_params_cow in &self.cc_params {
            for cc_param in cc_params_cow.iter() {
                cc_array[cc_param.cc_index] = cc_param.value.as_f32().unwrap()
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
    pub fn get_osc_type(&self, name: &str) -> anyhow::Result<OscillatorType> {
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
                ParamType::Float(v) | ParamType::ZeroToOneFloat(v) => {
                    if let Some(num) = toml_value.as_float() {
                        *v = num as f32;
                    }
                }
                ParamType::Int(v) => {
                    if let Some(num) = toml_value.as_integer() {
                        *v = num as usize;
                    } else if let Some(num) = toml_value.as_float() {
                        *v = num as usize;
                    }
                }
                ParamType::String(s) => {
                    if let Some(str_val) = toml_value.as_str() {
                        *s = str_val.to_string();
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

fn deserialize_polarity_type<'de, D>(deserializer: D) -> Result<Polarity, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.to_lowercase().as_str() {
        "positive" => Ok(Polarity::Positive),
        "negative" => Ok(Polarity::Negative),
        _ => Err(serde::de::Error::unknown_variant(
            &s,
            &["positive", "negative"],
        )),
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
    Pulse, // todo: add Pulse Width
    Square,
    None,
}

impl FromStr for OscillatorType {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "saw" => Ok(OscillatorType::Saw),
            "triangle" => Ok(OscillatorType::Triangle),
            "sine" => Ok(OscillatorType::Sine),
            "pulse" => Ok(OscillatorType::Pulse),
            "square" => Ok(OscillatorType::Square),
            "none" => Ok(OscillatorType::None),
            _ => Err("unknown oscillator type"),
        }
    }
}

impl OscillatorType {
    pub fn get_osc(&self) -> An<Unit<U1, U1>> {
        match self {
            OscillatorType::Saw => unit::<U1, U1>(Box::new(poly_saw())),
            OscillatorType::Triangle => unit::<U1, U1>(Box::new(triangle())),
            OscillatorType::Sine => unit::<U1, U1>(Box::new(sine())),
            OscillatorType::Pulse => unit::<U1, U1>(Box::new(poly_pulse())),
            OscillatorType::Square => unit::<U1, U1>(Box::new(poly_square())),
            OscillatorType::None => unit::<U1, U1>(Box::new(sine() * 0.0)),
        }
    }
}
