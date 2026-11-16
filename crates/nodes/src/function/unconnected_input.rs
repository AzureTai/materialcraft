use materialcraft_core::ValueType;

/// Missing operands have their port's type, which can differ from the result.
pub(super) fn value_type(op: &str, name: &str, inferred: u32, other: Option<u32>) -> ValueType {
    if matches!(name, "cond" | "condition") || matches!(op, "and" | "or" | "not") {
        return ValueType::Boolean;
    }

    if matches!(op, "samplelum" | "samplecol" | "atan2" | "dot") {
        return ValueType::Float2;
    }

    if op == "cartesian" || (op == "mulscalar" && name == "scalar") {
        return ValueType::Float;
    }
    let count = op.chars().last().and_then(|c| c.to_digit(10)).unwrap_or(1);
    if op.starts_with("tofloat") {
        return numeric(true, count);
    }

    if op.starts_with("toint") {
        return numeric(false, count);
    }

    if op.contains("swizzle") {
        return numeric(op.starts_with('i'), 1);
    }

    if op.contains("vector") {
        let other_width = other.and_then(ValueType::from_code).map(|kind| match kind {
            ValueType::Integer2 | ValueType::Float2 => 2,
            ValueType::Integer3 | ValueType::Float3 => 3,
            ValueType::Integer4 | ValueType::Float4 => 4,
            _ => 1,
        });
        let width = other_width.map_or(if name == "componentslast" { 1 } else { count.saturating_sub(1) }, |width| count.saturating_sub(width));
        return numeric(op.starts_with('i'), width);
    }
    ValueType::from_code(inferred).unwrap_or(ValueType::Float)
}

fn numeric(integer: bool, count: u32) -> ValueType {
    match (integer, count) {
        (true, 2) => ValueType::Integer2,
        (true, 3) => ValueType::Integer3,
        (true, 4) => ValueType::Integer4,
        (true, _) => ValueType::Integer,
        (false, 2) => ValueType::Float2,
        (false, 3) => ValueType::Float3,
        (false, 4) => ValueType::Float4,
        (false, _) => ValueType::Float,
    }
}
