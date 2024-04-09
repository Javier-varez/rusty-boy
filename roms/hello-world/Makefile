BUILD_DIR := build

all: $(BUILD_DIR)/main.gb
clean:
	@rm -rf ${BUILD_DIR}

$(BUILD_DIR)/%.o: %.asm
	@mkdir -p $(dir $@)
	rgbasm $< -o $@ -Wall

$(BUILD_DIR)/%.gb: $(BUILD_DIR)/%.o
	rgblink -o $@ --map $@.map $<
	rgbfix -v $@ -p 0xff
