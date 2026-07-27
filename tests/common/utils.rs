use wasp::executor::Val;

/// Returns whether the two collection of values are the same.
pub fn vals_match(a: &[Val], b: &[Val]) -> bool {
    if a.len() != b.len() { return false; }

    a.iter().zip(b).all(|(va, vb)| match (va, vb) {
        (Val::I32(va), Val::I32(vb)) => va == vb,
        (Val::I64(va), Val::I64(vb)) => va == vb,
        (Val::F32(va), Val::F32(vb)) => va.to_bits() == vb.to_bits(),
        (Val::F64(va), Val::F64(vb)) => va.to_bits() == vb.to_bits(),
        _ => false,
    })
}