use serde::Deserialize;
use wasp::executor::Val;

#[derive(Deserialize)]
pub struct Manifest {
    pub commands: Vec<Command>
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Command {
    #[serde(rename = "module")]
    Module { 
        filename: String, 
        
        line: u32,
    
        #[serde(default)]
        name: String, 
    },

    #[serde(rename = "register")]
    Register { 
        line: u32, 
        
        #[serde(default)]
        name: String, 
        
        #[serde(rename = "as")]
        as_: String 
    },

    #[serde(rename = "action")]
    Action { action: Action, line: u32 },

    #[serde(rename = "assert_return")]
    AssertReturn { action: Action, expected: Vec<ArgVal>, line: u32 },

    #[serde(rename = "assert_trap")]
    AssertTrap { action: Action, line: u32 },

    #[serde(rename = "assert_exhaustion")]
    AssertExhaustion { action: Action, line: u32 },

    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "invoke")]
    Invoke {
        #[serde(default)]
        module: Option<String>,

        field: String,

        #[serde(default)]
        args: Vec<ArgVal>,
    },

    #[serde(rename = "get")]
    Get {
        #[serde(default)]
        module: Option<String>,

        field: String,
    },
}

#[derive(Deserialize, Clone)]
pub struct ArgVal {
    #[serde(rename = "type")]
    pub ty: String,

    pub value: String,
}

impl TryInto<Val> for ArgVal {
    type Error = ();

    fn try_into(self) -> Result<Val, Self::Error> {
        match self.ty.as_str() {
            "i32" => Ok(Val::I32(self.value.parse::<u32>().unwrap() as i32)),
            "i64" => Ok(Val::I64(self.value.parse::<u64>().unwrap() as i64)),
            "f32" => Ok(Val::F32(f32::from_bits(self.value.parse::<u32>().unwrap()))),
            "f64" => Ok(Val::F64(f64::from_bits(self.value.parse::<u64>().unwrap()))),
            _ => panic!("unsupported arg value type {}", self.ty.as_str())
        }
    }
}
