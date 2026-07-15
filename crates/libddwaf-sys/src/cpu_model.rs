// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception

// Rust port of compiler-rt 19.1.7's lib/builtins/cpu_model/x86.c.

use core::arch::x86_64::__cpuid_count;
use core::sync::atomic::{AtomicU32, Ordering};

const SIG_INTEL: u32 = 0x756e_6547; // "Genu"
const SIG_AMD: u32 = 0x6874_7541; // "Auth"

const VENDOR_INTEL: u32 = 1;
const VENDOR_AMD: u32 = 2;
const VENDOR_OTHER: u32 = 3;

mod cpu_type {
    pub const INTEL_BONNELL: u32 = 1;
    pub const INTEL_CORE2: u32 = 2;
    pub const INTEL_COREI7: u32 = 3;
    pub const AMDFAM10H: u32 = 4;
    pub const AMDFAM15H: u32 = 5;
    pub const INTEL_SILVERMONT: u32 = 6;
    pub const INTEL_KNL: u32 = 7;
    pub const AMD_BTVER1: u32 = 8;
    pub const AMD_BTVER2: u32 = 9;
    pub const AMDFAM17H: u32 = 10;
    pub const INTEL_KNM: u32 = 11;
    pub const INTEL_GOLDMONT: u32 = 12;
    pub const INTEL_GOLDMONT_PLUS: u32 = 13;
    pub const INTEL_TREMONT: u32 = 14;
    pub const AMDFAM19H: u32 = 15;
    #[allow(dead_code)]
    pub const ZHAOXIN_FAM7H: u32 = 16;
    pub const INTEL_SIERRAFOREST: u32 = 17;
    pub const INTEL_GRANDRIDGE: u32 = 18;
    pub const INTEL_CLEARWATERFOREST: u32 = 19;
    pub const AMDFAM1AH: u32 = 20;
}

mod cpu_subtype {
    pub const INTEL_COREI7_NEHALEM: u32 = 1;
    pub const INTEL_COREI7_WESTMERE: u32 = 2;
    pub const INTEL_COREI7_SANDYBRIDGE: u32 = 3;
    pub const AMDFAM10H_BARCELONA: u32 = 4;
    pub const AMDFAM10H_SHANGHAI: u32 = 5;
    pub const AMDFAM10H_ISTANBUL: u32 = 6;
    pub const AMDFAM15H_BDVER1: u32 = 7;
    pub const AMDFAM15H_BDVER2: u32 = 8;
    pub const AMDFAM15H_BDVER3: u32 = 9;
    pub const AMDFAM15H_BDVER4: u32 = 10;
    pub const AMDFAM17H_ZNVER1: u32 = 11;
    pub const INTEL_COREI7_IVYBRIDGE: u32 = 12;
    pub const INTEL_COREI7_HASWELL: u32 = 13;
    pub const INTEL_COREI7_BROADWELL: u32 = 14;
    pub const INTEL_COREI7_SKYLAKE: u32 = 15;
    pub const INTEL_COREI7_SKYLAKE_AVX512: u32 = 16;
    pub const INTEL_COREI7_CANNONLAKE: u32 = 17;
    pub const INTEL_COREI7_ICELAKE_CLIENT: u32 = 18;
    pub const INTEL_COREI7_ICELAKE_SERVER: u32 = 19;
    pub const AMDFAM17H_ZNVER2: u32 = 20;
    pub const INTEL_COREI7_CASCADELAKE: u32 = 21;
    pub const INTEL_COREI7_TIGERLAKE: u32 = 22;
    pub const INTEL_COREI7_COOPERLAKE: u32 = 23;
    pub const INTEL_COREI7_SAPPHIRERAPIDS: u32 = 24;
    pub const INTEL_COREI7_ALDERLAKE: u32 = 25;
    pub const AMDFAM19H_ZNVER3: u32 = 26;
    pub const INTEL_COREI7_ROCKETLAKE: u32 = 27;
    #[allow(dead_code)]
    pub const ZHAOXIN_FAM7H_LUJIAZUI: u32 = 28;
    pub const AMDFAM19H_ZNVER4: u32 = 29;
    pub const INTEL_COREI7_GRANITERAPIDS: u32 = 30;
    pub const INTEL_COREI7_GRANITERAPIDS_D: u32 = 31;
    pub const INTEL_COREI7_ARROWLAKE: u32 = 32;
    pub const INTEL_COREI7_ARROWLAKE_S: u32 = 33;
    pub const INTEL_COREI7_PANTHERLAKE: u32 = 34;
    pub const AMDFAM1AH_ZNVER5: u32 = 35;
}

mod feature {
    pub const CMOV: usize = 0;
    pub const MMX: usize = 1;
    pub const POPCNT: usize = 2;
    pub const SSE: usize = 3;
    pub const SSE2: usize = 4;
    pub const SSE3: usize = 5;
    pub const SSSE3: usize = 6;
    pub const SSE4_1: usize = 7;
    pub const SSE4_2: usize = 8;
    pub const AVX: usize = 9;
    pub const AVX2: usize = 10;
    pub const SSE4_A: usize = 11;
    pub const FMA4: usize = 12;
    pub const XOP: usize = 13;
    pub const FMA: usize = 14;
    pub const AVX512F: usize = 15;
    pub const BMI: usize = 16;
    pub const BMI2: usize = 17;
    pub const AES: usize = 18;
    pub const PCLMUL: usize = 19;
    pub const AVX512VL: usize = 20;
    pub const AVX512BW: usize = 21;
    pub const AVX512DQ: usize = 22;
    pub const AVX512CD: usize = 23;
    pub const AVX512ER: usize = 24;
    pub const AVX512PF: usize = 25;
    pub const AVX512VBMI: usize = 26;
    pub const AVX512IFMA: usize = 27;
    pub const AVX5124VNNIW: usize = 28;
    pub const AVX5124FMAPS: usize = 29;
    pub const AVX512VPOPCNTDQ: usize = 30;
    pub const AVX512VBMI2: usize = 31;
    pub const GFNI: usize = 32;
    pub const VPCLMULQDQ: usize = 33;
    pub const AVX512VNNI: usize = 34;
    pub const AVX512BITALG: usize = 35;
    pub const AVX512BF16: usize = 36;
    pub const AVX512VP2INTERSECT: usize = 37;
    pub const ADX: usize = 40;
    pub const CLDEMOTE: usize = 42;
    // Present in the ABI table but not populated by compiler-rt 19.1.7.
    #[allow(dead_code)]
    pub const CLFLUSHOPT: usize = 43;
    pub const CLWB: usize = 44;
    pub const CLZERO: usize = 45;
    pub const CMPXCHG16B: usize = 46;
    pub const ENQCMD: usize = 48;
    pub const F16C: usize = 49;
    pub const FSGSBASE: usize = 50;
    pub const LAHF_LM: usize = 54;
    pub const LM: usize = 55;
    pub const LWP: usize = 56;
    pub const LZCNT: usize = 57;
    pub const MOVBE: usize = 58;
    pub const MOVDIR64B: usize = 59;
    pub const MOVDIRI: usize = 60;
    pub const MWAITX: usize = 61;
    pub const PCONFIG: usize = 63;
    pub const PKU: usize = 64;
    pub const PREFETCHWT1: usize = 65;
    pub const PRFCHW: usize = 66;
    pub const PTWRITE: usize = 67;
    pub const RDPID: usize = 68;
    pub const RDRND: usize = 69;
    pub const RDSEED: usize = 70;
    pub const RTM: usize = 71;
    pub const SERIALIZE: usize = 72;
    pub const SGX: usize = 73;
    pub const SHA: usize = 74;
    pub const SHSTK: usize = 75;
    pub const TBM: usize = 76;
    pub const TSXLDTRK: usize = 77;
    pub const VAES: usize = 78;
    pub const WAITPKG: usize = 79;
    pub const WBNOINVD: usize = 80;
    pub const XSAVE: usize = 81;
    pub const XSAVEC: usize = 82;
    pub const XSAVEOPT: usize = 83;
    pub const XSAVES: usize = 84;
    pub const AMX_TILE: usize = 85;
    pub const AMX_INT8: usize = 86;
    pub const AMX_BF16: usize = 87;
    pub const UINTR: usize = 88;
    pub const HRESET: usize = 89;
    pub const KL: usize = 90;
    pub const WIDEKL: usize = 92;
    pub const AVXVNNI: usize = 93;
    pub const AVX512FP16: usize = 94;
    pub const X86_64_BASELINE: usize = 95;
    pub const X86_64_V2: usize = 96;
    pub const X86_64_V3: usize = 97;
    pub const X86_64_V4: usize = 98;
    pub const AVXIFMA: usize = 99;
    pub const AVXVNNIINT8: usize = 100;
    pub const AVXNECONVERT: usize = 101;
    pub const CMPCCXADD: usize = 102;
    pub const AMX_FP16: usize = 103;
    pub const PREFETCHI: usize = 104;
    pub const RAOINT: usize = 105;
    pub const AMX_COMPLEX: usize = 106;
    pub const AVXVNNIINT16: usize = 107;
    pub const SM3: usize = 108;
    pub const SHA512: usize = 109;
    pub const SM4: usize = 110;
    pub const APXF: usize = 111;
    pub const USERMSR: usize = 112;
    pub const AVX10_1_256: usize = 113;
    pub const AVX10_1_512: usize = 114;

    #[allow(dead_code)]
    pub const CPU_FEATURE_MAX: usize = 115;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Registers {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
}

trait Hardware {
    fn cpuid(&self, leaf: u32, subleaf: u32) -> Registers;
    fn xcr0(&self) -> u64;
}

struct NativeHardware;

impl Hardware for NativeHardware {
    #[inline]
    #[allow(unused_unsafe)]
    fn cpuid(&self, leaf: u32, subleaf: u32) -> Registers {
        // SAFETY: CPUID is guaranteed to exist in 64-bit mode.
        let result = unsafe { __cpuid_count(leaf, subleaf) };
        Registers {
            eax: result.eax,
            ebx: result.ebx,
            ecx: result.ecx,
            edx: result.edx,
        }
    }

    #[inline]
    fn xcr0(&self) -> u64 {
        let eax: u32;
        let edx: u32;
        // SAFETY: callers execute this only after CPUID reports OSXSAVE.
        unsafe {
            core::arch::asm!(
                ".byte 0x0f, 0x01, 0xd0",
                in("ecx") 0_u32,
                out("eax") eax,
                out("edx") edx,
                options(nomem, nostack),
            );
        }
        (u64::from(edx) << 32) | u64::from(eax)
    }
}

#[repr(C)]
struct ProcessorModel {
    vendor: AtomicU32,
    cpu_type: AtomicU32,
    subtype: AtomicU32,
    features: [AtomicU32; 1],
}

#[no_mangle]
static __cpu_model: ProcessorModel = ProcessorModel {
    vendor: AtomicU32::new(0),
    cpu_type: AtomicU32::new(0),
    subtype: AtomicU32::new(0),
    features: [AtomicU32::new(0)],
};

#[no_mangle]
static __cpu_features2: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DetectedCpu {
    vendor: u32,
    cpu_type: u32,
    subtype: u32,
    features: [u32; 4],
}

#[no_mangle]
pub extern "C" fn __cpu_indicator_init() -> i32 {
    if __cpu_model.vendor.load(Ordering::Acquire) != 0 {
        return 0;
    }

    let detected = match detect_cpu(&NativeHardware) {
        Some(detected) => detected,
        None => {
            __cpu_model.vendor.store(VENDOR_OTHER, Ordering::Release);
            return -1;
        }
    };

    __cpu_model
        .cpu_type
        .store(detected.cpu_type, Ordering::Relaxed);
    __cpu_model
        .subtype
        .store(detected.subtype, Ordering::Relaxed);
    __cpu_model.features[0].store(detected.features[0], Ordering::Relaxed);
    for (destination, value) in __cpu_features2.iter().zip(detected.features[1..].iter()) {
        destination.store(*value, Ordering::Relaxed);
    }
    __cpu_model.vendor.store(detected.vendor, Ordering::Release);
    0
}

core::arch::global_asm!(
    ".hidden __cpu_model",
    ".hidden __cpu_features2",
    ".hidden __cpu_indicator_init",
);

#[used]
#[link_section = ".init_array.90"]
static CPU_INDICATOR_CONSTRUCTOR: extern "C" fn() -> i32 = __cpu_indicator_init;

const _: () = assert!(core::mem::size_of::<ProcessorModel>() == 16);
const _: () = assert!(core::mem::size_of::<[AtomicU32; 3]>() == 12);
const _: () = assert!(feature::CPU_FEATURE_MAX.div_ceil(32) == 4);

fn detect_cpu(hardware: &impl Hardware) -> Option<DetectedCpu> {
    let leaf0 = hardware.cpuid(0, 0);
    if leaf0.eax < 1 {
        return None;
    }

    let leaf1 = hardware.cpuid(1, 0);
    let (family, model) = detect_family_model(leaf1.eax);
    let features = available_features(hardware, leaf1.ecx, leaf1.edx, leaf0.eax);

    let (vendor, cpu_type, subtype) = match leaf0.ebx {
        SIG_INTEL => {
            let (cpu_type, subtype) = intel_processor_type(family, model, &features);
            (VENDOR_INTEL, cpu_type, subtype)
        }
        SIG_AMD => {
            let (cpu_type, subtype) = amd_processor_type(family, model, &features);
            (VENDOR_AMD, cpu_type, subtype)
        }
        _ => (VENDOR_OTHER, 0, 0),
    };

    Some(DetectedCpu {
        vendor,
        cpu_type,
        subtype,
        features,
    })
}

fn detect_family_model(eax: u32) -> (u32, u32) {
    let mut family = (eax >> 8) & 0xf;
    let mut model = (eax >> 4) & 0xf;
    if family == 6 || family == 0xf {
        if family == 0xf {
            family += (eax >> 20) & 0xff;
        }
        model += ((eax >> 16) & 0xf) << 4;
    }
    (family, model)
}

fn intel_processor_type(family: u32, model: u32, features: &[u32; 4]) -> (u32, u32) {
    use cpu_subtype::*;
    use cpu_type::*;

    if family != 6 {
        return (0, 0);
    }

    match model {
        0x0f | 0x16 | 0x17 | 0x1d => (INTEL_CORE2, 0),
        0x1a | 0x1e | 0x1f | 0x2e => (INTEL_COREI7, INTEL_COREI7_NEHALEM),
        0x25 | 0x2c | 0x2f => (INTEL_COREI7, INTEL_COREI7_WESTMERE),
        0x2a | 0x2d => (INTEL_COREI7, INTEL_COREI7_SANDYBRIDGE),
        0x3a | 0x3e => (INTEL_COREI7, INTEL_COREI7_IVYBRIDGE),
        0x3c | 0x3f | 0x45 | 0x46 => (INTEL_COREI7, INTEL_COREI7_HASWELL),
        0x3d | 0x47 | 0x4f | 0x56 => (INTEL_COREI7, INTEL_COREI7_BROADWELL),
        0x4e | 0x5e | 0x8e | 0x9e | 0xa5 | 0xa6 => (INTEL_COREI7, INTEL_COREI7_SKYLAKE),
        0xa7 => (INTEL_COREI7, INTEL_COREI7_ROCKETLAKE),
        0x55 => {
            let subtype = if has_feature(features, feature::AVX512BF16) {
                INTEL_COREI7_COOPERLAKE
            } else if has_feature(features, feature::AVX512VNNI) {
                INTEL_COREI7_CASCADELAKE
            } else {
                INTEL_COREI7_SKYLAKE_AVX512
            };
            (INTEL_COREI7, subtype)
        }
        0x66 => (INTEL_COREI7, INTEL_COREI7_CANNONLAKE),
        0x7d | 0x7e => (INTEL_COREI7, INTEL_COREI7_ICELAKE_CLIENT),
        0x8c | 0x8d => (INTEL_COREI7, INTEL_COREI7_TIGERLAKE),
        0x97 | 0x9a | 0xb7 | 0xba | 0xbf | 0xaa | 0xac | 0xbe => {
            (INTEL_COREI7, INTEL_COREI7_ALDERLAKE)
        }
        0xc5 => (INTEL_COREI7, INTEL_COREI7_ARROWLAKE),
        0xc6 | 0xbd => (INTEL_COREI7, INTEL_COREI7_ARROWLAKE_S),
        0xcc => (INTEL_COREI7, INTEL_COREI7_PANTHERLAKE),
        0x6a | 0x6c => (INTEL_COREI7, INTEL_COREI7_ICELAKE_SERVER),
        0xcf | 0x8f => (INTEL_COREI7, INTEL_COREI7_SAPPHIRERAPIDS),
        0xad => (INTEL_COREI7, INTEL_COREI7_GRANITERAPIDS),
        0xae => (INTEL_COREI7, INTEL_COREI7_GRANITERAPIDS_D),
        0x1c | 0x26 | 0x27 | 0x35 | 0x36 => (INTEL_BONNELL, 0),
        0x37 | 0x4a | 0x4d | 0x5a | 0x5d | 0x4c => (INTEL_SILVERMONT, 0),
        0x5c | 0x5f => (INTEL_GOLDMONT, 0),
        0x7a => (INTEL_GOLDMONT_PLUS, 0),
        0x86 | 0x8a | 0x96 | 0x9c => (INTEL_TREMONT, 0),
        0xaf => (INTEL_SIERRAFOREST, 0),
        0xb6 => (INTEL_GRANDRIDGE, 0),
        // This unusual value is intentional: it matches compiler-rt 19.1.7.
        0xdd => (INTEL_COREI7, INTEL_CLEARWATERFOREST),
        0x57 => (INTEL_KNL, 0),
        0x85 => (INTEL_KNM, 0),
        _ => (0, 0),
    }
}

fn amd_processor_type(family: u32, model: u32, features: &[u32; 4]) -> (u32, u32) {
    use cpu_subtype::*;
    use cpu_type::*;

    match family {
        16 => {
            let subtype = match model {
                2 => AMDFAM10H_BARCELONA,
                4 => AMDFAM10H_SHANGHAI,
                8 => AMDFAM10H_ISTANBUL,
                _ => 0,
            };
            (AMDFAM10H, subtype)
        }
        20 => (AMD_BTVER1, 0),
        21 => {
            let subtype = if (0x60..=0x7f).contains(&model) {
                AMDFAM15H_BDVER4
            } else if (0x30..=0x3f).contains(&model) {
                AMDFAM15H_BDVER3
            } else if (0x10..=0x1f).contains(&model) || model == 0x02 {
                AMDFAM15H_BDVER2
            } else if model <= 0x0f {
                AMDFAM15H_BDVER1
            } else {
                0
            };
            (AMDFAM15H, subtype)
        }
        22 => (AMD_BTVER2, 0),
        23 => {
            let subtype = if (0x30..=0x3f).contains(&model)
                || model == 0x47
                || (0x60..=0x67).contains(&model)
                || (0x68..=0x6f).contains(&model)
                || (0x70..=0x7f).contains(&model)
                || (0x84..=0x87).contains(&model)
                || (0x90..=0x97).contains(&model)
                || (0x98..=0x9f).contains(&model)
                || (0xa0..=0xaf).contains(&model)
            {
                AMDFAM17H_ZNVER2
            } else if (0x10..=0x1f).contains(&model) || (0x20..=0x2f).contains(&model) {
                AMDFAM17H_ZNVER1
            } else {
                0
            };
            (AMDFAM17H, subtype)
        }
        25 => {
            let subtype = if model <= 0x0f
                || (0x20..=0x2f).contains(&model)
                || (0x30..=0x3f).contains(&model)
                || (0x40..=0x4f).contains(&model)
                || (0x50..=0x5f).contains(&model)
            {
                AMDFAM19H_ZNVER3
            } else if (0x10..=0x1f).contains(&model)
                || (0x60..=0x6f).contains(&model)
                || (0x70..=0x77).contains(&model)
                || (0x78..=0x7f).contains(&model)
                || (0xa0..=0xaf).contains(&model)
            {
                AMDFAM19H_ZNVER4
            } else {
                0
            };
            (AMDFAM19H, subtype)
        }
        26 => {
            let subtype = if model <= 0x77 { AMDFAM1AH_ZNVER5 } else { 0 };
            (AMDFAM1AH, subtype)
        }
        // Families 4, 5, 6, and 15 have CPU names in LLVM Host.cpp but do
        // not have compiler-rt ABI type/subtype values.
        _ => {
            let _ = features;
            (0, 0)
        }
    }
}

fn available_features(
    hardware: &impl Hardware,
    leaf1_ecx: u32,
    leaf1_edx: u32,
    max_leaf: u32,
) -> [u32; 4] {
    use feature::*;

    let mut features = [0_u32; 4];

    set_if(&mut features, bit(leaf1_edx, 15), CMOV);
    set_if(&mut features, bit(leaf1_edx, 23), MMX);
    set_if(&mut features, bit(leaf1_edx, 25), SSE);
    set_if(&mut features, bit(leaf1_edx, 26), SSE2);

    set_if(&mut features, bit(leaf1_ecx, 0), SSE3);
    set_if(&mut features, bit(leaf1_ecx, 1), PCLMUL);
    set_if(&mut features, bit(leaf1_ecx, 9), SSSE3);
    set_if(&mut features, bit(leaf1_ecx, 12), FMA);
    set_if(&mut features, bit(leaf1_ecx, 13), CMPXCHG16B);
    set_if(&mut features, bit(leaf1_ecx, 19), SSE4_1);
    set_if(&mut features, bit(leaf1_ecx, 20), SSE4_2);
    set_if(&mut features, bit(leaf1_ecx, 22), MOVBE);
    set_if(&mut features, bit(leaf1_ecx, 23), POPCNT);
    set_if(&mut features, bit(leaf1_ecx, 25), AES);
    set_if(&mut features, bit(leaf1_ecx, 29), F16C);
    set_if(&mut features, bit(leaf1_ecx, 30), RDRND);

    let has_avx_save = leaf1_ecx & ((1 << 27) | (1 << 28)) == ((1 << 27) | (1 << 28))
        && hardware.xcr0() & 0x6 == 0x6;
    let xcr0 = if bit(leaf1_ecx, 27) {
        hardware.xcr0()
    } else {
        0
    };
    let has_avx512_save = has_avx_save && xcr0 & 0xe0 == 0xe0;
    let has_amx_save = bit(leaf1_ecx, 27)
        && xcr0 & ((1 << 17) | (1 << 18)) != 0
        && xcr0 & ((1 << 17) | (1 << 18)) == ((1 << 17) | (1 << 18));

    set_if(&mut features, has_avx_save, AVX);
    set_if(&mut features, bit(leaf1_ecx, 26) && has_avx_save, XSAVE);

    let leaf7 = (max_leaf >= 7).then(|| hardware.cpuid(7, 0));
    if let Some(regs) = leaf7 {
        set_if(&mut features, bit(regs.ebx, 0), FSGSBASE);
        set_if(&mut features, bit(regs.ebx, 2), SGX);
        set_if(&mut features, bit(regs.ebx, 3), BMI);
        set_if(&mut features, bit(regs.ebx, 5) && has_avx_save, AVX2);
        set_if(&mut features, bit(regs.ebx, 8), BMI2);
        set_if(&mut features, bit(regs.ebx, 11), RTM);
        set_if(&mut features, bit(regs.ebx, 16) && has_avx512_save, AVX512F);
        set_if(
            &mut features,
            bit(regs.ebx, 17) && has_avx512_save,
            AVX512DQ,
        );
        set_if(&mut features, bit(regs.ebx, 18), RDSEED);
        set_if(&mut features, bit(regs.ebx, 19), ADX);
        set_if(
            &mut features,
            bit(regs.ebx, 21) && has_avx512_save,
            AVX512IFMA,
        );
        set_if(&mut features, bit(regs.ebx, 24), CLWB);
        set_if(
            &mut features,
            bit(regs.ebx, 26) && has_avx512_save,
            AVX512PF,
        );
        set_if(
            &mut features,
            bit(regs.ebx, 27) && has_avx512_save,
            AVX512ER,
        );
        set_if(
            &mut features,
            bit(regs.ebx, 28) && has_avx512_save,
            AVX512CD,
        );
        set_if(&mut features, bit(regs.ebx, 29), SHA);
        set_if(
            &mut features,
            bit(regs.ebx, 30) && has_avx512_save,
            AVX512BW,
        );
        set_if(
            &mut features,
            bit(regs.ebx, 31) && has_avx512_save,
            AVX512VL,
        );

        set_if(&mut features, bit(regs.ecx, 0), PREFETCHWT1);
        set_if(
            &mut features,
            bit(regs.ecx, 1) && has_avx512_save,
            AVX512VBMI,
        );
        set_if(&mut features, bit(regs.ecx, 4), PKU);
        set_if(&mut features, bit(regs.ecx, 5), WAITPKG);
        set_if(
            &mut features,
            bit(regs.ecx, 6) && has_avx512_save,
            AVX512VBMI2,
        );
        set_if(&mut features, bit(regs.ecx, 7), SHSTK);
        set_if(&mut features, bit(regs.ecx, 8), GFNI);
        set_if(&mut features, bit(regs.ecx, 9) && has_avx_save, VAES);
        set_if(&mut features, bit(regs.ecx, 10) && has_avx_save, VPCLMULQDQ);
        set_if(
            &mut features,
            bit(regs.ecx, 11) && has_avx512_save,
            AVX512VNNI,
        );
        set_if(
            &mut features,
            bit(regs.ecx, 12) && has_avx512_save,
            AVX512BITALG,
        );
        set_if(
            &mut features,
            bit(regs.ecx, 14) && has_avx512_save,
            AVX512VPOPCNTDQ,
        );
        set_if(&mut features, bit(regs.ecx, 22), RDPID);
        set_if(&mut features, bit(regs.ecx, 23), KL);
        set_if(&mut features, bit(regs.ecx, 25), CLDEMOTE);
        set_if(&mut features, bit(regs.ecx, 27), MOVDIRI);
        set_if(&mut features, bit(regs.ecx, 28), MOVDIR64B);
        set_if(&mut features, bit(regs.ecx, 29), ENQCMD);

        set_if(
            &mut features,
            bit(regs.edx, 2) && has_avx512_save,
            AVX5124VNNIW,
        );
        set_if(
            &mut features,
            bit(regs.edx, 3) && has_avx512_save,
            AVX5124FMAPS,
        );
        set_if(&mut features, bit(regs.edx, 5), UINTR);
        set_if(
            &mut features,
            bit(regs.edx, 8) && has_avx512_save,
            AVX512VP2INTERSECT,
        );
        set_if(&mut features, bit(regs.edx, 14), SERIALIZE);
        set_if(&mut features, bit(regs.edx, 16), TSXLDTRK);
        set_if(&mut features, bit(regs.edx, 18), PCONFIG);
        set_if(&mut features, bit(regs.edx, 22) && has_amx_save, AMX_BF16);
        set_if(
            &mut features,
            bit(regs.edx, 23) && has_avx512_save,
            AVX512FP16,
        );
        set_if(&mut features, bit(regs.edx, 24) && has_amx_save, AMX_TILE);
        set_if(&mut features, bit(regs.edx, 25) && has_amx_save, AMX_INT8);
    }

    let leaf7_subleaf1 = leaf7
        .filter(|regs| regs.eax >= 1)
        .map(|_| hardware.cpuid(7, 1));
    if let Some(regs) = leaf7_subleaf1 {
        set_if(&mut features, bit(regs.eax, 0), SHA512);
        set_if(&mut features, bit(regs.eax, 1), SM3);
        set_if(&mut features, bit(regs.eax, 2), SM4);
        set_if(&mut features, bit(regs.eax, 3), RAOINT);
        set_if(&mut features, bit(regs.eax, 4) && has_avx_save, AVXVNNI);
        set_if(
            &mut features,
            bit(regs.eax, 5) && has_avx512_save,
            AVX512BF16,
        );
        set_if(&mut features, bit(regs.eax, 7), CMPCCXADD);
        set_if(&mut features, bit(regs.eax, 21) && has_amx_save, AMX_FP16);
        set_if(&mut features, bit(regs.eax, 22), HRESET);
        set_if(&mut features, bit(regs.eax, 23) && has_avx_save, AVXIFMA);

        set_if(&mut features, bit(regs.edx, 4) && has_avx_save, AVXVNNIINT8);
        set_if(
            &mut features,
            bit(regs.edx, 5) && has_avx_save,
            AVXNECONVERT,
        );
        set_if(&mut features, bit(regs.edx, 8) && has_amx_save, AMX_COMPLEX);
        set_if(
            &mut features,
            bit(regs.edx, 10) && has_avx_save,
            AVXVNNIINT16,
        );
        set_if(&mut features, bit(regs.edx, 14), PREFETCHI);
        set_if(&mut features, bit(regs.edx, 15), USERMSR);
        set_if(&mut features, bit(regs.edx, 19), AVX10_1_256);
        set_if(&mut features, bit(regs.edx, 21), APXF);
    }

    let max_level = hardware.cpuid(0, 0).eax;
    if max_level >= 0xd {
        let regs = hardware.cpuid(0xd, 1);
        set_if(&mut features, bit(regs.eax, 0) && has_avx_save, XSAVEOPT);
        set_if(&mut features, bit(regs.eax, 1) && has_avx_save, XSAVEC);
        set_if(&mut features, bit(regs.eax, 3) && has_avx_save, XSAVES);
    }

    let leaf24 = (max_level >= 0x24).then(|| hardware.cpuid(0x24, 0));
    // This intentionally uses leaf 24 EDX, matching compiler-rt 19.1.7's
    // variable-reuse semantics.
    if leaf7_subleaf1.is_some()
        && leaf24.is_some_and(|regs| bit(regs.edx, 19))
        && leaf24.is_some_and(|regs| bit(regs.ebx, 18))
    {
        set_feature(&mut features, AVX10_1_512);
    }

    let max_ext_level = hardware.cpuid(0x8000_0000, 0).eax;
    if max_ext_level >= 0x8000_0001 {
        let regs = hardware.cpuid(0x8000_0001, 0);
        set_if(&mut features, bit(regs.ecx, 0), LAHF_LM);
        set_if(&mut features, bit(regs.ecx, 5), LZCNT);
        set_if(&mut features, bit(regs.ecx, 6), SSE4_A);
        set_if(&mut features, bit(regs.ecx, 8), PRFCHW);
        set_if(&mut features, bit(regs.ecx, 11), XOP);
        set_if(&mut features, bit(regs.ecx, 15), LWP);
        set_if(&mut features, bit(regs.ecx, 16), FMA4);
        set_if(&mut features, bit(regs.ecx, 21), TBM);
        set_if(&mut features, bit(regs.ecx, 29), MWAITX);
        set_if(&mut features, bit(regs.edx, 29), LM);
    }

    if max_ext_level >= 0x8000_0008 {
        let regs = hardware.cpuid(0x8000_0008, 0);
        set_if(&mut features, bit(regs.ebx, 0), CLZERO);
        set_if(&mut features, bit(regs.ebx, 9), WBNOINVD);
    }

    if max_level >= 0x14 {
        let regs = hardware.cpuid(0x14, 0);
        set_if(&mut features, bit(regs.ebx, 4), PTWRITE);
    }

    if leaf7.is_some() && max_level >= 0x19 {
        let regs = hardware.cpuid(0x19, 0);
        set_if(&mut features, bit(regs.ebx, 2), WIDEKL);
    }

    if has_feature(&features, LM) && has_feature(&features, SSE2) {
        set_feature(&mut features, X86_64_BASELINE);
        if has_feature(&features, CMPXCHG16B)
            && has_feature(&features, POPCNT)
            && has_feature(&features, LAHF_LM)
            && has_feature(&features, SSE4_2)
        {
            set_feature(&mut features, X86_64_V2);
            if has_feature(&features, AVX2)
                && has_feature(&features, BMI)
                && has_feature(&features, BMI2)
                && has_feature(&features, F16C)
                && has_feature(&features, FMA)
                && has_feature(&features, LZCNT)
                && has_feature(&features, MOVBE)
            {
                set_feature(&mut features, X86_64_V3);
                if has_feature(&features, AVX512BW)
                    && has_feature(&features, AVX512CD)
                    && has_feature(&features, AVX512DQ)
                    && has_feature(&features, AVX512VL)
                {
                    set_feature(&mut features, X86_64_V4);
                }
            }
        }
    }

    features
}

#[inline]
const fn bit(value: u32, bit: u32) -> bool {
    value & (1 << bit) != 0
}

#[inline]
fn set_if(features: &mut [u32; 4], condition: bool, feature: usize) {
    if condition {
        set_feature(features, feature);
    }
}

#[inline]
fn set_feature(features: &mut [u32; 4], feature: usize) {
    features[feature / 32] |= 1_u32 << (feature % 32);
}

#[inline]
fn has_feature(features: &[u32; 4], feature: usize) -> bool {
    features[feature / 32] & (1_u32 << (feature % 32)) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Leaf {
        leaf: u32,
        subleaf: u32,
        registers: Registers,
    }

    struct MockHardware<'a> {
        leaves: &'a [Leaf],
        xcr0: u64,
    }

    impl Hardware for MockHardware<'_> {
        fn cpuid(&self, leaf: u32, subleaf: u32) -> Registers {
            self.leaves
                .iter()
                .find(|entry| entry.leaf == leaf && entry.subleaf == subleaf)
                .map_or_else(Registers::default, |entry| entry.registers)
        }

        fn xcr0(&self) -> u64 {
            self.xcr0
        }
    }

    const FULL_LEAVES: &[Leaf] = &[
        Leaf {
            leaf: 0,
            subleaf: 0,
            registers: Registers {
                eax: 0x24,
                ebx: SIG_INTEL,
                ecx: 0,
                edx: 0,
            },
        },
        Leaf {
            leaf: 1,
            subleaf: 0,
            registers: Registers {
                eax: 0,
                ebx: 0,
                ecx: u32::MAX,
                edx: u32::MAX,
            },
        },
        Leaf {
            leaf: 7,
            subleaf: 0,
            registers: Registers {
                eax: 1,
                ebx: u32::MAX,
                ecx: u32::MAX,
                edx: u32::MAX,
            },
        },
        Leaf {
            leaf: 7,
            subleaf: 1,
            registers: Registers {
                eax: u32::MAX,
                ebx: u32::MAX,
                ecx: u32::MAX,
                edx: u32::MAX,
            },
        },
        Leaf {
            leaf: 0xd,
            subleaf: 1,
            registers: Registers {
                eax: u32::MAX,
                ebx: 0,
                ecx: 0,
                edx: 0,
            },
        },
        Leaf {
            leaf: 0x14,
            subleaf: 0,
            registers: Registers {
                eax: 0,
                ebx: u32::MAX,
                ecx: 0,
                edx: 0,
            },
        },
        Leaf {
            leaf: 0x19,
            subleaf: 0,
            registers: Registers {
                eax: 0,
                ebx: u32::MAX,
                ecx: 0,
                edx: 0,
            },
        },
        Leaf {
            leaf: 0x24,
            subleaf: 0,
            registers: Registers {
                eax: 0,
                ebx: u32::MAX,
                ecx: 0,
                edx: u32::MAX,
            },
        },
        Leaf {
            leaf: 0x8000_0000,
            subleaf: 0,
            registers: Registers {
                eax: 0x8000_0008,
                ebx: 0,
                ecx: 0,
                edx: 0,
            },
        },
        Leaf {
            leaf: 0x8000_0001,
            subleaf: 0,
            registers: Registers {
                eax: 0,
                ebx: 0,
                ecx: u32::MAX,
                edx: u32::MAX,
            },
        },
        Leaf {
            leaf: 0x8000_0008,
            subleaf: 0,
            registers: Registers {
                eax: 0,
                ebx: u32::MAX,
                ecx: 0,
                edx: 0,
            },
        },
    ];

    #[test]
    fn abi_layout_matches_compiler_rt() {
        assert_eq!(core::mem::size_of::<ProcessorModel>(), 16);
        assert_eq!(core::mem::align_of::<ProcessorModel>(), 4);
        assert_eq!(core::mem::size_of_val(&__cpu_features2), 12);
        assert_eq!(core::mem::align_of_val(&__cpu_features2), 4);
    }

    #[test]
    fn native_initializer_populates_a_valid_model() {
        assert_eq!(__cpu_indicator_init(), 0);
        let vendor = __cpu_model.vendor.load(Ordering::Acquire);
        let features = [
            __cpu_model.features[0].load(Ordering::Relaxed),
            __cpu_features2[0].load(Ordering::Relaxed),
            __cpu_features2[1].load(Ordering::Relaxed),
            __cpu_features2[2].load(Ordering::Relaxed),
        ];
        assert!((VENDOR_INTEL..=VENDOR_OTHER).contains(&vendor));
        assert!(has_feature(&features, feature::SSE2));
        assert!(has_feature(&features, feature::X86_64_BASELINE));
    }

    #[test]
    fn family_and_model_decode_extended_fields() {
        assert_eq!(detect_family_model(6 << 8 | 0xa << 4 | 3 << 16), (6, 0x3a));
        assert_eq!(
            detect_family_model(0xf << 8 | 1 << 4 | 8 << 16 | 8 << 20),
            (23, 0x81)
        );
        assert_eq!(detect_family_model(5 << 8 | 9 << 4 | 7 << 16), (5, 9));
    }

    #[test]
    fn intel_model_table_matches_compiler_rt() {
        use cpu_subtype::*;
        use cpu_type::*;

        let empty = [0; 4];
        let cases = [
            (0x0f, (INTEL_CORE2, 0)),
            (0x1a, (INTEL_COREI7, INTEL_COREI7_NEHALEM)),
            (0x25, (INTEL_COREI7, INTEL_COREI7_WESTMERE)),
            (0x2a, (INTEL_COREI7, INTEL_COREI7_SANDYBRIDGE)),
            (0x3a, (INTEL_COREI7, INTEL_COREI7_IVYBRIDGE)),
            (0x3c, (INTEL_COREI7, INTEL_COREI7_HASWELL)),
            (0x3d, (INTEL_COREI7, INTEL_COREI7_BROADWELL)),
            (0x4e, (INTEL_COREI7, INTEL_COREI7_SKYLAKE)),
            (0xa7, (INTEL_COREI7, INTEL_COREI7_ROCKETLAKE)),
            (0x66, (INTEL_COREI7, INTEL_COREI7_CANNONLAKE)),
            (0x7d, (INTEL_COREI7, INTEL_COREI7_ICELAKE_CLIENT)),
            (0x8c, (INTEL_COREI7, INTEL_COREI7_TIGERLAKE)),
            (0x97, (INTEL_COREI7, INTEL_COREI7_ALDERLAKE)),
            (0xc5, (INTEL_COREI7, INTEL_COREI7_ARROWLAKE)),
            (0xbd, (INTEL_COREI7, INTEL_COREI7_ARROWLAKE_S)),
            (0xcc, (INTEL_COREI7, INTEL_COREI7_PANTHERLAKE)),
            (0x6a, (INTEL_COREI7, INTEL_COREI7_ICELAKE_SERVER)),
            (0x8f, (INTEL_COREI7, INTEL_COREI7_SAPPHIRERAPIDS)),
            (0xad, (INTEL_COREI7, INTEL_COREI7_GRANITERAPIDS)),
            (0xae, (INTEL_COREI7, INTEL_COREI7_GRANITERAPIDS_D)),
            (0x1c, (INTEL_BONNELL, 0)),
            (0x37, (INTEL_SILVERMONT, 0)),
            (0x5c, (INTEL_GOLDMONT, 0)),
            (0x7a, (INTEL_GOLDMONT_PLUS, 0)),
            (0x86, (INTEL_TREMONT, 0)),
            (0xaf, (INTEL_SIERRAFOREST, 0)),
            (0xb6, (INTEL_GRANDRIDGE, 0)),
            (0xdd, (INTEL_COREI7, INTEL_CLEARWATERFOREST)),
            (0x57, (INTEL_KNL, 0)),
            (0x85, (INTEL_KNM, 0)),
            (0xffff, (0, 0)),
        ];

        for (model, expected) in cases {
            assert_eq!(intel_processor_type(6, model, &empty), expected);
        }
        assert_eq!(intel_processor_type(5, 0x3c, &empty), (0, 0));
    }

    #[test]
    fn skylake_server_subtype_uses_feature_bits() {
        use cpu_subtype::*;

        let mut features = [0; 4];
        assert_eq!(
            intel_processor_type(6, 0x55, &features).1,
            INTEL_COREI7_SKYLAKE_AVX512
        );
        set_feature(&mut features, feature::AVX512VNNI);
        assert_eq!(
            intel_processor_type(6, 0x55, &features).1,
            INTEL_COREI7_CASCADELAKE
        );
        set_feature(&mut features, feature::AVX512BF16);
        assert_eq!(
            intel_processor_type(6, 0x55, &features).1,
            INTEL_COREI7_COOPERLAKE
        );
    }

    #[test]
    fn amd_family_and_model_table_matches_compiler_rt() {
        use cpu_subtype::*;
        use cpu_type::*;

        let features = [0; 4];
        let cases = [
            ((16, 2), (AMDFAM10H, AMDFAM10H_BARCELONA)),
            ((16, 4), (AMDFAM10H, AMDFAM10H_SHANGHAI)),
            ((16, 8), (AMDFAM10H, AMDFAM10H_ISTANBUL)),
            ((20, 0), (AMD_BTVER1, 0)),
            ((21, 0x01), (AMDFAM15H, AMDFAM15H_BDVER1)),
            ((21, 0x02), (AMDFAM15H, AMDFAM15H_BDVER2)),
            ((21, 0x30), (AMDFAM15H, AMDFAM15H_BDVER3)),
            ((21, 0x60), (AMDFAM15H, AMDFAM15H_BDVER4)),
            ((22, 0), (AMD_BTVER2, 0)),
            ((23, 0x10), (AMDFAM17H, AMDFAM17H_ZNVER1)),
            ((23, 0x30), (AMDFAM17H, AMDFAM17H_ZNVER2)),
            ((25, 0x20), (AMDFAM19H, AMDFAM19H_ZNVER3)),
            ((25, 0x60), (AMDFAM19H, AMDFAM19H_ZNVER4)),
            ((26, 0x70), (AMDFAM1AH, AMDFAM1AH_ZNVER5)),
            ((26, 0x78), (AMDFAM1AH, 0)),
            ((6, 0), (0, 0)),
        ];

        for ((family, model), expected) in cases {
            assert_eq!(
                amd_processor_type(family, model, &features),
                expected,
                "family={family:#x} model={model:#x}"
            );
        }
    }

    #[test]
    fn every_compiler_rt_feature_mapping_is_populated() {
        use feature::*;

        let hardware = MockHardware {
            leaves: FULL_LEAVES,
            xcr0: 0x6 | 0xe0 | (1 << 17) | (1 << 18),
        };
        let actual = available_features(&hardware, u32::MAX, u32::MAX, 0x24);
        let mut expected = [0; 4];
        let populated = [
            CMOV,
            MMX,
            POPCNT,
            SSE,
            SSE2,
            SSE3,
            SSSE3,
            SSE4_1,
            SSE4_2,
            AVX,
            AVX2,
            SSE4_A,
            FMA4,
            XOP,
            FMA,
            AVX512F,
            BMI,
            BMI2,
            AES,
            PCLMUL,
            AVX512VL,
            AVX512BW,
            AVX512DQ,
            AVX512CD,
            AVX512ER,
            AVX512PF,
            AVX512VBMI,
            AVX512IFMA,
            AVX5124VNNIW,
            AVX5124FMAPS,
            AVX512VPOPCNTDQ,
            AVX512VBMI2,
            GFNI,
            VPCLMULQDQ,
            AVX512VNNI,
            AVX512BITALG,
            AVX512BF16,
            AVX512VP2INTERSECT,
            ADX,
            CLDEMOTE,
            CLWB,
            CLZERO,
            CMPXCHG16B,
            ENQCMD,
            F16C,
            FSGSBASE,
            LAHF_LM,
            LM,
            LWP,
            LZCNT,
            MOVBE,
            MOVDIR64B,
            MOVDIRI,
            MWAITX,
            PCONFIG,
            PKU,
            PREFETCHWT1,
            PRFCHW,
            PTWRITE,
            RDPID,
            RDRND,
            RDSEED,
            RTM,
            SERIALIZE,
            SGX,
            SHA,
            SHSTK,
            TBM,
            TSXLDTRK,
            VAES,
            WAITPKG,
            WBNOINVD,
            XSAVE,
            XSAVEC,
            XSAVEOPT,
            XSAVES,
            AMX_TILE,
            AMX_INT8,
            AMX_BF16,
            UINTR,
            HRESET,
            KL,
            WIDEKL,
            AVXVNNI,
            AVX512FP16,
            X86_64_BASELINE,
            X86_64_V2,
            X86_64_V3,
            X86_64_V4,
            AVXIFMA,
            AVXVNNIINT8,
            AVXNECONVERT,
            CMPCCXADD,
            AMX_FP16,
            PREFETCHI,
            RAOINT,
            AMX_COMPLEX,
            AVXVNNIINT16,
            SM3,
            SHA512,
            SM4,
            APXF,
            USERMSR,
            AVX10_1_256,
            AVX10_1_512,
        ];
        for feature in populated {
            set_feature(&mut expected, feature);
        }

        assert_eq!(actual, expected);
        assert!(!has_feature(&actual, CLFLUSHOPT));
    }

    #[test]
    fn os_context_state_gates_avx_avx512_and_amx() {
        use feature::*;

        let hardware = MockHardware {
            leaves: FULL_LEAVES,
            xcr0: 0,
        };
        let features = available_features(&hardware, u32::MAX, u32::MAX, 0x24);

        for gated in [
            AVX,
            AVX2,
            AVX512F,
            AVX512BW,
            AVX512BF16,
            VAES,
            VPCLMULQDQ,
            XSAVE,
            XSAVEC,
            XSAVEOPT,
            XSAVES,
            AMX_TILE,
            AMX_INT8,
            AMX_BF16,
            AMX_FP16,
            AMX_COMPLEX,
        ] {
            assert!(!has_feature(&features, gated), "feature {gated} was set");
        }

        // These compiler-rt bits report CPUID capability without XCR0 gating.
        assert!(has_feature(&features, FMA));
        assert!(has_feature(&features, F16C));
    }
}
