use rand_chacha::{rand_core::SeedableRng, ChaCha20Rng};
use raw_cpuid::CpuId;
use x86_64::instructions::{port::Port, random::RdRand};

/// Gather entropy from various sources to seed a RNG.
pub fn build_rng() -> ChaCha20Rng {
    const ENTROPY_SOURCES: [fn() -> [u8; 32]; 3] = [rd_rand_entropy, tsc_entropy, pit_entropy];

    // Collect entropy from different sources and xor them all together.
    let mut seed = [0; 32];
    for entropy_source in ENTROPY_SOURCES {
        let entropy = entropy_source();

        for (seed, entropy) in seed.iter_mut().zip(entropy) {
            *seed ^= entropy;
        }
    }

    // Construct the RNG.
    ChaCha20Rng::from_seed(seed)
}

/// Gather entropy by requesting random numbers with `RDRAND` instruction if it's available.
///
/// This function provides excellent entropy (unless you don't trust the CPU vendors).
fn rd_rand_entropy() -> [u8; 32] {
    let mut entropy = [0; 32];

    // Check if the CPU supports `RDRAND`.
    if let Some(rd_rand) = RdRand::new() {
        for i in 0..4 {
            let value = loop {
                if let Some(value) = rd_rand.get_u64() {
                    break value;
                }
            };
            entropy[i * 8..(i + 1) * 8].copy_from_slice(&value.to_ne_bytes());
        }
    }

    entropy
}

/// Gather entropy by reading the current time with the `RDTSC` instruction if it's available.
///
/// This function doesn't provide particulary good entropy, but it's better than nothing.
fn tsc_entropy() -> [u8; 32] {
    let mut entropy = [0; 32];

    // Check if the CPU supports `RDTSC`.
    let cpu_id = CpuId::new();
    if let Some(feature_info) = cpu_id.get_feature_info() {
        if !feature_info.has_tsc() {
            for i in 0..4 {
                let value = unsafe {
                    // SAFETY: We checked that the cpu supports `RDTSC` and we run in ring 0.
                    core::arch::x86_64::_rdtsc()
                };
                entropy[i * 8..(i + 1) * 8].copy_from_slice(&value.to_ne_bytes());
            }
        }
    }

    entropy
}

/// Gather entropy by reading the current count of PIT channel 1-3.
///
/// This function doesn't provide particulary good entropy, but it's always available.
fn pit_entropy() -> [u8; 32] {
    let mut entropy = [0; 32];

    for (i, entropy_byte) in entropy.iter_mut().enumerate() {
        // Cycle through channels 1-3.
        let channel = i % 3;

        let mut port = Port::<u8>::new(0x40 + channel as u16);
        let value = unsafe {
            // SAFETY: It's safe to read from ports 0x40-0x42.
            port.read()
        };

        *entropy_byte = value;
    }

    entropy
}
