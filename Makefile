BINFILE := wbroker-rs
SRCFILES := $(shell find src -type f -name '*.rs')
UTILFILES := $(shell find utils -type f -name '*')
TARGETARCH := armv7-unknown-linux-gnueabihf
PACKAGENAME := $(BINFILE).tar.gz

.PHONY: all clean

all: dist/$(PACKAGENAME)

dist/$(PACKAGENAME): target/$(TARGETARCH)/release/$(BINFILE) $(UTILFILES)
	mkdir -p dist/$(BINFILE)
	cp -r -t dist/$(BINFILE) $(UTILFILES) 
	cp    -t dist/$(BINFILE) target/$(TARGETARCH)/release/$(BINFILE)
	tar -czf dist/$(PACKAGENAME) -C dist $(BINFILE)

target/$(TARGETARCH)/release/$(BINFILE): $(SRCFILES)
	cross build --target $(TARGETARCH) --release

clean:
	rm -rf dist
	cross clean --target $(TARGETARCH)