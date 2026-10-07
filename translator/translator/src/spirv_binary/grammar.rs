#![allow(dead_code)]

use crate::spirv_operand::Operand;
use spirv::Op;
use std::marker::PhantomData;

#[derive(Debug)]
pub(super) struct InstructionGrammar {
    pub(super) opcode: Op,
    pub(super) capabilities: &'static [spirv::Capability],
    pub(super) extensions: &'static [&'static str],
    pub(super) operands: &'static [LogicalOperand],
}

#[derive(Clone, Copy, Debug)]
pub(super) struct LogicalOperand {
    pub(super) kind: OperandKind,
    pub(super) quantifier: OperandQuantifier,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum OperandQuantifier {
    One,
    ZeroOrOne,
    ZeroOrMore,
}

macro_rules! inst {
    ($op:ident, [$($cap:ident),*], [$($ext:expr),*], [$(($kind:ident, $quant:ident)),*]) => {
        InstructionGrammar {
            opcode: Op::$op,
            capabilities: &[$(spirv::Capability::$cap),*],
            extensions: &[$($ext),*],
            operands: &[
                $(LogicalOperand {
                    kind: OperandKind::$kind,
                    quantifier: OperandQuantifier::$quant,
                }),*
            ],
        }
    };
}

pub(super) struct InstructionTable(&'static [InstructionGrammar], PhantomData<Op>);

impl InstructionTable {
    pub(super) fn lookup_opcode(&self, opcode: u32) -> Option<&'static InstructionGrammar> {
        self.0
            .iter()
            .find(|instruction| instruction.opcode as u32 == opcode)
    }
}

pub(crate) fn instruction_result_shape(opcode: Op) -> Option<(bool, bool)> {
    let grammar = INSTRUCTION_TABLE.lookup_opcode(opcode as u32)?;
    let has_result_type = grammar
        .operands
        .iter()
        .any(|operand| operand.kind == OperandKind::IdResultType);
    let has_result_id = grammar
        .operands
        .iter()
        .any(|operand| operand.kind == OperandKind::IdResult);
    Some((has_result_type, has_result_id))
}

pub(crate) fn instruction_declaration_requirements(
    opcode: Op,
) -> Option<(&'static [spirv::Capability], &'static [&'static str])> {
    let grammar = INSTRUCTION_TABLE.lookup_opcode(opcode as u32)?;
    Some((grammar.capabilities, grammar.extensions))
}

include!("enumerant_requirements_generated.rs");

pub(crate) fn operand_declaration_requirements(
    operand: &Operand,
) -> impl Iterator<Item = &'static EnumerantRequirement> {
    operand_declaration_requirements_generated(operand)
}

#[derive(Clone, Copy)]
enum OperandShape {
    IdRef,
    IdScope,
    LiteralBit32,
    LiteralString,
    BuiltIn,
    FunctionParameterAttribute,
    FPRoundingMode,
    FPFastMathMode,
    LinkageType,
    FPDenormMode,
    FPOperationMode,
    AccessQualifier,
    HostAccessQualifier,
    InitializationModeQualifier,
    LoadCacheControl,
    StoreCacheControl,
    NamedMaximumNumberOfRegisters,
}

fn operand_has_shape(operand: &Operand, shape: OperandShape) -> bool {
    matches!(
        (operand, shape),
        (Operand::IdRef(_), OperandShape::IdRef)
            | (Operand::IdScope(_), OperandShape::IdScope)
            | (Operand::LiteralBit32(_), OperandShape::LiteralBit32)
            | (Operand::LiteralString(_), OperandShape::LiteralString)
            | (Operand::BuiltIn(_), OperandShape::BuiltIn)
            | (
                Operand::FunctionParameterAttribute(_),
                OperandShape::FunctionParameterAttribute
            )
            | (Operand::FPRoundingMode(_), OperandShape::FPRoundingMode)
            | (Operand::FPFastMathMode(_), OperandShape::FPFastMathMode)
            | (Operand::LinkageType(_), OperandShape::LinkageType)
            | (Operand::FPDenormMode(_), OperandShape::FPDenormMode)
            | (Operand::FPOperationMode(_), OperandShape::FPOperationMode)
            | (Operand::AccessQualifier(_), OperandShape::AccessQualifier)
            | (
                Operand::HostAccessQualifier(_),
                OperandShape::HostAccessQualifier
            )
            | (
                Operand::InitializationModeQualifier(_),
                OperandShape::InitializationModeQualifier
            )
            | (Operand::LoadCacheControl(_), OperandShape::LoadCacheControl)
            | (
                Operand::StoreCacheControl(_),
                OperandShape::StoreCacheControl
            )
            | (
                Operand::NamedMaximumNumberOfRegisters(_),
                OperandShape::NamedMaximumNumberOfRegisters
            )
    )
}

fn shape_prefix(operands: &[Operand], shapes: &[OperandShape]) -> Option<usize> {
    (operands.len() >= shapes.len()
        && operands
            .iter()
            .zip(shapes)
            .all(|(operand, shape)| operand_has_shape(operand, *shape)))
    .then_some(shapes.len())
}

fn image_operand_tail(operands: &[Operand], mask: spirv::ImageOperands) -> Option<usize> {
    let mut shapes = Vec::new();
    for (flag, shape) in [
        (spirv::ImageOperands::BIAS, OperandShape::IdRef),
        (spirv::ImageOperands::LOD, OperandShape::IdRef),
    ] {
        if mask.contains(flag) {
            shapes.push(shape);
        }
    }
    if mask.contains(spirv::ImageOperands::GRAD) {
        shapes.extend([OperandShape::IdRef, OperandShape::IdRef]);
    }
    for flag in [
        spirv::ImageOperands::CONST_OFFSET,
        spirv::ImageOperands::OFFSET,
        spirv::ImageOperands::CONST_OFFSETS,
        spirv::ImageOperands::SAMPLE,
        spirv::ImageOperands::MIN_LOD,
    ] {
        if mask.contains(flag) {
            shapes.push(OperandShape::IdRef);
        }
    }
    for flag in [
        spirv::ImageOperands::MAKE_TEXEL_AVAILABLE,
        spirv::ImageOperands::MAKE_TEXEL_VISIBLE,
    ] {
        if mask.contains(flag) {
            shapes.push(OperandShape::IdScope);
        }
    }
    if mask.contains(spirv::ImageOperands::OFFSETS) {
        shapes.push(OperandShape::IdRef);
    }
    shape_prefix(operands, &shapes)
}

fn loop_control_tail(operands: &[Operand], mask: spirv::LoopControl) -> Option<usize> {
    let flags = [
        spirv::LoopControl::DEPENDENCY_LENGTH,
        spirv::LoopControl::MIN_ITERATIONS,
        spirv::LoopControl::MAX_ITERATIONS,
        spirv::LoopControl::ITERATION_MULTIPLE,
        spirv::LoopControl::PEEL_COUNT,
        spirv::LoopControl::PARTIAL_COUNT,
        spirv::LoopControl::INITIATION_INTERVAL_ALTERA,
        spirv::LoopControl::MAX_CONCURRENCY_ALTERA,
        spirv::LoopControl::DEPENDENCY_ARRAY_ALTERA,
        spirv::LoopControl::PIPELINE_ENABLE_ALTERA,
        spirv::LoopControl::LOOP_COALESCE_ALTERA,
        spirv::LoopControl::MAX_INTERLEAVING_ALTERA,
        spirv::LoopControl::SPECULATED_ITERATIONS_ALTERA,
        spirv::LoopControl::LOOP_COUNT_ALTERA,
        spirv::LoopControl::MAX_REINVOCATION_DELAY_ALTERA,
    ];
    let count = flags.iter().filter(|flag| mask.contains(**flag)).count();
    shape_prefix(operands, &vec![OperandShape::LiteralBit32; count])
}

fn memory_access_tail(operands: &[Operand], mask: spirv::MemoryAccess) -> Option<usize> {
    let mut shapes = Vec::new();
    if mask.contains(spirv::MemoryAccess::ALIGNED) {
        shapes.push(OperandShape::LiteralBit32);
    }
    if mask.contains(spirv::MemoryAccess::MAKE_POINTER_AVAILABLE) {
        shapes.push(OperandShape::IdScope);
    }
    if mask.contains(spirv::MemoryAccess::MAKE_POINTER_VISIBLE) {
        shapes.push(OperandShape::IdScope);
    }
    if mask.contains(spirv::MemoryAccess::ALIAS_SCOPE_INTEL_MASK) {
        shapes.push(OperandShape::IdRef);
    }
    if mask.contains(spirv::MemoryAccess::NO_ALIAS_INTEL_MASK) {
        shapes.push(OperandShape::IdRef);
    }
    shape_prefix(operands, &shapes)
}

fn execution_mode_tail(operands: &[Operand], mode: spirv::ExecutionMode) -> Option<usize> {
    use spirv::ExecutionMode as E;
    use OperandShape as S;
    let shapes: &[S] = match mode {
        E::Invocations
        | E::OutputVertices
        | E::VecTypeHint
        | E::SubgroupSize
        | E::SubgroupsPerWorkgroup
        | E::DenormPreserve
        | E::DenormFlushToZero
        | E::SignedZeroInfNanPreserve
        | E::RoundingModeRTE
        | E::RoundingModeRTZ
        | E::OutputPrimitivesEXT
        | E::SharedLocalMemorySizeINTEL
        | E::RoundingModeRTPINTEL
        | E::RoundingModeRTNINTEL
        | E::FloatingPointModeALTINTEL
        | E::FloatingPointModeIEEEINTEL
        | E::MaxWorkDimINTEL
        | E::NumSIMDWorkitemsINTEL
        | E::SchedulerTargetFmaxMhzINTEL
        | E::StreamingInterfaceINTEL
        | E::RegisterMapInterfaceINTEL
        | E::NamedBarrierCountINTEL
        | E::MaximumRegistersINTEL => &[S::LiteralBit32],
        E::LocalSize | E::LocalSizeHint | E::TileShadingRateQCOM | E::MaxWorkgroupSizeINTEL => {
            &[S::LiteralBit32, S::LiteralBit32, S::LiteralBit32]
        }
        E::SubgroupsPerWorkgroupId
        | E::IsApiEntryAMDX
        | E::MaxNodeRecursionAMDX
        | E::ShaderIndexAMDX
        | E::MaximumRegistersIdINTEL => &[S::IdRef],
        E::LocalSizeId
        | E::LocalSizeHintId
        | E::StaticNumWorkgroupsAMDX
        | E::MaxNumWorkgroupsAMDX => &[S::IdRef, S::IdRef, S::IdRef],
        E::SharesInputWithAMDX | E::FPFastMathDefault => &[S::IdRef, S::IdRef],
        E::NamedMaximumRegistersINTEL => &[S::NamedMaximumNumberOfRegisters],
        _ => &[],
    };
    shape_prefix(operands, shapes)
}

fn decoration_tail(operands: &[Operand], decoration: spirv::Decoration) -> Option<usize> {
    use spirv::Decoration as D;
    use OperandShape as S;
    let shapes: &[S] = match decoration {
        D::SpecId
        | D::ArrayStride
        | D::MatrixStride
        | D::Stream
        | D::Location
        | D::Component
        | D::Index
        | D::Binding
        | D::DescriptorSet
        | D::Offset
        | D::XfbBuffer
        | D::XfbStride
        | D::InputAttachmentIndex
        | D::Alignment
        | D::MaxByteOffset
        | D::SecondaryViewportRelativeNV
        | D::MemberOffsetNV
        | D::BankNV
        | D::SIMTCallINTEL
        | D::FuncParamIOKindINTEL
        | D::GlobalVariableOffsetINTEL
        | D::NumbanksALTERA
        | D::BankwidthALTERA
        | D::MaxPrivateCopiesALTERA
        | D::MaxReplicatesALTERA
        | D::BankBitsALTERA
        | D::ForcePow2DepthALTERA
        | D::StridesizeALTERA
        | D::WordsizeALTERA
        | D::CacheSizeALTERA
        | D::PrefetchALTERA
        | D::InitiationIntervalALTERA
        | D::MaxConcurrencyALTERA
        | D::PipelineEnableALTERA
        | D::BufferLocationALTERA
        | D::IOPipeStorageALTERA
        | D::FPMaxErrorDecorationINTEL
        | D::LatencyControlLabelALTERA
        | D::MMHostInterfaceAddressWidthALTERA
        | D::MMHostInterfaceDataWidthALTERA
        | D::MMHostInterfaceLatencyALTERA
        | D::MMHostInterfaceMaxBurstALTERA
        | D::MMHostInterfaceWaitRequestALTERA
        | D::ImplementInRegisterMapALTERA => &[S::LiteralBit32],
        D::BuiltIn => &[S::BuiltIn],
        D::UniformId => &[S::IdScope],
        D::FuncParamAttr => &[S::FunctionParameterAttribute],
        D::FPRoundingMode => &[S::FPRoundingMode],
        D::FPFastMathMode => &[S::FPFastMathMode],
        D::LinkageAttributes => &[S::LiteralString, S::LinkageType],
        D::AlignmentId
        | D::MaxByteOffsetId
        | D::NodeSharesPayloadLimitsWithAMDX
        | D::NodeMaxPayloadsAMDX
        | D::PayloadNodeNameAMDX
        | D::PayloadNodeBaseIndexAMDX
        | D::PayloadNodeArraySizeAMDX
        | D::ArrayStrideIdEXT
        | D::OffsetIdEXT
        | D::CounterBuffer
        | D::AliasScopeINTEL
        | D::NoAliasINTEL
        | D::ConditionalINTEL => &[S::IdRef],
        D::ClobberINTEL | D::UserSemantic | D::UserTypeGOOGLE | D::MemoryALTERA => {
            &[S::LiteralString]
        }
        D::FunctionRoundingModeINTEL => &[S::LiteralBit32, S::FPRoundingMode],
        D::FunctionDenormModeINTEL => &[S::LiteralBit32, S::FPDenormMode],
        D::MergeALTERA => &[S::LiteralString, S::LiteralString],
        D::MathOpDSPModeALTERA => &[S::LiteralBit32, S::LiteralBit32],
        D::FunctionFloatingPointModeINTEL => &[S::LiteralBit32, S::FPOperationMode],
        D::LatencyControlConstraintALTERA => &[S::LiteralBit32, S::LiteralBit32, S::LiteralBit32],
        D::MMHostInterfaceReadWriteModeALTERA => &[S::AccessQualifier],
        D::HostAccessINTEL => &[S::HostAccessQualifier, S::LiteralString],
        D::InitModeALTERA => &[S::InitializationModeQualifier],
        D::CacheControlLoadINTEL => &[S::LiteralBit32, S::LoadCacheControl],
        D::CacheControlStoreINTEL => &[S::LiteralBit32, S::StoreCacheControl],
        _ => &[],
    };
    shape_prefix(operands, shapes)
}

fn tensor_addressing_tail(
    operands: &[Operand],
    mask: spirv::TensorAddressingOperands,
) -> Option<usize> {
    let count = [
        spirv::TensorAddressingOperands::TENSOR_VIEW,
        spirv::TensorAddressingOperands::DECODE_FUNC,
    ]
    .iter()
    .filter(|flag| mask.contains(**flag))
    .count();
    (operands.len() >= count
        && operands
            .iter()
            .take(count)
            .all(|operand| matches!(operand, Operand::IdRef(_))))
    .then_some(count)
}

fn tensor_tail(operands: &[Operand], mask: spirv::TensorOperands) -> Option<usize> {
    let count = [
        spirv::TensorOperands::OUT_OF_BOUNDS_VALUE_ARM,
        spirv::TensorOperands::MAKE_ELEMENT_AVAILABLE_ARM,
        spirv::TensorOperands::MAKE_ELEMENT_VISIBLE_ARM,
    ]
    .iter()
    .filter(|flag| mask.contains(**flag))
    .count();
    (operands.len() >= count
        && operands
            .iter()
            .take(count)
            .all(|operand| matches!(operand, Operand::IdRef(_))))
    .then_some(count)
}

fn operand_mask_bits_are_known(operand: &Operand) -> bool {
    macro_rules! known_bits {
        ($value:expr, $mask:ty) => {
            $value.bits() & !<$mask>::all().bits() == 0
        };
    }

    match operand {
        Operand::ImageOperands(value) => known_bits!(value, spirv::ImageOperands),
        Operand::FPFastMathMode(value) => known_bits!(value, spirv::FPFastMathMode),
        Operand::SelectionControl(value) => known_bits!(value, spirv::SelectionControl),
        Operand::LoopControl(value) => known_bits!(value, spirv::LoopControl),
        Operand::FunctionControl(value) => known_bits!(value, spirv::FunctionControl),
        Operand::MemorySemantics(value) => known_bits!(value, spirv::MemorySemantics),
        Operand::MemoryAccess(value) => known_bits!(value, spirv::MemoryAccess),
        Operand::KernelProfilingInfo(value) => known_bits!(value, spirv::KernelProfilingInfo),
        Operand::RayFlags(value) => known_bits!(value, spirv::RayFlags),
        Operand::FragmentShadingRate(value) => known_bits!(value, spirv::FragmentShadingRate),
        Operand::RawAccessChainOperands(value) => {
            known_bits!(value, spirv::RawAccessChainOperands)
        }
        Operand::CooperativeMatrixOperands(value) => {
            known_bits!(value, spirv::CooperativeMatrixOperands)
        }
        Operand::CooperativeMatrixReduce(value) => {
            known_bits!(value, spirv::CooperativeMatrixReduce)
        }
        Operand::TensorAddressingOperands(value) => {
            known_bits!(value, spirv::TensorAddressingOperands)
        }
        Operand::MatrixMultiplyAccumulateOperands(value) => {
            known_bits!(value, spirv::MatrixMultiplyAccumulateOperands)
        }
        Operand::TensorOperands(value) => known_bits!(value, spirv::TensorOperands),
        _ => true,
    }
}

fn consume_operand(kind: OperandKind, operands: &[Operand]) -> Option<usize> {
    let first = operands.first()?;
    if !operand_mask_bits_are_known(first) {
        return None;
    }
    let one = matches!(
        (kind, first),
        (OperandKind::FPFastMathMode, Operand::FPFastMathMode(_))
            | (OperandKind::SelectionControl, Operand::SelectionControl(_))
            | (OperandKind::FunctionControl, Operand::FunctionControl(_))
            | (OperandKind::MemorySemantics, Operand::MemorySemantics(_))
            | (
                OperandKind::KernelProfilingInfo,
                Operand::KernelProfilingInfo(_)
            )
            | (OperandKind::RayFlags, Operand::RayFlags(_))
            | (
                OperandKind::FragmentShadingRate,
                Operand::FragmentShadingRate(_)
            )
            | (
                OperandKind::RawAccessChainOperands,
                Operand::RawAccessChainOperands(_)
            )
            | (OperandKind::SourceLanguage, Operand::SourceLanguage(_))
            | (OperandKind::ExecutionModel, Operand::ExecutionModel(_))
            | (OperandKind::AddressingModel, Operand::AddressingModel(_))
            | (OperandKind::MemoryModel, Operand::MemoryModel(_))
            | (OperandKind::StorageClass, Operand::StorageClass(_))
            | (OperandKind::Dim, Operand::Dim(_))
            | (
                OperandKind::SamplerAddressingMode,
                Operand::SamplerAddressingMode(_)
            )
            | (
                OperandKind::SamplerFilterMode,
                Operand::SamplerFilterMode(_)
            )
            | (OperandKind::ImageFormat, Operand::ImageFormat(_))
            | (
                OperandKind::ImageChannelOrder,
                Operand::ImageChannelOrder(_)
            )
            | (
                OperandKind::ImageChannelDataType,
                Operand::ImageChannelDataType(_)
            )
            | (OperandKind::FPRoundingMode, Operand::FPRoundingMode(_))
            | (OperandKind::FPDenormMode, Operand::FPDenormMode(_))
            | (
                OperandKind::QuantizationModes,
                Operand::QuantizationModes(_)
            )
            | (OperandKind::FPOperationMode, Operand::FPOperationMode(_))
            | (OperandKind::OverflowModes, Operand::OverflowModes(_))
            | (OperandKind::LinkageType, Operand::LinkageType(_))
            | (OperandKind::AccessQualifier, Operand::AccessQualifier(_))
            | (
                OperandKind::HostAccessQualifier,
                Operand::HostAccessQualifier(_)
            )
            | (
                OperandKind::FunctionParameterAttribute,
                Operand::FunctionParameterAttribute(_)
            )
            | (OperandKind::BuiltIn, Operand::BuiltIn(_))
            | (OperandKind::Scope, Operand::Scope(_))
            | (OperandKind::GroupOperation, Operand::GroupOperation(_))
            | (
                OperandKind::KernelEnqueueFlags,
                Operand::KernelEnqueueFlags(_)
            )
            | (OperandKind::Capability, Operand::Capability(_))
            | (
                OperandKind::RayQueryIntersection,
                Operand::RayQueryIntersection(_)
            )
            | (
                OperandKind::RayQueryCommittedIntersectionType,
                Operand::RayQueryCommittedIntersectionType(_),
            )
            | (
                OperandKind::RayQueryCandidateIntersectionType,
                Operand::RayQueryCandidateIntersectionType(_),
            )
            | (
                OperandKind::PackedVectorFormat,
                Operand::PackedVectorFormat(_)
            )
            | (
                OperandKind::CooperativeMatrixOperands,
                Operand::CooperativeMatrixOperands(_)
            )
            | (
                OperandKind::CooperativeMatrixLayout,
                Operand::CooperativeMatrixLayout(_)
            )
            | (
                OperandKind::CooperativeMatrixUse,
                Operand::CooperativeMatrixUse(_)
            )
            | (
                OperandKind::CooperativeMatrixReduce,
                Operand::CooperativeMatrixReduce(_)
            )
            | (OperandKind::TensorClampMode, Operand::TensorClampMode(_))
            | (
                OperandKind::InitializationModeQualifier,
                Operand::InitializationModeQualifier(_)
            )
            | (OperandKind::LoadCacheControl, Operand::LoadCacheControl(_))
            | (
                OperandKind::StoreCacheControl,
                Operand::StoreCacheControl(_)
            )
            | (
                OperandKind::NamedMaximumNumberOfRegisters,
                Operand::NamedMaximumNumberOfRegisters(_)
            )
            | (
                OperandKind::MatrixMultiplyAccumulateOperands,
                Operand::MatrixMultiplyAccumulateOperands(_),
            )
            | (OperandKind::FPEncoding, Operand::FPEncoding(_))
            | (
                OperandKind::CooperativeVectorMatrixLayout,
                Operand::CooperativeVectorMatrixLayout(_)
            )
            | (OperandKind::ComponentType, Operand::ComponentType(_))
            | (
                OperandKind::IdMemorySemantics,
                Operand::IdMemorySemantics(_)
            )
            | (OperandKind::IdScope, Operand::IdScope(_))
            | (OperandKind::IdRef, Operand::IdRef(_))
            | (OperandKind::LiteralInteger, Operand::LiteralBit32(_))
            | (OperandKind::LiteralFloat, Operand::LiteralBit32(_))
            | (OperandKind::LiteralString, Operand::LiteralString(_))
            | (
                OperandKind::LiteralContextDependentNumber,
                Operand::LiteralBit32(_) | Operand::LiteralBit64(_),
            )
            | (
                OperandKind::LiteralExtInstInteger,
                Operand::LiteralExtInstInteger(_)
            )
    );
    if one {
        return Some(1);
    }
    match (kind, first) {
        (OperandKind::ImageOperands, Operand::ImageOperands(mask)) => {
            image_operand_tail(&operands[1..], *mask).map(|tail| tail + 1)
        }
        (OperandKind::LoopControl, Operand::LoopControl(mask)) => {
            loop_control_tail(&operands[1..], *mask).map(|tail| tail + 1)
        }
        (OperandKind::MemoryAccess, Operand::MemoryAccess(mask)) => {
            memory_access_tail(&operands[1..], *mask).map(|tail| tail + 1)
        }
        (OperandKind::ExecutionMode, Operand::ExecutionMode(mode)) => {
            execution_mode_tail(&operands[1..], *mode).map(|tail| tail + 1)
        }
        (OperandKind::Decoration, Operand::Decoration(decoration)) => {
            decoration_tail(&operands[1..], *decoration).map(|tail| tail + 1)
        }
        (OperandKind::TensorAddressingOperands, Operand::TensorAddressingOperands(mask)) => {
            tensor_addressing_tail(&operands[1..], *mask).map(|tail| tail + 1)
        }
        (OperandKind::TensorOperands, Operand::TensorOperands(mask)) => {
            tensor_tail(&operands[1..], *mask).map(|tail| tail + 1)
        }
        (
            OperandKind::PairLiteralIntegerIdRef,
            Operand::LiteralBit32(_) | Operand::LiteralBit64(_),
        ) if matches!(operands.get(1), Some(Operand::IdRef(_))) => Some(2),
        (OperandKind::PairIdRefLiteralInteger, Operand::IdRef(_))
            if matches!(operands.get(1), Some(Operand::LiteralBit32(_))) =>
        {
            Some(2)
        }
        (OperandKind::PairIdRefIdRef, Operand::IdRef(_))
            if matches!(operands.get(1), Some(Operand::IdRef(_))) =>
        {
            Some(2)
        }
        (
            OperandKind::LiteralSpecConstantOpInteger,
            Operand::LiteralSpecConstantOpInteger(opcode),
        ) => {
            let nested = INSTRUCTION_TABLE.lookup_opcode(*opcode as u32)?;
            operands_match(nested.operands, &operands[1..]).then_some(operands.len())
        }
        (OperandKind::IdResultType | OperandKind::IdResult, _) => None,
        _ => None,
    }
}

fn operands_match(logical: &[LogicalOperand], operands: &[Operand]) -> bool {
    let Some((first, rest)) = logical.split_first() else {
        return operands.is_empty();
    };
    if matches!(
        first.kind,
        OperandKind::IdResultType | OperandKind::IdResult
    ) {
        return operands_match(rest, operands);
    }
    match first.quantifier {
        OperandQuantifier::One => consume_operand(first.kind, operands)
            .is_some_and(|count| operands_match(rest, &operands[count..])),
        OperandQuantifier::ZeroOrOne => {
            operands_match(rest, operands)
                || consume_operand(first.kind, operands)
                    .is_some_and(|count| operands_match(rest, &operands[count..]))
        }
        OperandQuantifier::ZeroOrMore => {
            let mut offsets = vec![0];
            while let Some(count) =
                consume_operand(first.kind, &operands[*offsets.last().unwrap()..])
            {
                let next = offsets.last().unwrap() + count;
                offsets.push(next);
                if next == operands.len() {
                    break;
                }
            }
            offsets
                .into_iter()
                .rev()
                .any(|offset| operands_match(rest, &operands[offset..]))
        }
    }
}

pub(crate) fn instruction_operands_match(opcode: Op, operands: &[Operand]) -> bool {
    INSTRUCTION_TABLE
        .lookup_opcode(opcode as u32)
        .is_some_and(|grammar| operands_match(grammar.operands, operands))
}

include!("grammar_generated.rs");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owned_operand_matching_enforces_fixed_and_repeated_grammar_operands() {
        assert!(instruction_operands_match(Op::Branch, &[Operand::IdRef(1)]));
        assert!(!instruction_operands_match(
            Op::Branch,
            &[Operand::IdRef(1), Operand::IdRef(2)]
        ));
        assert!(instruction_operands_match(
            Op::TypeStruct,
            &[Operand::IdRef(1), Operand::IdRef(2), Operand::IdRef(3)]
        ));
        assert!(!instruction_operands_match(
            Op::TypeStruct,
            &[Operand::LiteralBit32(1)]
        ));
        assert!(instruction_operands_match(
            Op::Switch,
            &[
                Operand::IdRef(1),
                Operand::IdRef(2),
                Operand::LiteralBit32(0),
                Operand::IdRef(3),
                Operand::LiteralBit64(1),
                Operand::IdRef(4),
            ]
        ));
        assert!(!instruction_operands_match(
            Op::Switch,
            &[
                Operand::IdRef(1),
                Operand::IdRef(2),
                Operand::LiteralBit32(0),
            ]
        ));
    }

    #[test]
    fn owned_operand_matching_enforces_parameterized_operand_tails() {
        assert!(instruction_operands_match(
            Op::LoopMerge,
            &[
                Operand::IdRef(1),
                Operand::IdRef(2),
                Operand::LoopControl(spirv::LoopControl::DEPENDENCY_LENGTH),
                Operand::LiteralBit32(4),
            ]
        ));
        assert!(!instruction_operands_match(
            Op::LoopMerge,
            &[
                Operand::IdRef(1),
                Operand::IdRef(2),
                Operand::LoopControl(spirv::LoopControl::DEPENDENCY_LENGTH),
            ]
        ));
        assert!(instruction_operands_match(
            Op::ImageFetch,
            &[
                Operand::IdRef(1),
                Operand::IdRef(2),
                Operand::ImageOperands(spirv::ImageOperands::LOD),
                Operand::IdRef(3),
            ]
        ));
        assert!(!instruction_operands_match(
            Op::ImageFetch,
            &[
                Operand::IdRef(1),
                Operand::IdRef(2),
                Operand::ImageOperands(spirv::ImageOperands::LOD),
            ]
        ));
        assert!(instruction_operands_match(
            Op::Decorate,
            &[
                Operand::IdRef(1),
                Operand::Decoration(spirv::Decoration::BuiltIn),
                Operand::BuiltIn(spirv::BuiltIn::Position),
            ]
        ));
        assert!(!instruction_operands_match(
            Op::Decorate,
            &[
                Operand::IdRef(1),
                Operand::Decoration(spirv::Decoration::BuiltIn),
                Operand::LiteralBit32(0),
            ]
        ));
        assert!(!instruction_operands_match(
            Op::SelectionMerge,
            &[
                Operand::IdRef(1),
                Operand::SelectionControl(spirv::SelectionControl::from_bits_retain(u32::MAX)),
            ]
        ));
    }

    #[test]
    fn owned_operand_matching_enforces_nested_spec_constant_opcode_shape() {
        assert!(instruction_operands_match(
            Op::SpecConstantOp,
            &[
                Operand::LiteralSpecConstantOpInteger(Op::IAdd),
                Operand::IdRef(1),
                Operand::IdRef(2),
            ]
        ));
        assert!(!instruction_operands_match(
            Op::SpecConstantOp,
            &[
                Operand::LiteralSpecConstantOpInteger(Op::IAdd),
                Operand::IdRef(1),
            ]
        ));
    }

    #[test]
    fn instruction_requirements_retain_generated_declaration_metadata() {
        let (capabilities, extensions) =
            instruction_declaration_requirements(Op::AtomicFAddEXT).expect("core opcode");
        assert!(capabilities.contains(&spirv::Capability::AtomicFloat32AddEXT));
        assert!(extensions.contains(&"SPV_EXT_shader_atomic_float_add"));

        let (capabilities, extensions) =
            instruction_declaration_requirements(Op::Nop).expect("core opcode");
        assert!(capabilities.is_empty());
        assert!(extensions.is_empty());
    }
}
