#[cfg(not(feature = "defmt"))]
use bitflags::bitflags;
#[cfg(feature = "defmt")]
use defmt::bitflags;

bitflags! {
    #[derive(Default, bytemuck::Pod, bytemuck::Zeroable)]
#[cfg_attr(not(feature = "defmt"), derive(Debug, Clone, Copy, PartialEq, Eq))]
    #[repr(transparent)]
    pub struct CharAttributes: u8 {
        const INVERT = 1;
        const UNDERLINE = 2;
        const STRIKE = 3;
    }
}

impl CharAttributes {
    /// Whether to invert the visuals of the cell.
    pub const fn contains_invert(&self) -> bool {
        self.contains(CharAttributes::INVERT)
    }

    /// Whether the apply an underline to the cell
    pub const fn contains_underline(&self) -> bool {
        self.contains(CharAttributes::UNDERLINE)
    }

    /// Whether the apply a strikethrough to the cell
    pub const fn contains_strikethrough(&self) -> bool {
        self.contains(CharAttributes::STRIKE)
    }
}

#[repr(C)]
#[derive(PartialEq, Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[non_exhaustive]
pub struct CharCell {
    pub glyph: u8,
    pub attrs: CharAttributes,
}

impl CharCell {
    /// Convert from a u8 glyph, replacing with '?' on when out of ASCII range
    pub const fn from_u8_lossy(glyph: u8) -> Self {
        Self {
            glyph: if glyph.is_ascii() { glyph } else { b'?' },
            attrs: CharAttributes::empty(),
        }
    }
}

impl TryFrom<u8> for CharCell {
    type Error = ();
    fn try_from(glyph: u8) -> Result<Self, Self::Error> {
        if !glyph.is_ascii() {
            Err(())
        } else {
            Ok(Self {
                glyph,
                attrs: CharAttributes::empty(),
            })
        }
    }
}

impl<'a> From<&'a CharCell> for &'a [u8; 2] {
    fn from(value: &'a CharCell) -> Self {
        bytemuck::cast_ref(value)
    }
}

impl CharCell {
    pub fn as_bytes(&self) -> &[u8; 2] {
        bytemuck::cast_ref(self)
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(core::slice::from_ref(&self.glyph)) }
    }
}
