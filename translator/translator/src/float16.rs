pub(crate) fn f32_to_f16_bits(value: f32) -> u16 {
    let bits = value.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32;
    let significand = bits & 0x007f_ffff;

    if exponent == 0xff {
        if significand == 0 {
            return sign | 0x7c00;
        }
        let payload = (significand >> 13) as u16;
        return sign | 0x7c00 | payload | u16::from(payload == 0);
    }

    let half_exponent = exponent - 127 + 15;
    if half_exponent >= 31 {
        return sign | 0x7c00;
    }
    if half_exponent <= 0 {
        if half_exponent < -10 {
            return sign;
        }
        let mantissa = significand | 0x0080_0000;
        let shift = (14 - half_exponent) as u32;
        return sign | round_shift_right_ties_even(mantissa, shift) as u16;
    }

    let rounded = round_shift_right_ties_even(significand, 13);
    let encoded = ((half_exponent as u32) << 10) + rounded;
    sign | encoded.min(0x7c00) as u16
}

pub(crate) fn f16_bits_to_f32(bits: u16) -> f32 {
    let negative = bits & 0x8000 != 0;
    let exponent = ((bits >> 10) & 0x1f) as u32;
    let significand = (bits & 0x03ff) as u32;
    let magnitude = if exponent == 0 {
        (significand as f32) * (2.0f32).powi(-24)
    } else if exponent == 0x1f {
        if significand == 0 {
            f32::INFINITY
        } else {
            f32::from_bits(0x7f80_0000 | (significand << 13))
        }
    } else {
        f32::from_bits(((exponent + 112) << 23) | (significand << 13))
    };
    if negative {
        -magnitude
    } else {
        magnitude
    }
}

fn round_shift_right_ties_even(value: u32, shift: u32) -> u32 {
    let truncated = value >> shift;
    let remainder = value & ((1 << shift) - 1);
    let halfway = 1 << (shift - 1);
    truncated + u32::from(remainder > halfway || (remainder == halfway && truncated & 1 != 0))
}

#[cfg(test)]
mod tests {
    use super::{f16_bits_to_f32, f32_to_f16_bits};

    #[test]
    fn every_representable_half_round_trips() {
        for bits in 0u16..=u16::MAX {
            if (bits >> 10) & 0x1f == 0x1f {
                continue;
            }
            let widened = f16_bits_to_f32(bits);
            assert_eq!(
                f32_to_f16_bits(widened),
                bits,
                "0x{bits:04x} widened to {widened} did not encode back"
            );
        }
    }

    #[test]
    fn every_boundary_between_two_halves_rounds_correctly() {
        for bits in 0u16..0x7bff {
            let (low, high) = (f16_bits_to_f32(bits), f16_bits_to_f32(bits + 1));
            let midpoint = (low + high) * 0.5;
            assert!(
                low < midpoint && midpoint < high,
                "midpoint of 0x{bits:04x} and its successor is not exactly between them"
            );

            let even = if bits & 1 == 0 { bits } else { bits + 1 };
            assert_eq!(
                f32_to_f16_bits(midpoint),
                even,
                "the tie at {midpoint} between 0x{bits:04x} and 0x{:04x} must go to the even one",
                bits + 1
            );

            let below = f32::from_bits(midpoint.to_bits() - 1);
            let above = f32::from_bits(midpoint.to_bits() + 1);
            assert_eq!(
                f32_to_f16_bits(below),
                bits,
                "{below} is nearest 0x{bits:04x}"
            );
            assert_eq!(
                f32_to_f16_bits(above),
                bits + 1,
                "{above} is nearest 0x{:04x}",
                bits + 1
            );

            for probe in [midpoint, below, above] {
                assert_eq!(
                    f32_to_f16_bits(-probe),
                    f32_to_f16_bits(probe) | 0x8000,
                    "the encoding of {probe} and its negation must differ only in the sign"
                );
            }
        }
    }

    #[test]
    fn rounding_that_carries_reaches_the_next_exponent() {
        assert_eq!(f32_to_f16_bits(1.999_755_9), 0x4000);
        assert_eq!(f32_to_f16_bits(3.999_511_7), 0x4400);
        assert_eq!(f32_to_f16_bits(-1.999_755_9), 0xc000);
        assert_eq!(f32_to_f16_bits(65504.0), 0x7bff);
        assert_eq!(f32_to_f16_bits(65520.0), 0x7c00);
        assert_eq!(f32_to_f16_bits(f32::MAX), 0x7c00);
    }

    #[test]
    fn ties_round_to_even() {
        assert_eq!(f32_to_f16_bits(f32::from_bits(0x3f80_3000)), 0x3c02);
        assert_eq!(f32_to_f16_bits(f32::from_bits(0x3f80_1000)), 0x3c00);
        assert_eq!(f32_to_f16_bits(f32::from_bits(0x3f80_1800)), 0x3c01);
        assert_eq!(f32_to_f16_bits((2.0f32).powi(-25)), 0x0000);
        assert_eq!(f32_to_f16_bits((2.0f32).powi(-25) * 3.0), 0x0002);
    }

    #[test]
    fn zeros_infinities_and_nans_keep_their_identity() {
        assert_eq!(f32_to_f16_bits(0.0), 0x0000);
        assert_eq!(f32_to_f16_bits(-0.0), 0x8000);
        assert_eq!(f32_to_f16_bits(f32::INFINITY), 0x7c00);
        assert_eq!(f32_to_f16_bits(f32::NEG_INFINITY), 0xfc00);
        let low_payload_nan = f32::from_bits(0x7f80_0001);
        assert_ne!(f32_to_f16_bits(low_payload_nan) & 0x03ff, 0);
        assert_eq!(f32_to_f16_bits(low_payload_nan) & 0x7c00, 0x7c00);
        assert_eq!(f32_to_f16_bits((2.0f32).powi(-30)), 0x0000);
        assert_eq!(f32_to_f16_bits(-(2.0f32).powi(-30)), 0x8000);
    }
}
