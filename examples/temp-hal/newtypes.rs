// Various newtypes so that I can (re)implement traits from upstream crates optimized for size.
// Perhaps eventually they will become full-fledged crates.

pub mod fmt {
    use core::fmt::Write;
    use core::fmt::{Display, Error, Formatter};
    use fixed::types::{I8F8, I9F7};

    pub struct I8F8SmallFmt(I8F8);
    pub struct I9F7SmallFmt(I9F7);

    impl Display for I8F8SmallFmt {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
            fixed16_fmt_impl(self.0.to_bits(), 8, f)
        }
    }

    impl Display for I9F7SmallFmt {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
            fixed16_fmt_impl(self.0.to_bits(), 7, f)
        }
    }

    impl From<I8F8> for I8F8SmallFmt {
        fn from(i: I8F8) -> Self {
            I8F8SmallFmt(i)
        }
    }

    impl From<I9F7> for I9F7SmallFmt {
        fn from(i: I9F7) -> Self {
            I9F7SmallFmt(i)
        }
    }

    #[inline(never)]
    fn fixed16_fmt_impl(inner: i16, num_frac_bits: u8, f: &mut Formatter<'_>) -> Result<(), Error> {
        let mut int_buf = [0u8; 6]; // Up to 32767/Down to -32768 (5 plus 1 sign)
        let mut frac_buf = [0u8; 6]; // Down to 0.000015

        let neg = inner < 0; // Sign bit in i16 is sign bit in fixed point :).
        let frac_mask = (1u16 << num_frac_bits) - 1;

        let mut int_part: u16;
        let mut frac_part: u16;
        let mut int_buf_start: usize;
        let mut frac_buf_end: usize = 6;

        if neg {
            int_buf_start = 0;

            let twos_comp = inner.wrapping_abs();

            // Handle most negative value specially.
            if twos_comp == inner {
                int_part = 2u16.pow((15 - num_frac_bits).into());
                frac_part = 0;
            } else {
                int_part = (twos_comp as u16) >> num_frac_bits;
                frac_part = (twos_comp as u16) & frac_mask;
            }
        } else {
            int_buf_start = 1;
            int_part = (inner as u16) >> num_frac_bits;
            frac_part = (inner as u16) & frac_mask;
        }

        for (offs, i) in int_buf.iter_mut().rev().enumerate() {
            let tens: u8 = (int_part % 10) as u8;
            *i = tens + ('0' as u8);

            int_part = int_part / 10;

            // TODO: Fill?
            if int_part == 0 {
                // offs + 1 because we just finished processing the last _used_
                // cell and store sign in the next cell.
                if neg {
                    int_buf_start = 5 - (offs + 1);
                } else {
                    int_buf_start = 5 - offs;
                }

                break;
            }
        }

        for (offs, fr) in frac_buf.iter_mut().enumerate() {
            let mut tmp_frac_part = frac_part;
            let mut tmp_num_frac_bits = num_frac_bits;

            // We multiply by 10, and then shift to only leave the int part.
            // We need at least 4 bits of room to store the result of multiplying
            // by 10, otherwise it'll get truncated. Temporary shift the fractional
            // part to get 4 bits of free space for the multiply and shift.
            //
            // We don't have this problem for the int part because we can grab
            // the bits shifted out via the remainder.
            if num_frac_bits > 11 {
                tmp_frac_part /= 16;
                tmp_num_frac_bits -= 4;
            }

            let tmp_frac_part = tmp_frac_part * 10;
            let frac_part_in_int_pos = tmp_frac_part >> tmp_num_frac_bits;

            let tens = (frac_part_in_int_pos % 10) as u8;
            *fr = tens + ('0' as u8);

            // Then update and remove part of frac that went past radix point.
            frac_part = frac_part.wrapping_mul(10);
            frac_part = frac_part & frac_mask;

            // TODO: Fill?
            // offs + 1 because we just finished processing the last _used_
            // cell.
            if frac_part == 0 || f.precision().unwrap_or(3) <= offs + 1 {
                frac_buf_end = offs + 1;
                break;
            }
        }

        if neg {
            int_buf[int_buf_start] = '-' as u8;
        }

        f.write_str(core::str::from_utf8(&int_buf[int_buf_start..]).unwrap())?;
        f.write_char('.')?;
        f.write_str(core::str::from_utf8(&frac_buf[..frac_buf_end]).unwrap())?;

        Ok(())
    }
}
