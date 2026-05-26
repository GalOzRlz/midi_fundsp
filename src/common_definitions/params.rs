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
}

#[derive(Debug, Clone)]
pub struct CcParam {
    pub default: ParamType,
    pub cc_index: usize,
    pub name: &'static str,
}

#[derive(Debug, Clone)]
pub struct NonCcParam {
    pub value: ParamType,
    pub name: &'static str,
}

#[derive(Clone)]
pub(crate) struct Parameterized {
    pub(crate) name: &'static str,
    pub(crate) cc_params: Option<&'static [CcParam]>,
    pub(crate) non_cc_params: Option<&'static [NonCcParam]>, // use slice if possible
}
impl Parameterized {
    pub fn get_cc_param(&self, name: &str) -> Option<&CcParam> {
        if let Some(vec) = self.cc_params {
            for i in vec.iter() {
                if i.name == name {
                    return Some(i);
                }
            }
        }
        None
    }

    pub fn get_non_cc_param(&self, name: &str) -> Option<&NonCcParam> {
        if let Some(vec) = self.non_cc_params {
            for i in vec.iter() {
                if i.name == name {
                    return Some(i);
                }
            }
        }
        None
    }
}
