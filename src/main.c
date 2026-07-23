#include "common_dvi_pin_configs.h"
#include "dvi.h"
#include "dvi_serialiser.h"
#include "hardware/clocks.h"
#include "hardware/dma.h"
#include "hardware/gpio.h"
#include "hardware/irq.h"
#include "hardware/spi.h"
#include "hardware/structs/bus_ctrl.h"
#include "hardware/sync.h"
#include "hardware/vreg.h"
#include "ntrix_vdc_runtime.h"
#include "pico/multicore.h"
#include "pico/sem.h"
#include "pico/stdlib.h"
#include "tmds_encode.h"
#include <hardware/structs/io_bank0.h>
#include <pico/stdio.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MODE_640x480_60Hz
#define FRAME_WIDTH 640
#define FRAME_HEIGHT 480
#define VREG_VSEL VREG_VOLTAGE_1_20
#define DVI_TIMING dvi_timing_640x480p_60hz

#define FRAME_ROW_SIZE FRAME_WIDTH / 8

#define SPI_DEVICE spi0
#define SPI_RX_PIN 4
#define SPI_SCK_PIN 2
#define SPI_TX_PIN 3
#define SPI_CSN_PIN 5

struct dvi_inst dvi0;
struct semaphore dvi_start_sem;

static uint y = 0;
static uint8_t scanbuf[FRAME_WIDTH / 8];
static uint16_t double_bits_lut[256]; // 1 byte -> 2 bytes (2x horiz expand)

static void build_luts(void) {
  for (int b = 0; b < 256; b++) {
    uint16_t out = 0;
    for (int i = 0; i < 8; i++) {
      int bit = (b >> (7 - i)) & 1;
      out |= (uint16_t)bit << (15 - i * 2);
      out |= (uint16_t)bit << (14 - i * 2);
    }
    double_bits_lut[b] = out;
  }
}

static inline void cpy_callback(struct RawFrameBuffer fb) {
  switch (fb.width) {
  case 640: { // no scaling
    memcpy(scanbuf, &fb.ptr[y * FRAME_ROW_SIZE], sizeof(scanbuf));
    break;
  }
  case 320: { // 2x scaling
    const uint src_row_size = 320 / 8;
    const uint src_y = y >> 1;
    const uint8_t *src_row = &fb.ptr[src_y * src_row_size];

    for (uint x = 0; x < src_row_size; x++) {
      uint16_t d = double_bits_lut[src_row[x]];
      scanbuf[x * 2] = (uint8_t)(d >> 8);
      scanbuf[x * 2 + 1] = (uint8_t)(d & 0xFF);
    }
    break;
  }
  case 160: { // 4x scaling
    const uint src_row_size = 160 / 8;
    const uint src_y = y >> 2;
    const uint8_t *src_row = &fb.ptr[src_y * src_row_size];

    for (uint x = 0; x < src_row_size; x++) {
      uint16_t d = double_bits_lut[src_row[x]]; // 8 px -> 16 px
      uint8_t hi = (uint8_t)(d >> 8);
      uint8_t lo = (uint8_t)(d & 0xFF);

      // could perform quad scaling lut,
      // or save memory and perform two passes
      uint16_t d_hi = double_bits_lut[hi]; // 16 px -> 32 px (first half)
      uint16_t d_lo = double_bits_lut[lo]; // 16 px -> 32 px (second half)

      scanbuf[x * 4] = (uint8_t)(d_hi >> 8);
      scanbuf[x * 4 + 1] = (uint8_t)(d_hi & 0xFF);
      scanbuf[x * 4 + 2] = (uint8_t)(d_lo >> 8);
      scanbuf[x * 4 + 3] = (uint8_t)(d_lo & 0xFF);
    }
    break;
  }
  }
}

static inline void prepare_scanline(uint y) {
  aquire_framebuffer(cpy_callback);
  uint32_t *tmdsbuf;
  queue_remove_blocking(&dvi0.q_tmds_free, &tmdsbuf);
  tmds_encode_1bpp((const uint32_t *)scanbuf, tmdsbuf, FRAME_WIDTH);
  queue_add_blocking(&dvi0.q_tmds_valid, &tmdsbuf);
}

void core1_scanline_callback() {
  prepare_scanline(y);
  y = (y + 1) % FRAME_HEIGHT;
}

static inline intptr_t hw_read_bus_blocking(uint8_t *dst, uintptr_t len) {
  return spi_read_blocking(SPI_DEVICE, 0x00, dst, len);
}

static inline intptr_t hw_write_bus_blocking(const uint8_t *src,
                                             uintptr_t len) {
  return spi_write_blocking(SPI_DEVICE, src, len);
}

void __not_in_flash("main") core1_main() {
  build_luts();

  dvi_register_irqs_this_core(&dvi0, DMA_IRQ_0);
  sem_acquire_blocking(&dvi_start_sem);
  dvi_start(&dvi0);
  // IRQ driven, so can sleep
  // should also have enough processing power
  // to do something simple if needed later
  while (1)
    __wfi();
  __builtin_unreachable();
}

int __not_in_flash("main") main() {
  vreg_set_voltage(VREG_VSEL);
  sleep_ms(10);
  // Run system at TMDS bit clock
  set_sys_clock_khz(DVI_TIMING.bit_clk_khz, true);

  stdio_init_all();

  // init/setup SPI
  uint spi_baudrate = spi_init(SPI_DEVICE, 1000000);
  printf("running at bus %d baud\n", spi_baudrate);
  spi_set_format(SPI_DEVICE, 8, SPI_CPOL_0,
                 SPI_CPHA_1, // NOTE required so CS does not need to be pulsed
                             // every data word transfer
                 SPI_MSB_FIRST);
  spi_set_slave(SPI_DEVICE, true);
  gpio_set_function(SPI_RX_PIN, GPIO_FUNC_SPI);
  gpio_set_function(SPI_SCK_PIN, GPIO_FUNC_SPI);
  gpio_set_function(SPI_TX_PIN, GPIO_FUNC_SPI);
  gpio_set_function(SPI_CSN_PIN, GPIO_FUNC_SPI);

  // Configure DVI
  dvi0.timing = &DVI_TIMING;
  dvi0.ser_cfg = DVI_DEFAULT_SERIAL_CONFIG;
  dvi0.scanline_callback = core1_scanline_callback;
  dvi_init(&dvi0, next_striped_spin_lock_num(), next_striped_spin_lock_num());

  // prep first scanline
  prepare_scanline(0);

  // start DVI
  sem_init(&dvi_start_sem, 0, 1);
  hw_set_bits(&bus_ctrl_hw->priority, BUSCTRL_BUS_PRIORITY_PROC1_BITS);
  multicore_launch_core1(core1_main);

  sem_release(&dvi_start_sem);

  // start VDC runtime
  struct HardwareCtx hw_ctx = {
      .printf = printf,
      .read_bus_blocking = hw_read_bus_blocking,
      .write_bus_blocking = hw_write_bus_blocking,
  };
  printf("handing over core0 to rust vdc runtime\n");
  run(hw_ctx);
}
