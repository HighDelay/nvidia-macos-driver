impl Parser<'_> {
    fn parse_operand(&mut self, kind: GOpKind) -> Result<Vec<Operand>> {
        Ok(match kind {
            GOpKind::FPFastMathMode => vec![Operand::FPFastMathMode(
                self.decoder.fp_fast_math_mode()?,
            )],
            GOpKind::SelectionControl => vec![Operand::SelectionControl(
                self.decoder.selection_control()?,
            )],
            GOpKind::FunctionControl => vec![Operand::FunctionControl(
                self.decoder.function_control()?,
            )],
            GOpKind::MemorySemantics => vec![Operand::MemorySemantics(
                self.decoder.memory_semantics()?,
            )],
            GOpKind::KernelProfilingInfo => vec![Operand::KernelProfilingInfo(
                self.decoder.kernel_profiling_info()?,
            )],
            GOpKind::RayFlags => vec![Operand::RayFlags(self.decoder.ray_flags()?)],
            GOpKind::FragmentShadingRate => vec![Operand::FragmentShadingRate(
                self.decoder.fragment_shading_rate()?,
            )],
            GOpKind::RawAccessChainOperands => vec![Operand::RawAccessChainOperands(
                self.decoder.raw_access_chain_operands()?,
            )],
            GOpKind::SourceLanguage => {
                vec![Operand::SourceLanguage(self.decoder.source_language()?)]
            }
            GOpKind::ExecutionModel => {
                vec![Operand::ExecutionModel(self.decoder.execution_model()?)]
            }
            GOpKind::AddressingModel => vec![Operand::AddressingModel(
                self.decoder.addressing_model()?,
            )],
            GOpKind::MemoryModel => vec![Operand::MemoryModel(self.decoder.memory_model()?)],
            GOpKind::StorageClass => vec![Operand::StorageClass(self.decoder.storage_class()?)],
            GOpKind::Dim => vec![Operand::Dim(self.decoder.dim()?)],
            GOpKind::SamplerAddressingMode => vec![Operand::SamplerAddressingMode(
                self.decoder.sampler_addressing_mode()?,
            )],
            GOpKind::SamplerFilterMode => vec![Operand::SamplerFilterMode(
                self.decoder.sampler_filter_mode()?,
            )],
            GOpKind::ImageFormat => vec![Operand::ImageFormat(self.decoder.image_format()?)],
            GOpKind::ImageChannelOrder => vec![Operand::ImageChannelOrder(
                self.decoder.image_channel_order()?,
            )],
            GOpKind::ImageChannelDataType => vec![Operand::ImageChannelDataType(
                self.decoder.image_channel_data_type()?,
            )],
            GOpKind::FPRoundingMode => vec![Operand::FPRoundingMode(
                self.decoder.fp_rounding_mode()?,
            )],
            GOpKind::FPDenormMode => {
                vec![Operand::FPDenormMode(self.decoder.fp_denorm_mode()?)]
            }
            GOpKind::QuantizationModes => vec![Operand::QuantizationModes(
                self.decoder.quantization_modes()?,
            )],
            GOpKind::FPOperationMode => vec![Operand::FPOperationMode(
                self.decoder.fp_operation_mode()?,
            )],
            GOpKind::OverflowModes => {
                vec![Operand::OverflowModes(self.decoder.overflow_modes()?)]
            }
            GOpKind::LinkageType => vec![Operand::LinkageType(self.decoder.linkage_type()?)],
            GOpKind::AccessQualifier => vec![Operand::AccessQualifier(
                self.decoder.access_qualifier()?,
            )],
            GOpKind::HostAccessQualifier => vec![Operand::HostAccessQualifier(
                self.decoder.host_access_qualifier()?,
            )],
            GOpKind::FunctionParameterAttribute => vec![Operand::FunctionParameterAttribute(
                self.decoder.function_parameter_attribute()?,
            )],
            GOpKind::BuiltIn => vec![Operand::BuiltIn(self.decoder.built_in()?)],
            GOpKind::Scope => vec![Operand::Scope(self.decoder.scope()?)],
            GOpKind::GroupOperation => {
                vec![Operand::GroupOperation(self.decoder.group_operation()?)]
            }
            GOpKind::KernelEnqueueFlags => vec![Operand::KernelEnqueueFlags(
                self.decoder.kernel_enqueue_flags()?,
            )],
            GOpKind::Capability => vec![Operand::Capability(self.decoder.capability()?)],
            GOpKind::RayQueryIntersection => vec![Operand::RayQueryIntersection(
                self.decoder.ray_query_intersection()?,
            )],
            GOpKind::RayQueryCommittedIntersectionType => {
                vec![Operand::RayQueryCommittedIntersectionType(
                    self.decoder.ray_query_committed_intersection_type()?,
                )]
            }
            GOpKind::RayQueryCandidateIntersectionType => {
                vec![Operand::RayQueryCandidateIntersectionType(
                    self.decoder.ray_query_candidate_intersection_type()?,
                )]
            }
            GOpKind::PackedVectorFormat => vec![Operand::PackedVectorFormat(
                self.decoder.packed_vector_format()?,
            )],
            GOpKind::CooperativeMatrixOperands => vec![Operand::CooperativeMatrixOperands(
                self.decoder.cooperative_matrix_operands()?,
            )],
            GOpKind::CooperativeMatrixLayout => vec![Operand::CooperativeMatrixLayout(
                self.decoder.cooperative_matrix_layout()?,
            )],
            GOpKind::CooperativeMatrixUse => vec![Operand::CooperativeMatrixUse(
                self.decoder.cooperative_matrix_use()?,
            )],
            GOpKind::CooperativeMatrixReduce => vec![Operand::CooperativeMatrixReduce(
                self.decoder.cooperative_matrix_reduce()?,
            )],
            GOpKind::TensorClampMode => vec![Operand::TensorClampMode(
                self.decoder.tensor_clamp_mode()?,
            )],
            GOpKind::InitializationModeQualifier => vec![Operand::InitializationModeQualifier(
                self.decoder.initialization_mode_qualifier()?,
            )],
            GOpKind::LoadCacheControl => vec![Operand::LoadCacheControl(
                self.decoder.load_cache_control()?,
            )],
            GOpKind::StoreCacheControl => vec![Operand::StoreCacheControl(
                self.decoder.store_cache_control()?,
            )],
            GOpKind::NamedMaximumNumberOfRegisters => {
                vec![Operand::NamedMaximumNumberOfRegisters(
                    self.decoder.named_maximum_number_of_registers()?,
                )]
            }
            GOpKind::MatrixMultiplyAccumulateOperands => {
                vec![Operand::MatrixMultiplyAccumulateOperands(
                    self.decoder.matrix_multiply_accumulate_operands()?,
                )]
            }
            GOpKind::FPEncoding => vec![Operand::FPEncoding(self.decoder.fp_encoding()?)],
            GOpKind::CooperativeVectorMatrixLayout => {
                vec![Operand::CooperativeVectorMatrixLayout(
                    self.decoder.cooperative_vector_matrix_layout()?,
                )]
            }
            GOpKind::ComponentType => {
                vec![Operand::ComponentType(self.decoder.component_type()?)]
            }
            GOpKind::IdMemorySemantics => vec![Operand::IdMemorySemantics(self.decoder.id()?)],
            GOpKind::IdScope => vec![Operand::IdScope(self.decoder.id()?)],
            GOpKind::IdRef => vec![Operand::IdRef(self.decoder.id()?)],
            GOpKind::LiteralInteger => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            GOpKind::LiteralString => vec![Operand::LiteralString(self.decoder.string()?)],
            GOpKind::LiteralFloat => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            GOpKind::LiteralExtInstInteger => vec![Operand::LiteralExtInstInteger(
                self.decoder.ext_inst_integer()?,
            )],
            GOpKind::PairIdRefLiteralInteger => vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
            ],
            GOpKind::PairIdRefIdRef => vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
            ],
            GOpKind::ImageOperands => {
                let val = self.decoder.image_operands()?;
                let mut ops = vec![Operand::ImageOperands(val)];
                ops.append(&mut self.parse_image_operands_arguments(val)?);
                ops
            }
            GOpKind::LoopControl => {
                let val = self.decoder.loop_control()?;
                let mut ops = vec![Operand::LoopControl(val)];
                ops.append(&mut self.parse_loop_control_arguments(val)?);
                ops
            }
            GOpKind::MemoryAccess => {
                let val = self.decoder.memory_access()?;
                let mut ops = vec![Operand::MemoryAccess(val)];
                ops.append(&mut self.parse_memory_access_arguments(val)?);
                ops
            }
            GOpKind::ExecutionMode => {
                let val = self.decoder.execution_mode()?;
                let mut ops = vec![Operand::ExecutionMode(val)];
                ops.append(&mut self.parse_execution_mode_arguments(val)?);
                ops
            }
            GOpKind::Decoration => {
                let val = self.decoder.decoration()?;
                let mut ops = vec![Operand::Decoration(val)];
                ops.append(&mut self.parse_decoration_arguments(val)?);
                ops
            }
            GOpKind::TensorAddressingOperands => {
                let val = self.decoder.tensor_addressing_operands()?;
                let mut ops = vec![Operand::TensorAddressingOperands(val)];
                ops.append(&mut self.parse_tensor_addressing_operands_arguments(val)?);
                ops
            }
            GOpKind::TensorOperands => {
                let val = self.decoder.tensor_operands()?;
                let mut ops = vec![Operand::TensorOperands(val)];
                ops.append(&mut self.parse_tensor_operands_arguments(val)?);
                ops
            }
            GOpKind::IdResultType => panic!(),
            GOpKind::IdResult => panic!(),
            GOpKind::LiteralContextDependentNumber => panic!(),
            GOpKind::LiteralSpecConstantOpInteger => panic!(),
            GOpKind::PairLiteralIntegerIdRef => panic!(),
        })
    }
    fn parse_image_operands_arguments(
        &mut self,
        image_operands: spirv::ImageOperands,
    ) -> Result<Vec<Operand>> {
        let mut params = vec![];
        if image_operands.contains(spirv::ImageOperands::BIAS) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::LOD) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::GRAD) {
            params.append(&mut vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
            ]);
        }
        if image_operands.contains(spirv::ImageOperands::CONST_OFFSET) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::OFFSET) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::CONST_OFFSETS) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::SAMPLE) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::MIN_LOD) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::MAKE_TEXEL_AVAILABLE) {
            params.append(&mut vec![Operand::IdScope(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::MAKE_TEXEL_VISIBLE) {
            params.append(&mut vec![Operand::IdScope(self.decoder.id()?)]);
        }
        if image_operands.contains(spirv::ImageOperands::OFFSETS) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        Ok(params)
    }
    fn parse_loop_control_arguments(
        &mut self,
        loop_control: spirv::LoopControl,
    ) -> Result<Vec<Operand>> {
        let mut params = vec![];
        if loop_control.contains(spirv::LoopControl::DEPENDENCY_LENGTH) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::MIN_ITERATIONS) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::MAX_ITERATIONS) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::ITERATION_MULTIPLE) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::PEEL_COUNT) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::PARTIAL_COUNT) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::INITIATION_INTERVAL_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::MAX_CONCURRENCY_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::DEPENDENCY_ARRAY_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::PIPELINE_ENABLE_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::LOOP_COALESCE_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::MAX_INTERLEAVING_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::SPECULATED_ITERATIONS_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::LOOP_COUNT_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if loop_control.contains(spirv::LoopControl::MAX_REINVOCATION_DELAY_ALTERA) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        Ok(params)
    }
    fn parse_memory_access_arguments(
        &mut self,
        memory_access: spirv::MemoryAccess,
    ) -> Result<Vec<Operand>> {
        let mut params = vec![];
        if memory_access.contains(spirv::MemoryAccess::ALIGNED) {
            params.append(&mut vec![Operand::LiteralBit32(self.decoder.bit32()?)]);
        }
        if memory_access.contains(spirv::MemoryAccess::MAKE_POINTER_AVAILABLE) {
            params.append(&mut vec![Operand::IdScope(self.decoder.id()?)]);
        }
        if memory_access.contains(spirv::MemoryAccess::MAKE_POINTER_VISIBLE) {
            params.append(&mut vec![Operand::IdScope(self.decoder.id()?)]);
        }
        if memory_access.contains(spirv::MemoryAccess::ALIAS_SCOPE_INTEL_MASK) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if memory_access.contains(spirv::MemoryAccess::NO_ALIAS_INTEL_MASK) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        Ok(params)
    }
    fn parse_execution_mode_arguments(
        &mut self,
        execution_mode: spirv::ExecutionMode,
    ) -> Result<Vec<Operand>> {
        Ok(match execution_mode {
            spirv::ExecutionMode::Invocations => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::LocalSize => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
            ],
            spirv::ExecutionMode::LocalSizeHint => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
            ],
            spirv::ExecutionMode::OutputVertices => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::VecTypeHint => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::SubgroupSize => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::SubgroupsPerWorkgroup => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::SubgroupsPerWorkgroupId => {
                vec![Operand::IdRef(self.decoder.id()?)]
            }
            spirv::ExecutionMode::LocalSizeId => vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
            ],
            spirv::ExecutionMode::LocalSizeHintId => vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
            ],
            spirv::ExecutionMode::DenormPreserve => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::DenormFlushToZero => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::SignedZeroInfNanPreserve => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::RoundingModeRTE => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::RoundingModeRTZ => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::TileShadingRateQCOM => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
            ],
            spirv::ExecutionMode::IsApiEntryAMDX => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::ExecutionMode::MaxNodeRecursionAMDX => {
                vec![Operand::IdRef(self.decoder.id()?)]
            }
            spirv::ExecutionMode::StaticNumWorkgroupsAMDX => vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
            ],
            spirv::ExecutionMode::ShaderIndexAMDX => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::ExecutionMode::MaxNumWorkgroupsAMDX => vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
            ],
            spirv::ExecutionMode::SharesInputWithAMDX => vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
            ],
            spirv::ExecutionMode::OutputPrimitivesEXT => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::SharedLocalMemorySizeINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::RoundingModeRTPINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::RoundingModeRTNINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::FloatingPointModeALTINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::FloatingPointModeIEEEINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::MaxWorkgroupSizeINTEL => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
            ],
            spirv::ExecutionMode::MaxWorkDimINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::NumSIMDWorkitemsINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::SchedulerTargetFmaxMhzINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::FPFastMathDefault => vec![
                Operand::IdRef(self.decoder.id()?),
                Operand::IdRef(self.decoder.id()?),
            ],
            spirv::ExecutionMode::StreamingInterfaceINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::RegisterMapInterfaceINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::NamedBarrierCountINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::MaximumRegistersINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::ExecutionMode::MaximumRegistersIdINTEL => {
                vec![Operand::IdRef(self.decoder.id()?)]
            }
            spirv::ExecutionMode::NamedMaximumRegistersINTEL => {
                vec![Operand::NamedMaximumNumberOfRegisters(
                    self.decoder.named_maximum_number_of_registers()?,
                )]
            }
            _ => vec![],
        })
    }
    fn parse_decoration_arguments(
        &mut self,
        decoration: spirv::Decoration,
    ) -> Result<Vec<Operand>> {
        Ok(match decoration {
            spirv::Decoration::SpecId => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::ArrayStride => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MatrixStride => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::BuiltIn => vec![Operand::BuiltIn(self.decoder.built_in()?)],
            spirv::Decoration::UniformId => vec![Operand::IdScope(self.decoder.id()?)],
            spirv::Decoration::Stream => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::Location => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::Component => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::Index => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::Binding => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::DescriptorSet => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::Offset => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::XfbBuffer => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::XfbStride => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::FuncParamAttr => vec![Operand::FunctionParameterAttribute(
                self.decoder.function_parameter_attribute()?,
            )],
            spirv::Decoration::FPRoundingMode => vec![Operand::FPRoundingMode(
                self.decoder.fp_rounding_mode()?,
            )],
            spirv::Decoration::FPFastMathMode => vec![Operand::FPFastMathMode(
                self.decoder.fp_fast_math_mode()?,
            )],
            spirv::Decoration::LinkageAttributes => vec![
                Operand::LiteralString(self.decoder.string()?),
                Operand::LinkageType(self.decoder.linkage_type()?),
            ],
            spirv::Decoration::InputAttachmentIndex => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::Alignment => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::MaxByteOffset => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::AlignmentId => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::MaxByteOffsetId => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::NodeSharesPayloadLimitsWithAMDX => {
                vec![Operand::IdRef(self.decoder.id()?)]
            }
            spirv::Decoration::NodeMaxPayloadsAMDX => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::PayloadNodeNameAMDX => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::PayloadNodeBaseIndexAMDX => {
                vec![Operand::IdRef(self.decoder.id()?)]
            }
            spirv::Decoration::PayloadNodeArraySizeAMDX => {
                vec![Operand::IdRef(self.decoder.id()?)]
            }
            spirv::Decoration::ArrayStrideIdEXT => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::OffsetIdEXT => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::SecondaryViewportRelativeNV => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MemberOffsetNV => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::BankNV => vec![Operand::LiteralBit32(self.decoder.bit32()?)],
            spirv::Decoration::SIMTCallINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::ClobberINTEL => {
                vec![Operand::LiteralString(self.decoder.string()?)]
            }
            spirv::Decoration::FuncParamIOKindINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::GlobalVariableOffsetINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::CounterBuffer => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::UserSemantic => {
                vec![Operand::LiteralString(self.decoder.string()?)]
            }
            spirv::Decoration::UserTypeGOOGLE => {
                vec![Operand::LiteralString(self.decoder.string()?)]
            }
            spirv::Decoration::FunctionRoundingModeINTEL => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::FPRoundingMode(self.decoder.fp_rounding_mode()?),
            ],
            spirv::Decoration::FunctionDenormModeINTEL => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::FPDenormMode(self.decoder.fp_denorm_mode()?),
            ],
            spirv::Decoration::MemoryALTERA => {
                vec![Operand::LiteralString(self.decoder.string()?)]
            }
            spirv::Decoration::NumbanksALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::BankwidthALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MaxPrivateCopiesALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MaxReplicatesALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MergeALTERA => vec![
                Operand::LiteralString(self.decoder.string()?),
                Operand::LiteralString(self.decoder.string()?),
            ],
            spirv::Decoration::BankBitsALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::ForcePow2DepthALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::StridesizeALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::WordsizeALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::CacheSizeALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::PrefetchALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MathOpDSPModeALTERA => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
            ],
            spirv::Decoration::AliasScopeINTEL => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::NoAliasINTEL => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::InitiationIntervalALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MaxConcurrencyALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::PipelineEnableALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::BufferLocationALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::IOPipeStorageALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::FunctionFloatingPointModeINTEL => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::FPOperationMode(self.decoder.fp_operation_mode()?),
            ],
            spirv::Decoration::FPMaxErrorDecorationINTEL => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::LatencyControlLabelALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::LatencyControlConstraintALTERA => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LiteralBit32(self.decoder.bit32()?),
            ],
            spirv::Decoration::MMHostInterfaceAddressWidthALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MMHostInterfaceDataWidthALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MMHostInterfaceLatencyALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MMHostInterfaceReadWriteModeALTERA => {
                vec![Operand::AccessQualifier(
                    self.decoder.access_qualifier()?,
                )]
            }
            spirv::Decoration::MMHostInterfaceMaxBurstALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::MMHostInterfaceWaitRequestALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::HostAccessINTEL => vec![
                Operand::HostAccessQualifier(self.decoder.host_access_qualifier()?),
                Operand::LiteralString(self.decoder.string()?),
            ],
            spirv::Decoration::InitModeALTERA => vec![Operand::InitializationModeQualifier(
                self.decoder.initialization_mode_qualifier()?,
            )],
            spirv::Decoration::ImplementInRegisterMapALTERA => {
                vec![Operand::LiteralBit32(self.decoder.bit32()?)]
            }
            spirv::Decoration::ConditionalINTEL => vec![Operand::IdRef(self.decoder.id()?)],
            spirv::Decoration::CacheControlLoadINTEL => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::LoadCacheControl(self.decoder.load_cache_control()?),
            ],
            spirv::Decoration::CacheControlStoreINTEL => vec![
                Operand::LiteralBit32(self.decoder.bit32()?),
                Operand::StoreCacheControl(self.decoder.store_cache_control()?),
            ],
            _ => vec![],
        })
    }
    fn parse_tensor_addressing_operands_arguments(
        &mut self,
        tensor_addressing_operands: spirv::TensorAddressingOperands,
    ) -> Result<Vec<Operand>> {
        let mut params = vec![];
        if tensor_addressing_operands.contains(spirv::TensorAddressingOperands::TENSOR_VIEW) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if tensor_addressing_operands.contains(spirv::TensorAddressingOperands::DECODE_FUNC) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        Ok(params)
    }
    fn parse_tensor_operands_arguments(
        &mut self,
        tensor_operands: spirv::TensorOperands,
    ) -> Result<Vec<Operand>> {
        let mut params = vec![];
        if tensor_operands.contains(spirv::TensorOperands::OUT_OF_BOUNDS_VALUE_ARM) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if tensor_operands.contains(spirv::TensorOperands::MAKE_ELEMENT_AVAILABLE_ARM) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        if tensor_operands.contains(spirv::TensorOperands::MAKE_ELEMENT_VISIBLE_ARM) {
            params.append(&mut vec![Operand::IdRef(self.decoder.id()?)]);
        }
        Ok(params)
    }
}
