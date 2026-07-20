add_executable(ntrix_vdc
    main.c
)

target_compile_definitions(ntrix_vdc PRIVATE
	DVI_DEFAULT_SERIAL_CONFIG=${DVI_DEFAULT_SERIAL_CONFIG}
	DVI_VERTICAL_REPEAT=1
	DVI_N_TMDS_BUFFERS=3
	DVI_MONOCHROME_TMDS=1
	)

target_link_libraries(ntrix_vdc
    pico_stdlib
	pico_multicore
	pico_util
    hardware_spi
	libdvi
    ntrix_vdc_runtime
)

pico_add_extra_outputs(ntrix_vdc)
