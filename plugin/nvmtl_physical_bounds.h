/*
 * NullMoth NVIDIA driver for macOS
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */

#ifndef NVMTL_PHYSICAL_BOUNDS_H
#define NVMTL_PHYSICAL_BOUNDS_H
#include <stdint.h>
#include <stdlib.h>
#include <stddef.h>

static int nvmtl_relax_physical_bounds(uint32_t *w, size_t count) {
    if (!w || count < 5 || w[0] != 0x07230203u || !w[3] || w[3] > 0x1000000u) return -1;
    unsigned char *physical = calloc(w[3], 1);
    if (!physical) return -1;
    for (size_t i=5; i<count;) {
        uint32_t n=w[i]>>16, op=w[i]&0xffffu;
        if (!n || n>count-i) { free(physical); return -1; }
        if (op==32) {
            if(n!=4 || w[i+1]>=w[3]) { free(physical); return -1; }
            if(w[i+2]==5349) physical[w[i+1]]=1;
        }
        if ((op==66 || op==70) && (n<4 || w[i+1]>=w[3])) { free(physical); return -1; }
        i+=n;
    }
    int changed=0;
    for (size_t i=5; i<count;) {
        uint32_t n=w[i]>>16, op=w[i]&0xffffu;
        if ((op==66 || op==70) && physical[w[i+1]]) {
            w[i]=(n<<16)|(op==66 ? 65u : 67u);
            changed++;
        }
        i+=n;
    }
    free(physical); return changed;
}
#endif
