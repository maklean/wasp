use std::{collections::HashMap, env, fs, path::Path};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct Relocation {
    addend: i8,
    kind: u8,
    offset: u8,
    ordinal: u8,
    
    #[serde(rename = "type")]
    ty: u8
}

/// Raw stencil from the JSON manifest.
#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct RawStencil {
    /// Machine code.
    code: Vec<u8>,

    /// Number of operands in registers.
    #[serde(default)]
    numInRegisterOperand: u8,

    /// Number of floating point parameters (in registers).
    #[serde(default)]
    numOpaqueFloatingParams: u8,

    /// Number of integral parameters (in registers).
    #[serde(default)]
    numOpaqueIntegralParams: u8,

    /// Type of operand in stencil.
    #[serde(default)]
    operandType: u8,

    /// Whether the output should be spilled onto the stack or not.
    #[serde(default)]
    spillOutput: u8,

    /// The relocations of the stencil.
    #[serde(default)]
    relocations: Vec<Relocation>,
}

fn main() {
    println!("cargo::rerun-if-changed=metavar-compiler/build/output/stencils.json");

    let raw_stencils_manifest = fs::read_to_string("metavar-compiler/build/output/stencils.json")
        .expect("Failed to get the stencil JSON manifest not found. Please run the metavar-compiler first.");

    let stencils_manifest: HashMap<String, RawStencil> = serde_json::from_str(&raw_stencils_manifest)
        .expect("Should be able to convert raw stencils manifest to HashMap.");

    println!("cargo:warning={stencils_manifest:?}")
}