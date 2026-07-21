use ntrix_vdc_sdk::prelude::CharCell;

const FONT_SHEET_STRIDE: usize = ibm437::CHARS_PER_ROW;
const FONT_DATA: &[u8] = ibm437::IBM437_8X8_REGULAR_DATA;
pub const FONT_WIDTH: usize = 8;
pub const FONT_HEIGHT: usize = 8;

/// Render a single character cell onto a pixel frame-buffer.
pub fn render_char_cell(fb: &mut [u8], cb_cols: usize, col: usize, row: usize, cell: &CharCell) {
    let fb_stride: usize = (cb_cols * FONT_WIDTH) / 8;
    let offset = cell.glyph as usize;
    let glyph_col = offset % FONT_SHEET_STRIDE;
    let glyph_row = offset / FONT_SHEET_STRIDE;
    for y in 0..FONT_HEIGHT {
        let sheet_byte = FONT_DATA[(glyph_row * FONT_WIDTH + y) * FONT_SHEET_STRIDE + glyph_col];
        let byte = if cell.attrs.contains_invert() {
            !sheet_byte
        } else {
            sheet_byte
        };
        fb[(row * FONT_HEIGHT + y) * fb_stride + col] = byte;
    }
}
