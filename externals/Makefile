BINFILE := wbroker-rs

install:
	install -d                   /opt/$(BINFILE)/bin
	install -D -m 755 $(BINFILE) /opt/$(BINFILE)/bin/$(BINFILE)
	install -D -m 644 $(BINFILE).service /etc/systemd/system/$(BINFILE).service
	systemctl daemon-reload
	systemctl enable wbroker-rs
