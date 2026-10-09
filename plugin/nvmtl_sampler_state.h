/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#ifndef NVMTL_SAMPLER_STATE_H
#define NVMTL_SAMPLER_STATE_H
#include <math.h>
#include <string.h>
#include "nvmtl_sampler_desc.h"
typedef struct { int anisotropy, mirror_clamp; float max_anisotropy; } nvmtl_sampler_caps;
static inline int nvmtl_sampler_address(uint32_t mode, int mirror, VkSamplerAddressMode *out)
{
    switch (mode) {
    case 0: *out = VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_EDGE; break;
    case 1: if (!mirror) return -1; *out = VK_SAMPLER_ADDRESS_MODE_MIRROR_CLAMP_TO_EDGE; break;
    case 2: *out = VK_SAMPLER_ADDRESS_MODE_REPEAT; break;
    case 3: *out = VK_SAMPLER_ADDRESS_MODE_MIRRORED_REPEAT; break;
    case 4: case 5: *out = VK_SAMPLER_ADDRESS_MODE_CLAMP_TO_BORDER; break;
    default: return -1;
    }
    return 0;
}
static inline int nvmtl_sampler_info(const nvmtl_sampler_desc *d,
                                    const nvmtl_sampler_caps *caps, VkSamplerCreateInfo *out)
{
    if (!out) return -1;
    memset(out, 0, sizeof *out);
    if (!d || !caps || d->min_filter > 1 || d->mag_filter > 1 || d->mip_filter > 2 ||
        d->compare_function > 7 || d->border_color > 2 || d->max_anisotropy < 1 ||
        d->max_anisotropy > 16 || !isfinite(d->lod_min) || !isfinite(d->lod_max) ||
        d->lod_max < d->lod_min) return -1;
    VkSamplerCreateInfo s = { VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO };
    s.minFilter = d->min_filter ? VK_FILTER_LINEAR : VK_FILTER_NEAREST;
    s.magFilter = d->mag_filter ? VK_FILTER_LINEAR : VK_FILTER_NEAREST;
    s.mipmapMode = d->mip_filter == 2 ? VK_SAMPLER_MIPMAP_MODE_LINEAR : VK_SAMPLER_MIPMAP_MODE_NEAREST;
    if (nvmtl_sampler_address(d->address_s, caps->mirror_clamp, &s.addressModeU) ||
        nvmtl_sampler_address(d->address_t, caps->mirror_clamp, &s.addressModeV) ||
        nvmtl_sampler_address(d->address_r, caps->mirror_clamp, &s.addressModeW)) return -1;
    int zero = d->address_s == 4 || d->address_t == 4 || d->address_r == 4;
    int border = d->address_s == 5 || d->address_t == 5 || d->address_r == 5;

    if (zero && border && d->border_color != 0) return -1;
    s.borderColor = zero || d->border_color == 0 ? VK_BORDER_COLOR_FLOAT_TRANSPARENT_BLACK :
                    d->border_color == 1 ? VK_BORDER_COLOR_FLOAT_OPAQUE_BLACK : VK_BORDER_COLOR_FLOAT_OPAQUE_WHITE;
    /* Metal takes any lodMinClamp; a negative one means "no lower clamp" (Blender 4.2 sets -1000 on every sampler).
       WAS refused - every Blender sampler failed and the UI drew without them. LOD 0 is the same floor in Vulkan. */
    s.minLod = d->lod_min < 0 ? 0.0f : d->lod_min; s.maxLod = d->lod_max < s.minLod ? s.minLod : d->lod_max;
    if (d->mip_filter == 0) {

        s.minLod = 0; s.maxLod = d->min_filter == d->mag_filter ? 0 : 0.25f;
    }
    s.compareEnable = d->compare_function != 0;

    s.compareOp = (VkCompareOp)d->compare_function;
    s.maxAnisotropy = 1.0f;
    if (d->max_anisotropy > 1) {
        if (!caps->anisotropy || !isfinite(caps->max_anisotropy) ||
            caps->max_anisotropy < (float)d->max_anisotropy) return -1;
        s.anisotropyEnable = VK_TRUE; s.maxAnisotropy = (float)d->max_anisotropy;
    }
    s.unnormalizedCoordinates = VK_FALSE;
    *out = s; return 0;
}
#endif
