BINFILE := wbroker-rs

install:
	install -d                   /opt/$(BINFILE)/bin
	install -d                   /opt/$(BINFILE)/etc
	install -D -m 755 $(BINFILE) /opt/$(BINFILE)/bin/$(BINFILE)
	[ ! -e "/opt/$(BINFILE)/etc/$(BINFILE).toml" ] && install -b -D -m 644 $(BINFILE).toml /opt/$(BINFILE)/etc/$(BINFILE).toml
	install -D -m 644 $(BINFILE).service /etc/systemd/system/$(BINFILE).service
	systemctl daemon-reload
	systemctl enable wbroker-rs
