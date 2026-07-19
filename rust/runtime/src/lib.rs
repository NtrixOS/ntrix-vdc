#![no_std]

use embassy_sync::blocking_mutex::CriticalSectionMutex as Mutex;
use ntrix_vdc_sdk::prelude::CharCell;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;
const FONT_SIZE: usize = 8;

static PIXEL_BUFFER: Mutex<[u8; SCREEN_WIDTH * SCREEN_HEIGHT / 8]> =
    Mutex::new([0; SCREEN_WIDTH * SCREEN_HEIGHT / 8]);
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
pub extern "C" fn run(app_ctx: &AppCtx) {
    // TODO
    todo!()
}
