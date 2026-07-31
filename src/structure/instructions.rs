/// Wasm expression.
#[derive(Default, Clone)]
pub struct Expr {
    /// Sequence of instructions.
    pub instructions: Vec<Instr>,
}

/// Wasm instructions.
#[derive(Clone, Debug)]
pub enum Instr {}