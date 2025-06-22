BINFILE := wbroker-rs
SRCFILES := $(shell find src -type f -name '*.rs')
EXTFILES := $(shell find externals -type f -name '*')
TARGETARCH := armv7-unknown-linux-gnueabihf
PACKAGENAME := $(BINFILE).tar.gz

.PHONY: all clean unittests

all: dist/$(PACKAGENAME)

dist/$(PACKAGENAME): target/$(TARGETARCH)/release/$(BINFILE) $(EXTFILES)
	mkdir -p dist/$(BINFILE)
	cp -r -t dist/$(BINFILE) $(EXTFILES)
	cp    -t dist/$(BINFILE) target/$(TARGETARCH)/release/$(BINFILE)
	tar -czf dist/$(PACKAGENAME) -C dist $(BINFILE)

unittests:
	cargo test
	(cd peripheral && cargo test)

target/$(TARGETARCH)/release/$(BINFILE): $(SRCFILES)
	cross build --target $(TARGETARCH) --release

clean:
	rm -rf dist
	cross clean --target $(TARGETARCH)
