/// Does the u16 fit in a u12.
pub fn does_u16_fit_u12(v: u16) -> bool {
    v <= 0x0FFF
}
