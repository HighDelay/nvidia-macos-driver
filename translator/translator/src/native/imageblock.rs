const MAX_THREADS_PER_THREADGROUP: u32 = 1024;

const TILE_BYTE_BUDGET: u32 = 16384;

pub(super) fn cell_capacity(cell_bytes: u32, cell_scale: u32) -> u32 {
    MAX_THREADS_PER_THREADGROUP
        .saturating_mul(cell_scale.saturating_mul(cell_scale))
        .min(TILE_BYTE_BUDGET / cell_bytes.max(1))
        .max(1)
}
