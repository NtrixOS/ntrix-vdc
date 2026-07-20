#![no_std]

use embassy_sync::blocking_mutex::CriticalSectionMutex as Mutex;
use ntrix_vdc_sdk::prelude::*;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;
const PIXEL_ROW_SIZE: usize = SCREEN_WIDTH / 8;
const FONT_SIZE: usize = 8;

static PIXEL_BUFFER: Mutex<[u8; PIXEL_ROW_SIZE * SCREEN_HEIGHT]> =
    Mutex::new([0; PIXEL_ROW_SIZE * SCREEN_HEIGHT]);
static mut CHAR_BUFFER: [CharCell; (SCREEN_WIDTH / FONT_SIZE) * (SCREEN_HEIGHT / FONT_SIZE)] =
    [CharCell::from_u8_lossy(0); (SCREEN_WIDTH / FONT_SIZE) * (SCREEN_HEIGHT / FONT_SIZE)];

#[repr(C)]
pub struct RawFrameBuffer {
    pub ptr: *const u8,
    pub width: usize,
    pub height: usize,
    pub bits_per_pixel: usize,
}

#[repr(C)]
pub struct AppCtx {
    pub read_bus_blocking: extern "C" fn(dst: *mut u8, len: usize) -> isize,
    pub write_bus_blocking: extern "C" fn(src: *const u8, len: usize) -> isize,
}

pub(crate) struct HardwareHandler {
    ctx: AppCtx,
}

impl HardwareHandler {
    pub(crate) fn new(ctx: AppCtx) -> Self {
        Self { ctx }
    }
    pub(crate) fn read_bus_blocking(&self, dst: &mut [u8]) -> Result<(), isize> {
        let bytes_read = (self.ctx.read_bus_blocking)(dst.as_mut_ptr(), dst.len());
        if bytes_read >= 0 && bytes_read as usize == dst.len() {
            Ok(())
        } else {
            Err(bytes_read)
        }
    }

    pub(crate) fn write_bus_blocking(&self, src: &[u8]) -> Result<(), isize> {
        let bytes_written = (self.ctx.write_bus_blocking)(src.as_ptr(), src.len());
        if bytes_written >= 0 && bytes_written as usize == src.len() {
            Ok(())
        } else {
            Err(bytes_written)
        }
    }
}

/// Acquire access to the raw frame buffer.
///
/// # Safety
/// - Should not be used outside of the call site.
#[unsafe(no_mangle)]
pub extern "C" fn aquire_framebuffer(op: extern "C" fn(RawFrameBuffer)) {
    PIXEL_BUFFER.lock(|fb| {
        op(RawFrameBuffer {
            ptr: fb.as_ptr(),
            width: SCREEN_WIDTH,
            height: SCREEN_HEIGHT,
            bits_per_pixel: 1,
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn run(ctx: AppCtx) {
    let hw = HardwareHandler::new(ctx);
    loop {
        let mut raw_control_packet = [0u8; 2];
        hw.read_bus_blocking(&mut raw_control_packet).unwrap();
        let control_packet = ControlPacket::unpack(raw_control_packet).unwrap();
        match control_packet {
            ControlPacket::GetMode => {
                let mode = DisplayMode::new(DisplayModeResolution::default(), false);
                let out = ControlPacket::SetMode(mode).pack();
                hw.write_bus_blocking(&out).unwrap();
            }
            ControlPacket::ReadRowPixels(row_index) => {
                PIXEL_BUFFER.lock(|fb| {
                    let row_offset = (row_index as usize) * PIXEL_ROW_SIZE;
                    let fb = &fb[row_offset..row_offset + PIXEL_ROW_SIZE];
                    hw.write_bus_blocking(fb).unwrap();
                });
            }
            ControlPacket::WriteRowPixels(row_index) => unsafe {
                PIXEL_BUFFER.lock_mut(|fb| {
                    let row_offset = (row_index as usize) * PIXEL_ROW_SIZE;
                    let fb = &mut fb[row_offset..row_offset + PIXEL_ROW_SIZE];
                    hw.read_bus_blocking(fb).unwrap();
                });
            },
            _ => todo!(),
        }
    }
}
