/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#ifndef NVMTL_SAMPLE_POSITIONS_H
#define NVMTL_SAMPLE_POSITIONS_H

#include <stdint.h>
#include <math.h>
#include <float.h>
#include <string.h>

#define NVMTL_MAX_SAMPLE_POSITIONS 8u
#define NVMTL_SAMPLE_POSITION_BYTES 64u
#define NVMTL_SAMPLE_POSITION_OFFSET 96u
typedef struct { float x, y; } nvmtl_sample_position;
typedef struct {
    uint32_t count;
    nvmtl_sample_position position[NVMTL_MAX_SAMPLE_POSITIONS];
} nvmtl_sample_pattern;
_Static_assert(FLT_RADIX == 2 && FLT_MANT_DIG == 24 && FLT_MAX_EXP == 128, "sample coordinates require IEEE binary32");
_Static_assert(sizeof(nvmtl_sample_position) == 8, "float2 sample-position ABI");
_Static_assert(sizeof(((nvmtl_sample_pattern *)0)->position) == NVMTL_SAMPLE_POSITION_BYTES,
               "fragment sample-position push ABI");

static inline int nvmtl_sample_default(uint32_t count, nvmtl_sample_pattern *out)
{
    static const uint8_t p1[][2] = {{8,8}};
    static const uint8_t p2[][2] = {{12,12},{4,4}};
    static const uint8_t p4[][2] = {{6,2},{14,6},{2,10},{10,14}};
    static const uint8_t p8[][2] = {{9,5},{7,11},{13,9},{5,3},{3,13},{1,7},{11,15},{15,1}};
    const uint8_t (*p)[2] = count == 1 ? p1 : count == 2 ? p2 : count == 4 ? p4 : count == 8 ? p8 : NULL;
    if (!p || !out) return -1;
    memset(out, 0, sizeof *out); out->count = count;
    for (uint32_t i = 0; i < count; ++i) {
        out->position[i].x = p[i][0] / 16.0f;
        out->position[i].y = p[i][1] / 16.0f;
    }
    return 0;
}

static inline int nvmtl_sample_pattern_valid(const nvmtl_sample_pattern *p)
{
    if (!p || (p->count != 1 && p->count != 2 && p->count != 4 && p->count != 8)) return 0;
    for (uint32_t i = 0; i < p->count; ++i) {
        const float x = p->position[i].x, y = p->position[i].y;
        if (!isfinite(x) || !isfinite(y) || x < 0 || x >= 1 || y < 0 || y >= 1) return 0;
    }
    return 1;
}

static inline int nvmtl_sample_pattern_quantize(const nvmtl_sample_pattern *requested,
                                               nvmtl_sample_pattern *canonical)
{
    if (!canonical || !nvmtl_sample_pattern_valid(requested)) return -1;
    nvmtl_sample_pattern result = {0}; result.count = requested->count;
    for (uint32_t i = 0; i < result.count; ++i) {
        result.position[i].x = floorf(requested->position[i].x * 16.0f + 0.5f) / 16.0f;
        result.position[i].y = floorf(requested->position[i].y * 16.0f + 0.5f) / 16.0f;
    }
    *canonical = result; return 0;
}
static inline float nvmtl_sample_raster_coordinate(float canonical)
{
    return canonical == 1.0f ? 0.0f : canonical;
}

#endif
