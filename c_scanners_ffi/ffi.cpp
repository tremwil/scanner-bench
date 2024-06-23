#include "mem/pattern.h"
#include "mem/simd_scanner.h"

#include "pattern16/Pattern16.h"
#include "pattern16/pfreq.h"
#include "pattern16/scanners/base.h"
#include "pattern16/util.h"

#include <cstdint>

extern "C" __cdecl uintptr_t scan_pattern16(
    uint8_t* region, 
    size_t region_len, 
    uint8_t* bytes, 
    uint8_t* mask, 
    size_t len
) {
    Pattern16::Impl::SplitSignatureU8 sig;
    for (size_t i = 0; i < len; i++) {
        sig.first.push_back(bytes[i]);
        sig.second.push_back(mask[i]);
    }
    auto freqs = Pattern16::Impl::loadFrequencyCache();

    // Skip the CPUID instruction by calling with AVX2 directly (for fairness)
    return (uintptr_t)Pattern16::Impl::scanT<__m256i>(region, region_len, sig, freqs);
}

extern "C" __cdecl uintptr_t scan_mem_simd(
    uint8_t* region, 
    size_t region_len, 
    uint8_t* bytes, 
    uint8_t* mask, 
    size_t len
) {
    auto pat = mem::pattern(bytes, mask, len);
    return mem::simd_scanner(pat).scan(mem::region(region, region_len)).as<uintptr_t>();
}