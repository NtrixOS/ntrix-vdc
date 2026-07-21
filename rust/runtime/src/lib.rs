#![no_std]
mod font;

use core::cell::UnsafeCell;

use bytemuck::{cast_slice, cast_slice_mut};
use ntrix_vdc_sdk::prelude::*;

use crate::font::{FONT_HEIGHT, FONT_WIDTH, render_char_cell};

#[inline(never)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;
const PIXEL_ROW_SIZE: usize = SCREEN_WIDTH / 8;
const CHAR_ROW_SIZE: usize = SCREEN_WIDTH / FONT_WIDTH;

/// # Safety
/// This is not actually Sync safe.
struct PixelBufferCell(UnsafeCell<[u8; PIXEL_ROW_SIZE * SCREEN_HEIGHT]>);
unsafe impl Sync for PixelBufferCell {}

static PIXEL_BUFFER: PixelBufferCell =
    PixelBufferCell(UnsafeCell::new([0; PIXEL_ROW_SIZE * SCREEN_HEIGHT]));
static mut CHAR_BUFFER: [CharCell; (SCREEN_WIDTH / FONT_WIDTH) * (SCREEN_HEIGHT / FONT_HEIGHT)] =
    [CharCell::from_u8_lossy(0); (SCREEN_WIDTH / FONT_WIDTH) * (SCREEN_HEIGHT / FONT_HEIGHT)];

struct DisplayModeCell(UnsafeCell<DisplayMode>);
unsafe impl Sync for DisplayModeCell {}
static CURRENT_DISPLAY_MODE: DisplayModeCell = DisplayModeCell(UnsafeCell::new(DisplayMode::new(
    DisplayModeResolution::R640x480,
    true,
)));

fn current_pixel_size() -> (usize, usize) {
    let res = unsafe { CURRENT_DISPLAY_MODE.0.get().as_ref_unchecked().resolution() };
    (res.width(), res.height())
}

fn current_pixel_row_size() -> usize {
    current_pixel_size().0 / 8
}

fn current_char_size() -> (usize, usize) {
    let res = unsafe { CURRENT_DISPLAY_MODE.0.get().as_ref_unchecked().resolution() };
    (res.width() / FONT_WIDTH, res.height() / FONT_HEIGHT)
}

fn current_char_row_size() -> usize {
    current_pixel_size().0 / FONT_WIDTH
}

fn get_current_display_mode() -> DisplayMode {
    unsafe { *CURRENT_DISPLAY_MODE.0.get() }
}

fn set_current_display_mode(mode: DisplayMode) {
    unsafe {
        *CURRENT_DISPLAY_MODE.0.get().as_mut_unchecked() = mode;
    }
}

#[repr(C)]
pub struct RawFrameBuffer {
    pub ptr: *const u8,
    pub width: usize,
    pub height: usize,
    pub bits_per_pixel: usize,
}

#[repr(C)]
pub struct HardwareCtx {
    pub read_bus_blocking: extern "C" fn(dst: *mut u8, len: usize) -> isize,
    pub write_bus_blocking: extern "C" fn(src: *const u8, len: usize) -> isize,
}

pub(crate) struct HardwareHandler {
    ctx: HardwareCtx,
}

impl HardwareHandler {
    pub(crate) fn new(ctx: HardwareCtx) -> Self {
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
    let (width, height) = current_pixel_size();
    let row_size = current_pixel_row_size();
    let fb = unsafe { &PIXEL_BUFFER.0.get().as_ref_unchecked()[0..row_size * height] };
    op(RawFrameBuffer {
        ptr: (*fb).as_ptr(),
        width,
        height,
        bits_per_pixel: 1,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn run(ctx: HardwareCtx) {
    let hw = HardwareHandler::new(ctx);
    loop {
        let mut raw_control_packet = [0u8; 2];
        hw.read_bus_blocking(&mut raw_control_packet).unwrap();
        let control_packet = ControlPacket::unpack(raw_control_packet).unwrap();
        match control_packet {
            ControlPacket::Reset => {
                unsafe {
                    CHAR_BUFFER[..].fill(CharCell::from_u8_lossy(0));
                    (*PIXEL_BUFFER.0.get()).fill(0);
                };
                set_current_display_mode(DisplayMode::new(DisplayModeResolution::R640x480, true));
            }
            ControlPacket::SetMode(mode) => {
                unsafe {
                    CHAR_BUFFER[..].fill(CharCell::from_u8_lossy(0));
                    (*PIXEL_BUFFER.0.get()).fill(0);
                };
                set_current_display_mode(mode);
            }
            ControlPacket::GetMode => {
                let mode = get_current_display_mode();
                let out = ControlPacket::SetMode(mode).pack();
                hw.write_bus_blocking(&out).unwrap();
            }
            ControlPacket::ReadRowPixels(row_index) => {
                let row_size = current_pixel_row_size();
                let row_offset = (row_index as usize) * row_size;
                let fb = unsafe {
                    &PIXEL_BUFFER.0.get().as_ref_unchecked()[row_offset..row_offset + row_size]
                };
                hw.write_bus_blocking(fb).unwrap();
            }
            ControlPacket::WriteRowPixels(row_index) => {
                let row_size = current_pixel_row_size();
                let row_offset = (row_index as usize) * row_size;
                let fb = unsafe {
                    &mut (&mut (*PIXEL_BUFFER.0.get()))[row_offset..row_offset + row_size]
                };
                hw.read_bus_blocking(fb).unwrap();
            }
            ControlPacket::ReadRowChars(row_index) => {
                let row_size = current_char_row_size();
                let row_offset = (row_index as usize) * row_size;
                let cb = unsafe { &CHAR_BUFFER[row_offset..row_offset + row_size] };
                hw.write_bus_blocking(cast_slice(&cb)).unwrap();
            }
            ControlPacket::WriteRowChars(row_i) => {
                let row_i = row_i as usize;
                let row_size = current_char_row_size();
                let row_offset = (row_i as usize) * row_size;
                let cb = unsafe { &mut CHAR_BUFFER[row_offset..row_offset + row_size] };
                hw.read_bus_blocking(cast_slice_mut(cb)).unwrap();
                let cells = unsafe { &CHAR_BUFFER[row_i + CHAR_ROW_SIZE..(row_i + 1) * row_size] };
                for (col_i, cell) in cells.iter().enumerate() {
                    unsafe {
                        render_char_cell(&mut *PIXEL_BUFFER.0.get(), row_size, col_i, row_i, cell)
                    };
                }
            }
        }
    }
}
