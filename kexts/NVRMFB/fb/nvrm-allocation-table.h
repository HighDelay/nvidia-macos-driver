/*
 * Copyright (c) 2026 NullMoth Systems.
 * SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
 */
#pragma once
// Calls must hold the shared display/allocation gate. A cookie never repeats.
template<class Memory, unsigned Capacity> class NVRMAllocationTable {
    struct Entry { Memory *memory; unsigned long long bytes, cookie; } entries[Capacity] = {};
    unsigned long long nextCookie = 1;
public:
    unsigned long long insert(Memory *memory, unsigned long long bytes) {
        if (!memory || !bytes || nextCookie == ~0ULL) return 0;
        for (unsigned i = 0; i < Capacity; ++i) if (entries[i].memory == memory) return 0;
        for (unsigned i = 0; i < Capacity; ++i) if (!entries[i].memory) {
            const unsigned long long cookie = nextCookie++;
            entries[i] = {memory, bytes, cookie}; return cookie;
        }
        return 0;
    }
    unsigned long long size(Memory *memory, unsigned long long cookie) const {
        if (!memory || !cookie) return 0;
        for (unsigned i = 0; i < Capacity; ++i)
            if (entries[i].memory == memory && entries[i].cookie == cookie) return entries[i].bytes;
        return 0;
    }
    bool remove(Memory *memory, unsigned long long cookie) {
        if (!memory || !cookie) return false;
        for (unsigned i = 0; i < Capacity; ++i) if (entries[i].memory == memory && entries[i].cookie == cookie) {
            entries[i] = {}; return true;
        }
        return false;
    }
};
