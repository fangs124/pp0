EXE = pp0
ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
else            
	NAME := $(EXE)
endif

rule:
	cargo rustc --bin pp0 --release --package pp0 -- -C target-cpu=native --emit link=$(NAME)