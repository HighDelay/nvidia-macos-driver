use super::decoder::Decoder;
use super::error::Error as DecodeError;
use super::grammar::{
    InstructionGrammar, OperandKind as GOpKind, OperandQuantifier as GOpCount, INSTRUCTION_TABLE,
};
use super::type_tracker::{ScalarType, TypeTracker};
use crate::spirv_module::{Instruction, ModuleHeader, Operand};
use spirv::Word;
use std::{error, fmt};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub(crate) enum Error {
    Complete,
    HeaderIncomplete(DecodeError),
    HeaderIncorrect,
    EndiannessUnsupported,
    WordCountZero(usize, usize),
    OpcodeUnknown(usize, usize, u16),
    OperandExpected(usize, usize),
    OperandExceeded(usize, usize),
    OperandDecode(DecodeError),
    TypeUnsupported(usize, usize),
    SpecConstantOpIntegerIncorrect(usize, usize),
}

impl From<DecodeError> for Error {
    fn from(error: DecodeError) -> Self {
        Self::OperandDecode(error)
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Complete => write!(formatter, "completed parsing"),
            Self::HeaderIncomplete(error) => write!(formatter, "incomplete module header: {error}"),
            Self::HeaderIncorrect => write!(formatter, "incorrect module header"),
            Self::EndiannessUnsupported => write!(formatter, "unsupported endianness"),
            Self::WordCountZero(offset, index) => {
                write!(
                    formatter,
                    "zero word count at offset {offset}, instruction {index}"
                )
            }
            Self::OpcodeUnknown(offset, index, opcode) => write!(
                formatter,
                "unknown opcode {opcode} at offset {offset}, instruction {index}"
            ),
            Self::OperandExpected(offset, index) => write!(
                formatter,
                "expected operand at offset {offset}, instruction {index}"
            ),
            Self::OperandExceeded(offset, index) => write!(
                formatter,
                "extra operand at offset {offset}, instruction {index}"
            ),
            Self::OperandDecode(error) => write!(formatter, "operand decoding error: {error}"),
            Self::TypeUnsupported(offset, index) => write!(
                formatter,
                "unsupported literal type at offset {offset}, instruction {index}"
            ),
            Self::SpecConstantOpIntegerIncorrect(offset, index) => write!(
                formatter,
                "invalid spec-constant opcode at offset {offset}, instruction {index}"
            ),
        }
    }
}

pub(crate) struct ParsedModule {
    pub(crate) header: ModuleHeader,
    pub(crate) instructions: Vec<Instruction>,
}

pub(crate) fn parse_bytes(bytes: &[u8]) -> Result<ParsedModule> {
    Parser::new(bytes).parse()
}

struct Parser<'a> {
    decoder: Decoder<'a>,
    types: TypeTracker,
    instruction_index: usize,
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            decoder: Decoder::new(bytes),
            types: TypeTracker::default(),
            instruction_index: 0,
        }
    }

    fn parse(mut self) -> Result<ParsedModule> {
        let header = self.parse_header()?;
        let mut instructions = Vec::new();
        loop {
            match self.parse_instruction() {
                Ok(instruction) => {
                    self.types.track(&instruction);
                    instructions.push(instruction);
                }
                Err(Error::Complete) => break,
                Err(error) => return Err(error),
            }
        }
        Ok(ParsedModule {
            header,
            instructions,
        })
    }

    fn parse_header(&mut self) -> Result<ModuleHeader> {
        let words = self.decoder.words(5).map_err(Error::HeaderIncomplete)?;
        if words[0] != spirv::MAGIC_NUMBER {
            return Err(if words[0] == spirv::MAGIC_NUMBER.swap_bytes() {
                Error::EndiannessUnsupported
            } else {
                Error::HeaderIncorrect
            });
        }
        let mut header = ModuleHeader::new(words[3]);
        header.version = words[1];
        Ok(header)
    }

    fn parse_instruction(&mut self) -> Result<Instruction> {
        self.instruction_index += 1;
        let first = match self.decoder.word() {
            Ok(word) => word,
            Err(_) => return Err(Error::Complete),
        };
        let word_count = (first >> 16) as u16;
        let raw_opcode = first as u16;
        if word_count == 0 {
            return Err(Error::WordCountZero(
                self.decoder.offset() - 4,
                self.instruction_index,
            ));
        }
        let grammar = INSTRUCTION_TABLE
            .lookup_opcode(u32::from(raw_opcode))
            .ok_or(Error::OpcodeUnknown(
                self.decoder.offset() - 4,
                self.instruction_index,
                raw_opcode,
            ))?;
        self.decoder.set_limit(usize::from(word_count - 1));
        let instruction = self.parse_operands(grammar)?;
        if !self.decoder.limit_reached() {
            return Err(Error::OperandExceeded(
                self.decoder.offset(),
                self.instruction_index,
            ));
        }
        self.decoder.clear_limit();
        Ok(instruction)
    }

    fn parse_literal(&mut self, type_id: Word) -> Result<Operand> {
        match self.types.resolve(type_id) {
            Some(ScalarType::Integer(8 | 16 | 32) | ScalarType::Float(16 | 32)) => {
                Ok(Operand::LiteralBit32(self.decoder.bit32()?))
            }
            Some(ScalarType::Integer(64) | ScalarType::Float(64)) => {
                Ok(Operand::LiteralBit64(self.decoder.bit64()?))
            }
            Some(_) => Err(Error::TypeUnsupported(
                self.decoder.offset(),
                self.instruction_index,
            )),
            None => Ok(Operand::LiteralBit32(self.decoder.bit32()?)),
        }
    }

    fn parse_spec_constant_op(&mut self) -> Result<Vec<Operand>> {
        let raw_opcode = self.decoder.bit32()?;
        let grammar = INSTRUCTION_TABLE.lookup_opcode(raw_opcode).ok_or(
            Error::SpecConstantOpIntegerIncorrect(self.decoder.offset(), self.instruction_index),
        )?;
        let mut operands = vec![Operand::LiteralSpecConstantOpInteger(grammar.opcode)];
        for logical in grammar.operands {
            if !matches!(logical.kind, GOpKind::IdResultType | GOpKind::IdResult) {
                operands.append(&mut self.parse_operand(logical.kind)?);
            }
        }
        Ok(operands)
    }

    fn parse_operands(&mut self, grammar: &'static InstructionGrammar) -> Result<Instruction> {
        let mut result_type = None;
        let mut result_id = None;
        let mut operands = Vec::new();
        let mut index = 0;
        while index < grammar.operands.len() {
            let logical = grammar.operands[index];
            if !self.decoder.limit_reached() {
                match logical.kind {
                    GOpKind::IdResultType => result_type = Some(self.decoder.id()?),
                    GOpKind::IdResult => result_id = Some(self.decoder.id()?),
                    GOpKind::LiteralContextDependentNumber => {
                        let id = result_type.expect("result type precedes context literal");
                        operands.push(self.parse_literal(id)?);
                    }
                    GOpKind::PairLiteralIntegerIdRef => {
                        let selector = match operands.first() {
                            Some(Operand::IdRef(id)) => *id,
                            _ => unreachable!("OpSwitch selector is an IdRef"),
                        };
                        operands.push(self.parse_literal(selector)?);
                        operands.push(Operand::IdRef(self.decoder.id()?));
                    }
                    GOpKind::LiteralSpecConstantOpInteger => {
                        operands.append(&mut self.parse_spec_constant_op()?);
                    }
                    _ => operands.append(&mut self.parse_operand(logical.kind)?),
                }
                match logical.quantifier {
                    GOpCount::One | GOpCount::ZeroOrOne => index += 1,
                    GOpCount::ZeroOrMore => {}
                }
            } else {
                match logical.quantifier {
                    GOpCount::One => {
                        return Err(Error::OperandExpected(
                            self.decoder.offset(),
                            self.instruction_index,
                        ));
                    }
                    GOpCount::ZeroOrOne | GOpCount::ZeroOrMore => break,
                }
            }
        }
        Ok(Instruction::new(
            grammar.opcode,
            result_type,
            result_id,
            operands,
        ))
    }
}

include!("parse_generated.rs");
