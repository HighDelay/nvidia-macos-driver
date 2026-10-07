/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#ifndef NVRM_AGDC_K5_H
#define NVRM_AGDC_K5_H
#include <stdint.h>

#define NVK5_CMD_LINKCONFIG   0x921u
#define NVK5_CMD_FBCAPEX      0x711u
#define NVK5_CAP_PLANESCALER  0x10u
#define NVK5_TIMING_TAG       0x4b350001u
#define NVK5_TIMING_WORDS     13

enum { NVK5_T_TAG, NVK5_T_PCLK, NVK5_T_HVIS, NVK5_T_HSS, NVK5_T_HSE, NVK5_T_HTOT, NVK5_T_VVIS, NVK5_T_VSS, NVK5_T_VSE, NVK5_T_VTOT,
       NVK5_T_REFRESH_MHZ, NVK5_T_HSYNCPOS, NVK5_T_VSYNCPOS };

#pragma pack(push, 1)
struct NVAGDCTiming {
    uint32_t hScaledInset, vScaledInset, scalerFlags, hScaled, vScaled, signalConfig, signalLevels;
    uint64_t pixelClock, minPixelClock, maxPixelClock;
    uint32_t hActive, hBlanking, hSyncOffset, hSyncPulseWidth;
    uint32_t vActive, vBlanking, vSyncOffset, vSyncPulseWidth;
    uint32_t hBorderLeft, hBorderRight, vBorderTop, vBorderBottom;
    uint32_t hSyncConfig, hSyncLevel, vSyncConfig, vSyncLevel;
    uint32_t numLinks, vBlankingExtension;
    uint16_t pixelEncoding, bitsPerColorComponent, colorimetry, dynamicRange;
    uint16_t dscBitsPerPixel, dscSliceHeight, dscSliceWidth, field8a;
    uint8_t  tail[8];
};
struct NVAGDCLinkConfig {
    int32_t  id;
    int32_t  groupID;
    uint64_t flags;
    struct NVAGDCTiming timing;
    uint32_t port, stream;
    uint32_t state;
};
struct NVAGDCFBExtCap {
    uint32_t fbIndex, type, count;
    uint8_t  entry[8][0x32c];
};
#pragma pack(pop)
_Static_assert(sizeof(struct NVAGDCTiming) == 0x94, "AGDC timing is 0x94");
_Static_assert(sizeof(struct NVAGDCLinkConfig) == 0xb0, "AGDCLinkConfig_t is 0xb0");
_Static_assert(__builtin_offsetof(struct NVAGDCLinkConfig, port) == 0xa4, "stream address at +0xa4");
_Static_assert(sizeof(struct NVAGDCFBExtCap) == 0x196c, "AGDCFBGetExtendedCapability_t is 0x196c");

static inline void nvk5_zero(void *p, unsigned long n) { uint8_t *b = (uint8_t *)p; while (n--) *b++ = 0; }
static inline void nvk5_put32(uint8_t *e, unsigned off, uint32_t v) { e[off] = (uint8_t)v; e[off+1] = (uint8_t)(v >> 8); e[off+2] = (uint8_t)(v >> 16); e[off+3] = (uint8_t)(v >> 24); }

static inline int nvk5_timing_bad(const uint32_t *t, unsigned long bytes)
{
    if (!t || bytes != NVK5_TIMING_WORDS * sizeof(uint32_t)) return 1;
    if (t[NVK5_T_TAG] != NVK5_TIMING_TAG) return 2;
    if (!t[NVK5_T_PCLK] || !t[NVK5_T_HVIS] || !t[NVK5_T_VVIS]) return 3;
    if (!(t[NVK5_T_HVIS] <= t[NVK5_T_HSS] && t[NVK5_T_HSS] < t[NVK5_T_HSE] && t[NVK5_T_HSE] <= t[NVK5_T_HTOT])) return 4;
    if (!(t[NVK5_T_VVIS] <= t[NVK5_T_VSS] && t[NVK5_T_VSS] < t[NVK5_T_VSE] && t[NVK5_T_VSE] <= t[NVK5_T_VTOT])) return 5;
    if (t[NVK5_T_HTOT] == t[NVK5_T_HVIS] || t[NVK5_T_VTOT] == t[NVK5_T_VVIS]) return 6;
    return 0;
}

static inline void nvk5_fill_link(struct NVAGDCLinkConfig *l, const uint32_t *t)
{
    int32_t id = l->id; nvk5_zero(l, sizeof *l); l->id = id;
    l->groupID = -1;
    l->flags = 0x8 | 0x1 | 0x2;
    struct NVAGDCTiming *g = &l->timing;
    g->pixelClock = g->minPixelClock = g->maxPixelClock = t[NVK5_T_PCLK];
    g->hActive = t[NVK5_T_HVIS]; g->hBlanking = t[NVK5_T_HTOT] - t[NVK5_T_HVIS];
    g->hSyncOffset = t[NVK5_T_HSS] - t[NVK5_T_HVIS]; g->hSyncPulseWidth = t[NVK5_T_HSE] - t[NVK5_T_HSS];
    g->vActive = t[NVK5_T_VVIS]; g->vBlanking = t[NVK5_T_VTOT] - t[NVK5_T_VVIS];
    g->vSyncOffset = t[NVK5_T_VSS] - t[NVK5_T_VVIS]; g->vSyncPulseWidth = t[NVK5_T_VSE] - t[NVK5_T_VSS];
    g->hSyncConfig = t[NVK5_T_HSYNCPOS] ? 1 : 0; g->vSyncConfig = t[NVK5_T_VSYNCPOS] ? 1 : 0;
    g->numLinks = 1;
    g->pixelEncoding = 0x0001;
    g->bitsPerColorComponent = 0x0002;
    g->colorimetry = 0x0001;
    g->dynamicRange = 0x0001;
    l->port = 1; l->stream = 0;
    l->state = 1;
}
static inline void nvk5_fill_link_port(struct NVAGDCLinkConfig *l, const uint32_t *t, uint32_t port)
{
    nvk5_fill_link(l, t); l->port = port;
}

static inline void nvk5_fill_planescaler(struct NVAGDCFBExtCap *c, const uint32_t *t)
{
    uint32_t idx = c->fbIndex, type = c->type; nvk5_zero(c, sizeof *c); c->fbIndex = idx; c->type = type;
    c->count = 1;
    uint8_t *e = c->entry[0]; uint32_t w = t[NVK5_T_HVIS], h = t[NVK5_T_VVIS];
    nvk5_put32(e, 0x000, 0x80004);
    nvk5_put32(e, 0x004, 0);
    nvk5_put32(e, 0x01c, 1);
    nvk5_put32(e, 0x024, h); nvk5_put32(e, 0x028, w); nvk5_put32(e, 0x02c, h); nvk5_put32(e, 0x030, w);
    nvk5_put32(e, 0x034, 0x70007);
    nvk5_put32(e, 0x038, 0);
    nvk5_put32(e, 0x1a4, 1);
    nvk5_put32(e, 0x1ac, h); nvk5_put32(e, 0x1b0, w); nvk5_put32(e, 0x1b4, h); nvk5_put32(e, 0x1b8, w);
    nvk5_put32(e, 0x1bc, 0x70007);
    nvk5_put32(e, 0x1c0, 0);
}

#define NVK5_R_UNSUPPORTED  0
#define NVK5_R_LINK         1
#define NVK5_R_CAPS         2
#define NVK5_R_BADARG     (-1)
#define NVK5_R_NOTFOUND   (-2)
struct nvk5_head { const uint32_t *t; unsigned long tBytes; };
static inline int nvk5_answer_heads(unsigned cmd, const void *in, unsigned long inSize, void *out, unsigned long outSize,
                                    const struct nvk5_head *h, unsigned nh, int *why, int32_t *idxOut)
{
    *idxOut = -1;
    if (cmd != NVK5_CMD_LINKCONFIG && cmd != NVK5_CMD_FBCAPEX) return NVK5_R_UNSUPPORTED;
    const int link = cmd == NVK5_CMD_LINKCONFIG;
    const unsigned long want = link ? sizeof(struct NVAGDCLinkConfig) : sizeof(struct NVAGDCFBExtCap);
    if (!out || outSize != want) { *why = link ? 0x10 : 0x20; return NVK5_R_BADARG; }
    if (!in || inSize != want) { *why = link ? 0x12 : 0x22; return NVK5_R_BADARG; }
    if (in != out) {
        const uint8_t *s = (const uint8_t *)in; uint8_t *d = (uint8_t *)out; unsigned long n = want;
        if (d < s) while (n--) *d++ = *s++; else while (n--) d[n] = s[n];
    }
    uint32_t idx = link ? (uint32_t)((struct NVAGDCLinkConfig *)out)->id : ((struct NVAGDCFBExtCap *)out)->fbIndex;
    *idxOut = (int32_t)idx;
    if (!link && ((struct NVAGDCFBExtCap *)out)->type != NVK5_CAP_PLANESCALER) return NVK5_R_UNSUPPORTED;
    if (idx >= nh) { *why = link ? 0x11 : 0x21; return NVK5_R_BADARG; }
    const uint32_t *t = h[idx].t;
    int bad = t ? nvk5_timing_bad(t, h[idx].tBytes) : 0x31;
    if (bad) { *why = bad; return NVK5_R_NOTFOUND; }
    if (link) { nvk5_fill_link_port((struct NVAGDCLinkConfig *)out, t, 1u + idx); return NVK5_R_LINK; }
    nvk5_fill_planescaler((struct NVAGDCFBExtCap *)out, t); return NVK5_R_CAPS;
}
static inline int nvk5_answer(unsigned cmd, const void *in, unsigned long inSize, void *out, unsigned long outSize,
                              const uint32_t *t, unsigned long tBytes, int *why)
{
    struct nvk5_head h0 = { t, tBytes }; int32_t idx;
    return nvk5_answer_heads(cmd, in, inSize, out, outSize, &h0, 1, why, &idx);
}
#endif
