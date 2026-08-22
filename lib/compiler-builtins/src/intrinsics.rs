#[unsafe(no_mangle)]
pub extern "C" fn __udivdi3(a: u64, b: u64) -> u64 {
    let (quot, _) = udivmod64(a, b);
    quot
}

#[unsafe(no_mangle)]
pub extern "C" fn __umoddi3(a: u64, b: u64) -> u64 {
    let (_, rem) = udivmod64(a, b);
    rem
}

#[inline(always)]
fn udivmod64(mut numerator: u64, mut denominator: u64) -> (u64, u64) {
    if denominator == 0 {
        panic!("division by zero");
    }

    if numerator < denominator {
        return (0, numerator);
    }

    if denominator == 1 {
        return (numerator, 0);
    }

    // Fast path: if both fit in 32-bit, use hardware 32-bit arithmetic
    // to avoid the 64-step bitwise loop.
    if (numerator >> 32) == 0 && (denominator >> 32) == 0 {
        let n = numerator as u32;
        let d = denominator as u32;
        return ((n / d) as u64, (n % d) as u64);
    }

    // Count leading zeros to shift denominator up for long division
    let shift = denominator.leading_zeros() - numerator.leading_zeros();
    denominator <<= shift;

    let mut quotient: u64 = 0;

    for _ in 0..=shift {
        quotient <<= 1;
        if numerator >= denominator {
            numerator -= denominator;
            quotient |= 1;
        }
        denominator >>= 1;
    }

    (quotient, numerator)
}
