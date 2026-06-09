//! Main parser entry point for Redcode warrior files.

#[cfg(feature = "std")]
use std::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

#[cfg(not(feature = "std"))]
use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};

use core::str::FromStr;

use corewar_core::{
    AddressingMode, Instruction, Modifier, Opcode, Warrior, CORESIZE, MAXCYCLES, MAXLENGTH,
    MAXPROCESSES, MAXWARRIORS, MINDISTANCE,
};
use nom::{
    branch::alt,
    bytes::complete::{tag_no_case, take_while, take_while1},
    character::complete::{char, digit1, one_of, space0, space1},
    combinator::{all_consuming, map, map_res, opt, recognize},
    multi::{fold_many0, separated_list1},
    sequence::{delimited, pair, preceded, terminated},
    Finish, IResult,
};

use crate::{
    defaults::infer_modifier,
    error::ParseError,
    lexer::{lex, LexedLine, MetadataKind, StatementKind},
};

#[derive(Debug, Clone)]
enum Expr {
    Number(i32),
    Symbol(String),
    UnaryMinus(Box<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
struct OperandExpr {
    mode: AddressingMode,
    expr: Expr,
}

#[derive(Debug, Clone)]
struct PendingInstruction {
    line: usize,
    opcode: Opcode,
    modifier: Option<Modifier>,
    operands: Vec<OperandExpr>,
}

#[derive(Debug, Clone)]
struct SymbolValue {
    line: usize,
    expr: Expr,
}

#[derive(Debug, Clone)]
struct LocatedExpr {
    line: usize,
    expr: Expr,
}

#[derive(Debug, Clone, Copy)]
enum LabelMode {
    Relative { current: usize },
    Absolute,
}

struct EvalContext<'a> {
    equs: &'a BTreeMap<String, SymbolValue>,
    labels: &'a BTreeMap<String, usize>,
}

/// Parse a Redcode source string into a Warrior.
///
/// Supports ICWS'94 standard with pMARS extensions including:
/// - Labels and EQU definitions
/// - All addressing modes (#, $, *, @, <, >, {, })
/// - All modifiers (.A, .B, .AB, .BA, .F, .X, .I)
/// - Predefined constants (CORESIZE, MAXPROCESSES, etc.)
/// - Metadata directives (;name, ;author, ;strategy)
/// - ORG and END directives
pub fn parse_warrior(source: &str) -> Result<Warrior, ParseError> {
    let lines = lex(source)?;
    let mut warrior = Warrior::new("unnamed", Vec::new());
    let mut pending = Vec::new();
    let mut labels = BTreeMap::new();
    let mut equs = predefined_symbols();
    let mut org: Option<LocatedExpr> = None;
    let mut end: Option<LocatedExpr> = None;

    for line in lines {
        match line {
            LexedLine::Metadata(metadata) => match metadata.kind {
                MetadataKind::Name => warrior.name = metadata.value,
                MetadataKind::Author => warrior.author = metadata.value,
                MetadataKind::Strategy => match &mut warrior.strategy {
                    Some(strategy) if !metadata.value.is_empty() => {
                        if !strategy.is_empty() {
                            strategy.push('\n');
                        }
                        strategy.push_str(&metadata.value);
                    }
                    Some(_) => {}
                    None if metadata.value.is_empty() => warrior.strategy = Some(String::new()),
                    None => warrior.strategy = Some(metadata.value),
                },
            },
            LexedLine::Statement(statement) => {
                if matches!(statement.kind, StatementKind::Equ) {
                    let name = statement.label.clone().ok_or_else(|| {
                        ParseError::syntax(statement.line, "EQU requires a symbol name")
                    })?;
                    if labels.contains_key(&name) || equs.contains_key(&name) {
                        return Err(ParseError::DuplicateLabel {
                            label: name,
                            line: statement.line,
                        });
                    }

                    let expr = parse_equ(&statement.statement, statement.line)?;
                    equs.insert(
                        name,
                        SymbolValue {
                            line: statement.line,
                            expr,
                        },
                    );
                    continue;
                }

                if let Some(label) = statement.label.as_ref() {
                    if labels.contains_key(label) || equs.contains_key(label) {
                        return Err(ParseError::DuplicateLabel {
                            label: label.clone(),
                            line: statement.line,
                        });
                    }
                    labels.insert(label.clone(), pending.len());
                }

                match statement.kind {
                    StatementKind::Org => {
                        org = Some(parse_org(&statement.statement, statement.line)?)
                    }
                    StatementKind::End => end = parse_end(&statement.statement, statement.line)?,
                    StatementKind::Instruction => {
                        pending.push(parse_instruction(&statement.statement, statement.line)?);
                    }
                    StatementKind::Equ => unreachable!(),
                }
            }
        }
    }

    if pending.len() > MAXLENGTH {
        return Err(ParseError::TooLong {
            max_length: MAXLENGTH,
        });
    }

    let ctx = EvalContext {
        equs: &equs,
        labels: &labels,
    };

    warrior.instructions = pending
        .iter()
        .enumerate()
        .map(|(index, instruction)| assemble_instruction(instruction, index, &ctx))
        .collect::<Result<Vec<_>, _>>()?;

    warrior.start_offset = match end.as_ref().or(org.as_ref()) {
        Some(located) => {
            let value = evaluate_expression(
                &located.expr,
                &ctx,
                LabelMode::Absolute,
                located.line,
                &mut Vec::new(),
            )?;
            normalize_start_offset(value, warrior.instructions.len(), located.line)?
        }
        None => 0,
    };

    Ok(warrior)
}

fn predefined_symbols() -> BTreeMap<String, SymbolValue> {
    [
        ("CORESIZE", CORESIZE as i32),
        ("MAXPROCESSES", MAXPROCESSES as i32),
        ("MAXCYCLES", MAXCYCLES as i32),
        ("MAXLENGTH", MAXLENGTH as i32),
        ("MINDISTANCE", MINDISTANCE as i32),
        ("WARRIORS", MAXWARRIORS as i32),
    ]
    .into_iter()
    .map(|(name, value)| {
        (
            name.to_string(),
            SymbolValue {
                line: 0,
                expr: Expr::Number(value),
            },
        )
    })
    .collect()
}

fn parse_equ(input: &str, line: usize) -> Result<Expr, ParseError> {
    let (_, expr) = all_consuming(preceded(
        pair(tag_no_case("EQU"), space1),
        terminated(expression, space0),
    ))(input)
    .finish()
    .map_err(|_| ParseError::InvalidDirective {
        directive: input.to_string(),
        line,
    })?;
    Ok(expr)
}

fn parse_org(input: &str, line: usize) -> Result<LocatedExpr, ParseError> {
    let (_, expr) = all_consuming(preceded(
        pair(tag_no_case("ORG"), space1),
        terminated(expression, space0),
    ))(input)
    .finish()
    .map_err(|_| ParseError::InvalidDirective {
        directive: input.to_string(),
        line,
    })?;
    Ok(LocatedExpr { line, expr })
}

fn parse_end(input: &str, line: usize) -> Result<Option<LocatedExpr>, ParseError> {
    let (_, expr) = all_consuming(pair(
        tag_no_case("END"),
        opt(preceded(space1, terminated(expression, space0))),
    ))(input)
    .finish()
    .map_err(|_| ParseError::InvalidDirective {
        directive: input.to_string(),
        line,
    })?;
    Ok(expr.1.map(|expr| LocatedExpr { line, expr }))
}

fn parse_instruction(input: &str, line: usize) -> Result<PendingInstruction, ParseError> {
    let (_, (opcode_name, modifier_name, operands)) = all_consuming(tuple_instruction)(input)
        .finish()
        .map_err(|_| ParseError::syntax(line, format!("could not parse instruction '{input}'")))?;

    let opcode = Opcode::from_str(opcode_name).map_err(|_| ParseError::InvalidOpcode {
        opcode: opcode_name.to_string(),
        line,
    })?;
    let modifier = match modifier_name {
        Some(name) => Some(
            Modifier::from_str(name).map_err(|_| ParseError::InvalidModifier {
                modifier: name.to_string(),
                line,
            })?,
        ),
        None => None,
    };

    Ok(PendingInstruction {
        line,
        opcode,
        modifier,
        operands,
    })
}

fn tuple_instruction(input: &str) -> IResult<&str, (&str, Option<&str>, Vec<OperandExpr>)> {
    let (input, opcode) = identifier(input)?;
    let (input, modifier) = opt(preceded(char('.'), identifier))(input)?;
    let (input, operands) = opt(preceded(
        space1,
        separated_list1(delimited(space0, char(','), space0), operand),
    ))(input)?;
    let (input, _) = space0(input)?;
    Ok((input, (opcode, modifier, operands.unwrap_or_default())))
}

fn assemble_instruction(
    instruction: &PendingInstruction,
    index: usize,
    ctx: &EvalContext<'_>,
) -> Result<Instruction, ParseError> {
    let (a_operand, b_operand) = match instruction.operands.as_slice() {
        [] => (
            OperandExpr {
                mode: AddressingMode::Direct,
                expr: Expr::Number(0),
            },
            OperandExpr {
                mode: AddressingMode::Direct,
                expr: Expr::Number(0),
            },
        ),
        [single] if instruction.opcode == Opcode::DAT => (
            OperandExpr {
                mode: AddressingMode::Immediate,
                expr: Expr::Number(0),
            },
            single.clone(),
        ),
        [single] => (
            single.clone(),
            OperandExpr {
                mode: AddressingMode::Direct,
                expr: Expr::Number(0),
            },
        ),
        [a, b] => (a.clone(), b.clone()),
        _ => return Err(ParseError::syntax(instruction.line, "too many operands")),
    };

    let modifier = instruction
        .modifier
        .unwrap_or_else(|| infer_modifier(instruction.opcode, a_operand.mode, b_operand.mode));
    let mut stack = Vec::new();

    let a_value = evaluate_expression(
        &a_operand.expr,
        ctx,
        LabelMode::Relative { current: index },
        instruction.line,
        &mut stack,
    )?;
    let b_value = evaluate_expression(
        &b_operand.expr,
        ctx,
        LabelMode::Relative { current: index },
        instruction.line,
        &mut stack,
    )?;

    Ok(Instruction::new(
        instruction.opcode,
        modifier,
        a_operand.mode,
        a_value,
        b_operand.mode,
        b_value,
    ))
}

fn normalize_start_offset(
    value: i32,
    instruction_count: usize,
    line: usize,
) -> Result<usize, ParseError> {
    if instruction_count == 0 {
        return Ok(0);
    }

    let value = usize::try_from(value)
        .map_err(|_| ParseError::syntax(line, "start offset must be non-negative"))?;

    if value >= instruction_count {
        return Err(ParseError::syntax(
            line,
            "start offset must point inside the assembled warrior",
        ));
    }

    Ok(value)
}

fn evaluate_expression(
    expr: &Expr,
    ctx: &EvalContext<'_>,
    label_mode: LabelMode,
    line: usize,
    stack: &mut Vec<String>,
) -> Result<i32, ParseError> {
    match expr {
        Expr::Number(value) => Ok(*value),
        Expr::Symbol(name) => {
            if let Some(symbol) = ctx.equs.get(name) {
                if !matches!(symbol.expr, Expr::Number(_)) {
                    if stack.iter().any(|entry| entry == name) {
                        return Err(ParseError::CircularEqu {
                            name: name.clone(),
                            line: symbol.line.max(line),
                        });
                    }
                    stack.push(name.clone());
                }

                let result = evaluate_expression(
                    &symbol.expr,
                    ctx,
                    label_mode,
                    symbol.line.max(line),
                    stack,
                );

                if !matches!(symbol.expr, Expr::Number(_)) {
                    stack.pop();
                }

                return result;
            }

            let target = ctx
                .labels
                .get(name)
                .ok_or_else(|| ParseError::UndefinedLabel {
                    label: name.clone(),
                    line,
                })?;

            Ok(match label_mode {
                LabelMode::Relative { current } => *target as i32 - current as i32,
                LabelMode::Absolute => *target as i32,
            })
        }
        Expr::UnaryMinus(inner) => Ok(-evaluate_expression(inner, ctx, label_mode, line, stack)?),
        Expr::Binary { left, op, right } => {
            let left = evaluate_expression(left, ctx, label_mode, line, stack)?;
            let right = evaluate_expression(right, ctx, label_mode, line, stack)?;
            match op {
                BinaryOp::Add => Ok(left + right),
                BinaryOp::Sub => Ok(left - right),
                BinaryOp::Mul => Ok(left * right),
                BinaryOp::Div => {
                    if right == 0 {
                        Err(ParseError::DivisionByZero { line })
                    } else {
                        Ok(left / right)
                    }
                }
            }
        }
    }
}

fn operand(input: &str) -> IResult<&str, OperandExpr> {
    let (input, _) = space0(input)?;
    let (input, mode) = opt(one_of("#$*@<>}{"))(input)?;
    let (input, expr) = expression(input)?;
    let (input, _) = space0(input)?;
    Ok((
        input,
        OperandExpr {
            mode: mode.map(addressing_mode).unwrap_or(AddressingMode::Direct),
            expr,
        },
    ))
}

fn addressing_mode(mode: char) -> AddressingMode {
    match mode {
        '#' => AddressingMode::Immediate,
        '$' => AddressingMode::Direct,
        '*' => AddressingMode::IndirectA,
        '@' => AddressingMode::IndirectB,
        '<' => AddressingMode::PreDecIndirectB,
        '>' => AddressingMode::PostIncIndirectB,
        '{' => AddressingMode::PreDecIndirectA,
        '}' => AddressingMode::PostIncIndirectA,
        _ => AddressingMode::Direct,
    }
}

fn expression(input: &str) -> IResult<&str, Expr> {
    let (input, init) = term(input)?;
    fold_many0(
        pair(delimited(space0, one_of("+-"), space0), term),
        move || init.clone(),
        |left, (operator, right)| Expr::Binary {
            left: Box::new(left),
            op: if operator == '+' {
                BinaryOp::Add
            } else {
                BinaryOp::Sub
            },
            right: Box::new(right),
        },
    )(input)
}

fn term(input: &str) -> IResult<&str, Expr> {
    let (input, init) = factor(input)?;
    fold_many0(
        pair(delimited(space0, one_of("*/"), space0), factor),
        move || init.clone(),
        |left, (operator, right)| Expr::Binary {
            left: Box::new(left),
            op: if operator == '*' {
                BinaryOp::Mul
            } else {
                BinaryOp::Div
            },
            right: Box::new(right),
        },
    )(input)
}

fn factor(input: &str) -> IResult<&str, Expr> {
    alt((
        map(
            preceded(delimited(space0, char('-'), space0), factor),
            |expr| Expr::UnaryMinus(Box::new(expr)),
        ),
        map(
            preceded(delimited(space0, char('+'), space0), factor),
            |expr| expr,
        ),
        delimited(
            delimited(space0, char('('), space0),
            expression,
            delimited(space0, char(')'), space0),
        ),
        map(number, Expr::Number),
        map(identifier, |name: &str| {
            Expr::Symbol(name.to_ascii_uppercase())
        }),
    ))(input)
}

fn number(input: &str) -> IResult<&str, i32> {
    map_res(delimited(space0, digit1, space0), str::parse::<i32>)(input)
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        take_while1(is_identifier_start),
        take_while(is_identifier_continue),
    ))(input)
}

fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_identifier_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '$')
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_parse_empty() {
        let result = parse_warrior("");
        assert!(result.is_ok());
        assert!(result.unwrap().instructions.is_empty());
    }

    #[test]
    fn parses_imp() {
        let warrior = parse_warrior("; Imp\nMOV 0, 1\n").unwrap();

        assert_eq!(warrior.instructions.len(), 1);
        assert_eq!(
            warrior.instructions[0],
            Instruction::new(
                Opcode::MOV,
                Modifier::I,
                AddressingMode::Direct,
                0,
                AddressingMode::Direct,
                1,
            )
        );
    }

    #[test]
    fn parses_dwarf() {
        let warrior = parse_warrior("; Dwarf\nADD #4, 3\nMOV 2, @2\nJMP -2\nDAT #0, #0\n").unwrap();

        assert_eq!(
            warrior.instructions,
            vec![
                Instruction::new(
                    Opcode::ADD,
                    Modifier::AB,
                    AddressingMode::Immediate,
                    4,
                    AddressingMode::Direct,
                    3,
                ),
                Instruction::new(
                    Opcode::MOV,
                    Modifier::I,
                    AddressingMode::Direct,
                    2,
                    AddressingMode::IndirectB,
                    2,
                ),
                Instruction::new(
                    Opcode::JMP,
                    Modifier::B,
                    AddressingMode::Direct,
                    -2,
                    AddressingMode::Direct,
                    0,
                ),
                Instruction::new(
                    Opcode::DAT,
                    Modifier::F,
                    AddressingMode::Immediate,
                    0,
                    AddressingMode::Immediate,
                    0,
                ),
            ]
        );
    }

    #[test]
    fn resolves_labels_equ_and_start_directives() {
        let warrior = parse_warrior(
            ";name Example\n;author Tester\nstep EQU 4\nstart MOV #step, target\nORG start\ntarget DAT #0, #0\nEND target\n",
        )
        .unwrap();

        assert_eq!(warrior.name, "Example");
        assert_eq!(warrior.author, "Tester");
        assert_eq!(warrior.start_offset, 1);
        assert_eq!(warrior.instructions[0].a_value, 4);
        assert_eq!(warrior.instructions[0].b_value, 1);
    }
}
