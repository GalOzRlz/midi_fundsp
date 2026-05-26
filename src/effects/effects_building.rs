use crate::SharedMidiState;
use crate::common_definitions::params::{ParamType, Parameterized};
use crate::config_builder::TomlEffectSection;
use crate::effects::helpers::to_stereo;
use crate::effects::master_fx::EFFECTS;
use fundsp::prelude64::{Net, NodeId};
use std::collections::HashMap;
use std::sync::Arc;
use toml::Table;

pub type EffectFunc = Box<dyn Fn(&SharedMidiState) -> Net + Send + Sync + 'static>;

pub type EffectFactory = fn(construction: &Table, knob_map: &HashMap<String, usize>) -> EffectFunc;

#[derive(Clone)]
pub struct EffectDef {
    pub factory: fn(Parameterized) -> EffectFunc,
    pub params: Parameterized,
}

#[derive(Clone)]
pub struct FxChainFactory {
    pub chain: Arc<Vec<EffectFunc>>,
    pub node_ids: Option<Vec<NodeId>>,
    pub definitions: Option<Vec<Parameterized>>,
    pub fx_names: Option<Vec<String>>,
}

impl FxChainFactory {
    pub fn initial_cc(&self) -> Vec<f32> {
        todo!()
    }

    pub fn connect_node_vec(&mut self, node_vec: Arc<Vec<Net>>) -> Net {
        let mut nodeid_vec: Vec<NodeId> = Vec::with_capacity(node_vec.len());
        let nodes = (*node_vec).clone();
        let mut net = Net::new(2, 2);
        for node in nodes {
            let id = net.chain(Box::new(to_stereo(node)));
            nodeid_vec.push(id)
        }
        self.node_ids = Some(nodeid_vec);
        net
    }

    pub fn build(&mut self, shared_midi_state: &SharedMidiState) -> Net {
        println!("initial cc: {:?}", self.initial_cc());
        let arc_vec: Arc<Vec<Net>> =
            Arc::new(self.chain.iter().map(|fx| fx(shared_midi_state)).collect());
        self.connect_node_vec(arc_vec)
    }
    pub fn new(effects_config: Option<&TomlEffectSection>, effect_cc_count: usize) -> Self {
        let registry: HashMap<&str, &EffectDef> =
            EFFECTS.iter().map(|e| (e.params.name.clone(), e)).collect();
        let Some(effects) = effects_config else {
            return FxChainFactory {
                chain: Arc::new(Vec::new()),
                node_ids: None,
                definitions: None,
                fx_names: None,
            };
        };
        let mut fx_names = Vec::new();
        let mut defenitions = Vec::new();
        let mut chain = Vec::new();
        for fx_name in &effects.chain {
            let mut def = registry
                .get(fx_name.as_str())
                .unwrap_or_else(|| panic!("Unknown effect: {}", fx_name));
            let def_override = def.params.clone();
            // ---- Construction values (raw TOML table, exactly what the factory expects) ----
            let mut toml_overrides = Table::new();
            if let Some(eff_cfg) = effects
                .extras
                .get(fx_name.as_str())
                .and_then(|v| v.as_table())
            {
                for (k, v) in eff_cfg {
                    if k != "mapping" {
                        toml_overrides.insert(k.clone(), v.clone());
                    }
                }
            }

            // ---- CC parameter mappings ----
            let user_mappings: Option<&Table> = effects
                .extras
                .get(fx_name.as_str())
                .and_then(|v| v.get("mapping"))
                .and_then(|v| v.as_table());

            if let Some(cc_params) = def.params.cc_params {
                for idx in 0..cc_params.len() {
                    let new = def_override.cc_params.unwrap().get_mut(idx).unwrap();
                    if let Some(m) = user_mappings {
                        if let Some(val) = m.get(fx_name).and_then(|v| v.as_integer()) {
                            new.cc_index = val as usize;
                        }
                    }
                    if let Some(val) = toml_overrides.get(fx_name).and_then(|v| v.as_float()) {
                        match &mut new.default {
                            ParamType::Float(v) => *v = val as f32,
                            ParamType::Int(v) => *v = val as usize,
                            ParamType::ZeroToOneFloat(v) => *v = val.clamp(0.0, 1.0) as f32,
                            ParamType::String(_) => {}
                        }
                    }
                    if let Some(string_val) = toml_overrides.get(fx_name).and_then(|v| v.as_str()) {
                        match &mut new.default {
                            ParamType::Float(v) => {}
                            ParamType::Int(v) => {}
                            ParamType::ZeroToOneFloat(v) => {}
                            ParamType::String(s) => *s = string_val.parse().unwrap(), // ignore, cannot set string from float
                        }
                    }
                }
            }
            let closure = (def.factory)(def_override.clone());
            chain.push(closure);
            defenitions.push(def_override);
            fx_names.push(fx_name.to_string());
        }
        FxChainFactory {
            chain: Arc::new(chain),
            node_ids: None,
            definitions: Some(defenitions),
            fx_names: Some(fx_names),
        }
    }
}
