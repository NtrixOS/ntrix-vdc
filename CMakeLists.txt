cmake_minimum_required(VERSION 3.13...3.27)

include(pico-sdk/pico_sdk_init.cmake)

project(pico_dvi_display C CXX ASM)
set(CMAKE_C_STANDARD 11)
set(CMAKE_CXX_STANDARD 17)

pico_sdk_init()

set(RUST_LIB_DIR ${CMAKE_SOURCE_DIR}/rust)
set(RUST_LIB_TARGET_DIR ${RUST_LIB_DIR}/target/thumbv8m.main-none-eabi/release)
set(RUST_LIB_PATH ${RUST_LIB_TARGET_DIR}/libntrix_vdc_runtime.a)
add_custom_command(
    OUTPUT ${RUST_LIB_PATH}
    COMMAND cargo build --release --manifest-path ${RUST_LIB_DIR}/Cargo.toml
    WORKING_DIRECTORY ${RUST_LIB_DIR}
    # Rebuild whenever any Rust source change
    # FIXME this does not rebuild on change
    DEPENDS ${RUST_LIB_DIR}
    COMMENT "Building Rust library"
    VERBATIM
)
add_custom_target(rust_lib_build ALL DEPENDS ${RUST_LIB_PATH})
add_library(ntrix_vdc_runtime STATIC IMPORTED GLOBAL)
set_target_properties(ntrix_vdc_runtime PROPERTIES IMPORTED_LOCATION ${RUST_LIB_PATH})
add_dependencies(ntrix_vdc_runtime rust_lib_build)
include_directories(rust/target/include)

add_subdirectory(pico-dvi/software)
include_directories(pico-dvi/software/include)
add_subdirectory(src)
