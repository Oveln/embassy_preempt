# ==============================================================================
# Embassy Preempt - Makefile
# ==============================================================================
# This Makefile provides targets for building, flashing, and debugging the
# Embassy Preempt RTOS project.
# ==============================================================================

# ******************************************************************************
# VARIABLES
# ******************************************************************************

# Arch
ARCH = arm
PLATFORM := thumbv7em-none-eabi
DEVICE := STM32F401RE
ENTRY_PA := 0x08000000

# Tool definitions
OBJDUMP := rust-objdump --arch-name=$(ARCH)
OBJCOPY := rust-objcopy --binary-architecture=$(ARCH)

# Build mode (default is debug)
MODE ?= debug
ifeq ($(MODE), release)
    MODE_ARG := --release
endif

# RTT configurations
RTT_PORT := 8765

# Default target configurations
TARGET ?= sync_time_performance

# File paths
TARGET_ELF := target/$(PLATFORM)/$(MODE)/$(TARGET)
TARGET_BIN := target/$(PLATFORM)/$(MODE)/$(TARGET).bin

# ******************************************************************************
# DEFAULT TARGET
# ******************************************************************************
.PHONY: all
all: build

# ******************************************************************************
# BUILD TARGETS
# ******************************************************************************
# Targets for building the project in different configurations
# ******************************************************************************

.PHONY: build
build: $(OBJ_FILES)
	@echo "Building project in $(MODE) mode..."
	cargo build $(MODE_ARG) --features "stm32f401re","alarm_test"

# ******************************************************************************
# TARGET BUILD TARGETS
# ******************************************************************************
# Targets for building specific binaries
# ******************************************************************************

.PHONY: $(TARGET)
$(TARGET): build $(TARGET_ELF)
	@echo "Creating binary for $(TARGET)..."
	$(OBJCOPY) -O binary $(TARGET_ELF) $(TARGET_BIN)

.PHONY: bin
bin: $(TARGET)
	@echo "Build done"

# ******************************************************************************
# FLASHING TARGETS (OpenOCD)
# ******************************************************************************
# Targets for flashing the binary to the microcontroller using OpenOCD
# ******************************************************************************

.PHONY: download flash
download: build bin
	@echo "Flashing binary to device using OpenOCD..."
	openocd -f interface/stlink.cfg -f target/stm32f4x.cfg \
	-c "adapter speed 1800" \
	-c "init" \
	-c "reset_config srst_only" \
	-c "reset halt" \
	-c "flash write_image erase $(TARGET_BIN) $(ENTRY_PA)" \
	-c "verify_image $(TARGET_BIN) $(ENTRY_PA)" \
	-c "reset run; shutdown"

flash: download

# ******************************************************************************
# DEBUGGING TARGETS (OpenOCD)
# ******************************************************************************
# Targets for debugging with OpenOCD and GDB
# ******************************************************************************

# Extract RTT address and size for the test ELF
RTT_ADDR := $(shell rust-nm -S $(TEST_ELF) 2>/dev/null | grep _SEGGER_RTT | awk '{print $$1}')
RTT_SIZE := $(shell rust-nm -S $(TEST_ELF) 2>/dev/null | grep _SEGGER_RTT | awk '{print $$2}')

.PHONY: debug-test
debug-test: bin
	@echo "Starting debugging session with OpenOCD and GDB..."
	# Flash the binary with verification using safer settings
	openocd -f interface/stlink.cfg -f target/stm32f4x.cfg \
		-c "adapter speed 1000" \
		-c "init" \
		-c "reset_config srst_only" \
		-c "reset halt" \
		-c "flash write_image erase $(TARGET_BIN) $(ENTRY_PA)" \
		-c "verify_image $(TARGET_BIN) $(ENTRY_PA)" \
		-c "reset halt" \
		-c "shutdown" && \
	# Start OpenOCD server with RTT support
	( \
		openocd -f interface/stlink.cfg -f target/stm32f4x.cfg \
		-c "adapter speed 1000" \
		-c "init" \
		-c "reset_config srst_only" \
		-c "reset halt" \
		-c "monitor reset halt" \
		-c "monitor rtt server start $(RTT_PORT) 0" \
		-c "monitor rtt setup 0x$(shell rust-nm -S $(TARGET_ELF) 2>/dev/null | grep _SEGGER_RTT | awk '{print $1}') 0x$(shell rust-nm -S $(TARGET_ELF) 2>/dev/null | grep _SEGGER_RTT | awk '{print $2}') \"SEGGER RTT\"" \
		-c "monitor rtt start" & \
		sleep 2 && \
		# Start GDB session in background
		tmux new-session -d -s embassy_debug \
			"RUST_GDB=/usr/bin/gdb-multiarch rust-gdb \
			-ex 'file $(TARGET_ELF)' \
			-ex 'set arch arm' \
			-ex 'target extended-remote localhost:3333' \
			-ex 'source ./bp.gdb' \
			-ex 'source ./.gdbinit' \
			-ex 'monitor reset halt' \
			-ex 'load' \
			-ex 'monitor reset halt' \
			-ex 'continue'"; \
		# Start RTT output viewer in another pane
		tmux split-window -h -t embassy_debug \
			"nc localhost $(RTT_PORT) | defmt-print -e $(TARGET_ELF)"; \
		# Attach to the session
		tmux attach-session -t embassy_debug \
	)

.PHONY: kill
kill:
	@PID=$$(lsof -t -i:2331); \
    if [ -n "$$PID" ]; then \
        echo "Killing process on port 2331 with PID: $$PID"; \
        kill -9 $$PID; \
    else \
        echo "No process found on port 2331"; \
    fi

# ******************************************************************************
# HELPER TARGETS
# ******************************************************************************

.PHONY: clean
clean: kill
	@echo "Cleaning project..."
	cargo clean

.PHONY: run
run:
	@echo "Running project in $(MODE) mode..."
	clear
	cargo run $(MODE_ARG)

# ******************************************************************************
# BINARY CONVERSION TARGETS
# ******************************************************************************
# Targets to convert ELF files to binary format
# ******************************************************************************

.PHONY: bin-test
bin-test:
	@echo "Converting test ELF to binary..."
	$(OBJCOPY) -O binary $(TEST_ELF) $(TEST_BINARY)
	$(OBJCOPY) $(TEST_ELF) $(TEST_ELF_NEW)

.PHONY: bin-lib
bin-lib:
	@echo "Converting lib ELF to binary..."
	$(OBJCOPY) -O binary $(TEST_ELF_LIB) $(TEST_BINARY_LIB)
	$(OBJCOPY) $(TEST_ELF_LIB) $(TEST_ELF_NEW_LIB)

.PHONY: debug-lib
debug-lib: bin-lib
	@echo "Starting debugging session for library with OpenOCD and GDB..."
	# Flash the library binary with verification using safer settings
	openocd -f interface/stlink.cfg -f target/stm32f4x.cfg \
		-c "adapter speed 1000" \
		-c "init" \
		-c "reset_config srst_only" \
		-c "reset halt" \
		-c "flash write_image erase $(TEST_BINARY_LIB) $(ENTRY_PA)" \
		-c "verify_image $(TEST_BINARY_LIB) $(ENTRY_PA)" \
		-c "reset halt" \
		-c "shutdown" && \
	# Start OpenOCD server with RTT support
	( \
		openocd -f interface/stlink.cfg -f target/stm32f4x.cfg \
		-c "adapter speed 1000" \
		-c "init" \
		-c "reset_config srst_only" \
		-c "reset halt" \
		-c "monitor reset halt" \
		-c "monitor rtt server start $(RTT_PORT) 0" \
		-c "monitor rtt setup 0x$(shell rust-nm -S $(TEST_ELF_LIB) 2>/dev/null | grep _SEGGER_RTT | awk '{print $1}') 0x$(shell rust-nm -S $(TEST_ELF_LIB) 2>/dev/null | grep _SEGGER_RTT | awk '{print $2}') \"SEGGER RTT\"" \
		-c "monitor rtt start" & \
		sleep 2 && \
		# Start GDB session in background
		tmux new-session -d -s embassy_lib_debug \
			"RUST_GDB=/usr/bin/gdb-multiarch rust-gdb \
			-ex 'file $(TEST_ELF_LIB)' \
			-ex 'set arch arm' \
			-ex 'target extended-remote localhost:3333' \
			-ex 'source ./bp.gdb' \
			-ex 'source ./.gdbinit' \
			-ex 'monitor reset halt' \
			-ex 'load' \
			-ex 'monitor reset halt' \
			-ex 'continue'"; \
		# Start RTT output viewer in another pane
		tmux split-window -h -t embassy_lib_debug \
			"nc localhost $(RTT_PORT) | defmt-print -e $(TEST_ELF_LIB)"; \
		# Attach to the session
		tmux attach-session -t embassy_lib_debug \
	)

# ******************************************************************************
# LOGGING/DEBUGGING OUTPUT TARGETS
# ******************************************************************************
# Targets for viewing RTT/defmt output
# ******************************************************************************

.PHONY: defmt-test
defmt-test:
	@echo "Displaying defmt output for test..."
	zsh -c "nc localhost $(RTT_PORT) | defmt-print -e $(TEST_ELF_NEW) "

.PHONY: defmt-lib
defmt-lib:
	@echo "Displaying defmt output for library..."
	zsh -c "nc localhost $(RTT_PORT) | defmt-print -e $(TEST_ELF_NEW_LIB) "

.PHONY: defmt
defmt:
	@echo "Displaying defmt output for current target..."
	zsh -c "nc localhost $(RTT_PORT) | defmt-print -e $(TARGET_ELF) "

# ******************************************************************************
# HELP TARGET
# ******************************************************************************
# Provides a help message showing available targets
# ******************************************************************************

.PHONY: help
help:
	@echo ""
	@echo "Embassy Preempt - Makefile Help"
	@echo "=============================================================================="
	@echo ""
	@echo "USAGE:"
	@echo "  make [TARGET] [OPTIONS]"
	@echo ""
	@echo "OPTIONS:"
	@echo "  MODE=debug|release    Build in debug (default) or release mode"
	@echo ""
	@echo "TARGETS:"
	@echo ""
	@echo "  Build Targets:"
	@echo "    all                 Build project in release mode (default)"
	@echo "    build               Build project in specified mode (default: release)"
	@echo "    lib                 Build library in $(MODE) mode"
	@echo "    test-run            Run integration tests"
	@echo "    lib-run             Run library tests"
	@echo ""
	@echo "  Binary Build Targets:"
	@echo "    bin                 Build the main target binary"
	@echo "    bin-test            Convert test ELF to binary"
	@echo "    bin-lib             Convert library ELF to binary"
	@echo ""
	@echo "  Flashing Targets (OpenOCD):"
	@echo "    download            Flash binary to device using OpenOCD"
	@echo ""
	@echo ""
	@echo "  Debugging Targets (OpenOCD):"
	@echo "    debug-test          Debug test with OpenOCD and GDB"
	@echo "    debug-lib           Debug library with OpenOCD and GDB"
	@echo ""
	@echo "    kill                Kill any processes running on port 2331"
	@echo ""
	@echo "  Logging/Output:"
	@echo "    defmt               Display defmt output for current target"
	@echo "    defmt-test          Display defmt output for tests"
	@echo "    defmt-lib           Display defmt output for library"
	@echo ""
	@echo "  Other:"
	@echo "    clean               Remove all build artifacts"
	@echo "    run                 Run project in specified mode"
	@echo "    help                Show this help message"
	@echo ""
	@echo "EXAMPLES:"
	@echo "  make build MODE=debug          # Build in debug mode"
	@echo "  make download                  # Flash release build to device"
	@echo "  make clean                     # Clean all build artifacts"
	@echo ""

# ******************************************************************************
# END OF MAKEFILE
# ******************************************************************************