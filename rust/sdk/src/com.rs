//! Shared API for the VDC wire protocol.
use num_enum::TryFromPrimitive;

use crate::helpers::does_u16_fit_u12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
enum ControlCode {
    Reset = 1,
    GetMode = 2,
    SetMode = 3,
    ReadRowPixels = 4,
    WriteRowPixels = 5,
    ReadRowChars = 6,
    WriteRowChars = 7,
    //ReadExt = 8,
    //WriteExt = 9,
}

/// Transform a control packet payload into/from it's raw form
pub trait PackablePayload
where
    Self: Sized,
{
    fn pack(self) -> u16;
    fn unpack(payload: u16) -> Result<Self, ControlPacketError>;
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u16)]
pub enum DisplayModeResolution {
    #[default]
    R640x480 = 0,
    //R320x240 = 2,
    //R160x120 = 4,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DisplayMode {
    resolution: DisplayModeResolution,
    chars_enabled: bool,
}

impl DisplayMode {
    pub fn new(resolution: DisplayModeResolution, chars_enabled: bool) -> Self {
        Self {
            resolution,
            chars_enabled,
        }
    }
}

impl PackablePayload for DisplayMode {
    fn pack(self) -> u16 {
        let res_bits = self.resolution as u16;
        let chars_bit = self.chars_enabled as u16;
        res_bits | chars_bit
    }

    fn unpack(payload: u16) -> Result<Self, ControlPacketError> {
        Ok(Self {
            resolution: DisplayModeResolution::try_from(payload & 0b110)
                .map_err(|_| ControlPacketError::InvalidPayload)?,
            chars_enabled: payload & 0b001 != 0,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ControlPacketError {
    InvalidCode,
    InvalidPayload,
}

pub enum ControlPacket {
    /// Reset the VDC
    Reset,
    /// Get current display mode
    GetMode,
    /// Set a new display mode
    SetMode(DisplayMode),
    /// Read a specific row of pixel data
    ReadRowPixels(u16),
    /// Write a specific row of pixel data
    WriteRowPixels(u16),
    /// Read a specific row of character data
    ReadRowChars(u16),
    /// Write a specific row of character data
    WriteRowChars(u16),
}

impl From<&ControlPacket> for ControlCode {
    fn from(value: &ControlPacket) -> Self {
        match value {
            ControlPacket::Reset => Self::Reset,
            ControlPacket::GetMode => Self::GetMode,
            ControlPacket::SetMode(_) => Self::SetMode,
            ControlPacket::ReadRowPixels(_) => Self::ReadRowPixels,
            ControlPacket::WriteRowPixels(_) => Self::WriteRowPixels,
            ControlPacket::ReadRowChars(_) => Self::ReadRowChars,
            ControlPacket::WriteRowChars(_) => Self::WriteRowChars,
        }
    }
}

impl ControlPacket {
    /// Pack ControlPacket into it's raw format,
    /// ready for sending over-the-wire.
    pub fn pack(&self) -> [u8; 2] {
        let cc = ControlCode::from(self);
        let payload = match self {
            Self::Reset => 0,
            Self::GetMode => 0,
            Self::SetMode(p) => p.pack(),
            Self::ReadRowPixels(p) => *p,
            Self::WriteRowPixels(p) => *p,
            Self::ReadRowChars(p) => *p,
            Self::WriteRowChars(p) => *p,
        };
        debug_assert!(does_u16_fit_u12(payload));
        (((cc as u16) << 12) | payload).to_be_bytes()
    }

    /// Unpack raw packet data received over-the-wire into a ControlPacket.
    pub fn unpack(bytes: [u8; 2]) -> Result<Self, ControlPacketError> {
        let packed = u16::from_be_bytes(bytes);
        let payload = packed & 0x0FFF; // bottom 12 bits

        let cc = ControlCode::try_from((packed >> 12) as u8)
            .map_err(|_| ControlPacketError::InvalidCode)?; // top 4 bits
        Ok(match cc {
            ControlCode::Reset => Self::Reset,
            ControlCode::GetMode => Self::GetMode,
            ControlCode::SetMode => Self::SetMode(DisplayMode::unpack(payload)?),
            ControlCode::ReadRowPixels => Self::ReadRowPixels(payload),
            ControlCode::WriteRowPixels => Self::WriteRowPixels(payload),
            ControlCode::ReadRowChars => Self::ReadRowChars(payload),
            ControlCode::WriteRowChars => Self::WriteRowChars(payload),
        })
    }
}
