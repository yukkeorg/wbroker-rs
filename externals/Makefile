BINFILE := wbroker-rs

install:
	install -D -m 755 $(BINFILE) /opt/$(BINFILE)/$(BINFILE)
	install -D -m 644 $(BINFILE).service /etc/systemd/system/$(BINFILE).service
	systemctl daemon-reload
	systemctl enable wbroker-rs
