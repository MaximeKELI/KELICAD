//! Tiny arithmetic expression evaluator for constraint formulas.

use std::collections::HashMap;

use crate::error::ConstraintError;

/// Evaluate `expr` with named `params`. Supports `+ - * / ( )`, numbers, identifiers.
pub fn eval_formula(expr: &str, params: &HashMap<String, f64>) -> Result<f64, ConstraintError> {
    let tokens = tokenize(expr)?;
    let mut i = 0;
    let v = parse_expr(&tokens, &mut i, params)?;
    if i != tokens.len() {
        return Err(ConstraintError::Formula(format!(
            "trailing tokens in `{expr}`"
        )));
    }
    if !v.is_finite() {
        return Err(ConstraintError::Formula("non-finite result".into()));
    }
    Ok(v)
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, ConstraintError> {
    let mut out = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || c == '.' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let n: f64 = chars[start..i]
                .iter()
                .collect::<String>()
                .parse()
                .map_err(|_| ConstraintError::Formula(format!("bad number near `{c}`")))?;
            out.push(Tok::Num(n));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(chars[start..i].iter().collect()));
            continue;
        }
        match c {
            '+' | '-' | '*' | '/' => {
                out.push(Tok::Op(c));
                i += 1;
            }
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            _ => {
                return Err(ConstraintError::Formula(format!("unexpected `{c}`")));
            }
        }
    }
    Ok(out)
}

fn parse_expr(
    toks: &[Tok],
    i: &mut usize,
    params: &HashMap<String, f64>,
) -> Result<f64, ConstraintError> {
    let mut v = parse_term(toks, i, params)?;
    while let Some(Tok::Op(op @ ('+' | '-'))) = toks.get(*i) {
        *i += 1;
        let r = parse_term(toks, i, params)?;
        v = if *op == '+' { v + r } else { v - r };
    }
    Ok(v)
}

fn parse_term(
    toks: &[Tok],
    i: &mut usize,
    params: &HashMap<String, f64>,
) -> Result<f64, ConstraintError> {
    let mut v = parse_factor(toks, i, params)?;
    while let Some(Tok::Op(op @ ('*' | '/'))) = toks.get(*i) {
        *i += 1;
        let r = parse_factor(toks, i, params)?;
        v = if *op == '*' {
            v * r
        } else if r.abs() < 1e-15 {
            return Err(ConstraintError::Formula("division by zero".into()));
        } else {
            v / r
        };
    }
    Ok(v)
}

fn parse_factor(
    toks: &[Tok],
    i: &mut usize,
    params: &HashMap<String, f64>,
) -> Result<f64, ConstraintError> {
    match toks.get(*i) {
        Some(Tok::Op('-')) => {
            *i += 1;
            Ok(-parse_factor(toks, i, params)?)
        }
        Some(Tok::Op('+')) => {
            *i += 1;
            parse_factor(toks, i, params)
        }
        Some(Tok::Num(n)) => {
            *i += 1;
            Ok(*n)
        }
        Some(Tok::Ident(name)) => {
            *i += 1;
            params
                .get(name)
                .copied()
                .ok_or_else(|| ConstraintError::Formula(format!("unknown param `{name}`")))
        }
        Some(Tok::LParen) => {
            *i += 1;
            let v = parse_expr(toks, i, params)?;
            match toks.get(*i) {
                Some(Tok::RParen) => {
                    *i += 1;
                    Ok(v)
                }
                _ => Err(ConstraintError::Formula("missing `)`".into())),
            }
        }
        _ => Err(ConstraintError::Formula("expected factor".into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eval_basic() {
        let mut p = HashMap::new();
        p.insert("W".into(), 10.0);
        assert!((eval_formula("2*W+1", &p).unwrap() - 21.0).abs() < 1e-9);
        assert!((eval_formula("(W+2)/2", &p).unwrap() - 6.0).abs() < 1e-9);
    }
}
