use spirv::Word;
use std::fmt;

pub(crate) trait Assemble {
    fn assemble_into(&self, words: &mut Vec<Word>);
}

pub(crate) trait Disassemble {
    fn disassemble(&self) -> String;
}

include!("spirv_disassemble_generated.rs");

macro_rules! define_operands {
    ($( $variant:ident($ty:ty) ),+ $(,)?) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        #[allow(clippy::upper_case_acronyms)]
        pub(crate) enum Operand {
            $( $variant($ty), )+
        }

    };
}

define_operands! {
    ImageOperands(spirv::ImageOperands),
    FPFastMathMode(spirv::FPFastMathMode),
    SelectionControl(spirv::SelectionControl),
    LoopControl(spirv::LoopControl),
    FunctionControl(spirv::FunctionControl),
    MemorySemantics(spirv::MemorySemantics),
    MemoryAccess(spirv::MemoryAccess),
    KernelProfilingInfo(spirv::KernelProfilingInfo),
    RayFlags(spirv::RayFlags),
    FragmentShadingRate(spirv::FragmentShadingRate),
    RawAccessChainOperands(spirv::RawAccessChainOperands),
    SourceLanguage(spirv::SourceLanguage),
    ExecutionModel(spirv::ExecutionModel),
    AddressingModel(spirv::AddressingModel),
    MemoryModel(spirv::MemoryModel),
    ExecutionMode(spirv::ExecutionMode),
    StorageClass(spirv::StorageClass),
    Dim(spirv::Dim),
    SamplerAddressingMode(spirv::SamplerAddressingMode),
    SamplerFilterMode(spirv::SamplerFilterMode),
    ImageFormat(spirv::ImageFormat),
    ImageChannelOrder(spirv::ImageChannelOrder),
    ImageChannelDataType(spirv::ImageChannelDataType),
    FPRoundingMode(spirv::FPRoundingMode),
    FPDenormMode(spirv::FPDenormMode),
    QuantizationModes(spirv::QuantizationModes),
    FPOperationMode(spirv::FPOperationMode),
    OverflowModes(spirv::OverflowModes),
    LinkageType(spirv::LinkageType),
    AccessQualifier(spirv::AccessQualifier),
    HostAccessQualifier(spirv::HostAccessQualifier),
    FunctionParameterAttribute(spirv::FunctionParameterAttribute),
    Decoration(spirv::Decoration),
    BuiltIn(spirv::BuiltIn),
    Scope(spirv::Scope),
    GroupOperation(spirv::GroupOperation),
    KernelEnqueueFlags(spirv::KernelEnqueueFlags),
    Capability(spirv::Capability),
    RayQueryIntersection(spirv::RayQueryIntersection),
    RayQueryCommittedIntersectionType(spirv::RayQueryCommittedIntersectionType),
    RayQueryCandidateIntersectionType(spirv::RayQueryCandidateIntersectionType),
    PackedVectorFormat(spirv::PackedVectorFormat),
    CooperativeMatrixOperands(spirv::CooperativeMatrixOperands),
    CooperativeMatrixLayout(spirv::CooperativeMatrixLayout),
    CooperativeMatrixUse(spirv::CooperativeMatrixUse),
    CooperativeMatrixReduce(spirv::CooperativeMatrixReduce),
    TensorClampMode(spirv::TensorClampMode),
    TensorAddressingOperands(spirv::TensorAddressingOperands),
    InitializationModeQualifier(spirv::InitializationModeQualifier),
    LoadCacheControl(spirv::LoadCacheControl),
    StoreCacheControl(spirv::StoreCacheControl),
    NamedMaximumNumberOfRegisters(spirv::NamedMaximumNumberOfRegisters),
    MatrixMultiplyAccumulateOperands(spirv::MatrixMultiplyAccumulateOperands),
    FPEncoding(spirv::FPEncoding),
    CooperativeVectorMatrixLayout(spirv::CooperativeVectorMatrixLayout),
    ComponentType(spirv::ComponentType),
    TensorOperands(spirv::TensorOperands),
    IdMemorySemantics(Word),
    IdScope(Word),
    IdRef(Word),
    LiteralBit32(u32),
    LiteralBit64(u64),
    LiteralExtInstInteger(u32),
    LiteralSpecConstantOpInteger(spirv::Op),
    LiteralString(String),
}

include!("spirv_operand_display_generated.rs");

impl From<u32> for Operand {
    fn from(value: u32) -> Self {
        Self::LiteralBit32(value)
    }
}

impl From<u64> for Operand {
    fn from(value: u64) -> Self {
        Self::LiteralBit64(value)
    }
}

impl From<spirv::Op> for Operand {
    fn from(value: spirv::Op) -> Self {
        Self::LiteralSpecConstantOpInteger(value)
    }
}

impl From<String> for Operand {
    fn from(value: String) -> Self {
        Self::LiteralString(value)
    }
}

impl From<&str> for Operand {
    fn from(value: &str) -> Self {
        Self::LiteralString(value.to_owned())
    }
}

impl Disassemble for Operand {
    fn disassemble(&self) -> String {
        match self {
            Self::IdMemorySemantics(value) | Self::IdScope(value) | Self::IdRef(value) => {
                format!("%{value}")
            }
            Self::ImageOperands(value) => value.disassemble(),
            Self::FPFastMathMode(value) => value.disassemble(),
            Self::SelectionControl(value) => value.disassemble(),
            Self::LoopControl(value) => value.disassemble(),
            Self::FunctionControl(value) => value.disassemble(),
            Self::MemorySemantics(value) => value.disassemble(),
            Self::MemoryAccess(value) => value.disassemble(),
            Self::KernelProfilingInfo(value) => value.disassemble(),
            _ => self.to_string(),
        }
    }
}

impl Assemble for Operand {
    fn assemble_into(&self, words: &mut Vec<Word>) {
        match self {
            Self::ImageOperands(value) => words.push(value.bits()),
            Self::FPFastMathMode(value) => words.push(value.bits()),
            Self::SelectionControl(value) => words.push(value.bits()),
            Self::LoopControl(value) => words.push(value.bits()),
            Self::FunctionControl(value) => words.push(value.bits()),
            Self::MemorySemantics(value) => words.push(value.bits()),
            Self::MemoryAccess(value) => words.push(value.bits()),
            Self::KernelProfilingInfo(value) => words.push(value.bits()),
            Self::CooperativeMatrixOperands(value) => words.push(value.bits()),
            Self::RayFlags(value) => words.push(value.bits()),
            Self::FragmentShadingRate(value) => words.push(value.bits()),
            Self::CooperativeMatrixReduce(value) => words.push(value.bits()),
            Self::TensorAddressingOperands(value) => words.push(value.bits()),
            Self::TensorOperands(value) => words.push(value.bits()),
            Self::RawAccessChainOperands(value) => words.push(value.bits()),
            Self::MatrixMultiplyAccumulateOperands(value) => words.push(value.bits()),
            Self::SourceLanguage(value) => words.push(*value as Word),
            Self::ExecutionModel(value) => words.push(*value as Word),
            Self::AddressingModel(value) => words.push(*value as Word),
            Self::MemoryModel(value) => words.push(*value as Word),
            Self::ExecutionMode(value) => words.push(*value as Word),
            Self::StorageClass(value) => words.push(*value as Word),
            Self::Dim(value) => words.push(*value as Word),
            Self::SamplerAddressingMode(value) => words.push(*value as Word),
            Self::SamplerFilterMode(value) => words.push(*value as Word),
            Self::ImageFormat(value) => words.push(*value as Word),
            Self::ImageChannelOrder(value) => words.push(*value as Word),
            Self::ImageChannelDataType(value) => words.push(*value as Word),
            Self::FPRoundingMode(value) => words.push(*value as Word),
            Self::FPDenormMode(value) => words.push(*value as Word),
            Self::QuantizationModes(value) => words.push(*value as Word),
            Self::FPOperationMode(value) => words.push(*value as Word),
            Self::OverflowModes(value) => words.push(*value as Word),
            Self::LinkageType(value) => words.push(*value as Word),
            Self::AccessQualifier(value) => words.push(*value as Word),
            Self::HostAccessQualifier(value) => words.push(*value as Word),
            Self::FunctionParameterAttribute(value) => words.push(*value as Word),
            Self::Decoration(value) => words.push(*value as Word),
            Self::BuiltIn(value) => words.push(*value as Word),
            Self::Scope(value) => words.push(*value as Word),
            Self::GroupOperation(value) => words.push(*value as Word),
            Self::KernelEnqueueFlags(value) => words.push(*value as Word),
            Self::Capability(value) => words.push(*value as Word),
            Self::RayQueryIntersection(value) => words.push(*value as Word),
            Self::RayQueryCommittedIntersectionType(value) => words.push(*value as Word),
            Self::RayQueryCandidateIntersectionType(value) => words.push(*value as Word),
            Self::PackedVectorFormat(value) => words.push(*value as Word),
            Self::CooperativeMatrixLayout(value) => words.push(*value as Word),
            Self::CooperativeMatrixUse(value) => words.push(*value as Word),
            Self::TensorClampMode(value) => words.push(*value as Word),
            Self::InitializationModeQualifier(value) => words.push(*value as Word),
            Self::LoadCacheControl(value) => words.push(*value as Word),
            Self::StoreCacheControl(value) => words.push(*value as Word),
            Self::NamedMaximumNumberOfRegisters(value) => words.push(*value as Word),
            Self::FPEncoding(value) => words.push(*value as Word),
            Self::CooperativeVectorMatrixLayout(value) => words.push(*value as Word),
            Self::ComponentType(value) => words.push(*value as Word),
            Self::IdMemorySemantics(value)
            | Self::IdScope(value)
            | Self::IdRef(value)
            | Self::LiteralBit32(value)
            | Self::LiteralExtInstInteger(value) => words.push(*value),
            Self::LiteralBit64(value) => {
                words.extend([*value as Word, (*value >> 32) as Word]);
            }
            Self::LiteralSpecConstantOpInteger(value) => words.push(*value as Word),
            Self::LiteralString(value) => assemble_string(value, words),
        }
    }
}

fn assemble_string(value: &str, words: &mut Vec<Word>) {
    let chunks = value.as_bytes().chunks_exact(4);
    let remainder = chunks.remainder();
    let mut last = [0_u8; 4];
    last[..remainder.len()].copy_from_slice(remainder);
    words.extend(
        chunks.map(|chunk| Word::from_le_bytes(chunk.try_into().expect("four-byte chunk"))),
    );
    words.push(Word::from_le_bytes(last));
}
