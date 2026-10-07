#[repr(C)]
pub struct AccelerationStructureShadowHeader {
    pub instance_count: u32,
    pub reserved: u32,
}

pub const INSTANCE_COUNT_BYTE_OFFSET: u64 =
    std::mem::offset_of!(AccelerationStructureShadowHeader, instance_count) as u64;
pub const CHILD_REFERENCES_BYTE_OFFSET: u64 =
    std::mem::size_of::<AccelerationStructureShadowHeader>() as u64;
pub const CHILD_REFERENCE_BYTE_STRIDE: u64 = std::mem::size_of::<u64>() as u64;

#[repr(C)]
pub struct PrimitiveAccelerationStructureShadowHeader {
    pub triangle_count: u32,
    pub reserved: u32,
}

pub const PRIMITIVE_TRIANGLES_BYTE_OFFSET: u64 =
    std::mem::size_of::<PrimitiveAccelerationStructureShadowHeader>() as u64;
pub const PRIMITIVE_TRIANGLE_BYTE_STRIDE: u64 = 9 * std::mem::size_of::<f32>() as u64;
