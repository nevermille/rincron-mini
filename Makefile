build:
	cargo build --release

deb: build
	mkdir -p "rincron-mini/DEBIAN" "rincron-mini/etc/systemd/system" "rincron-mini/etc/systemd/user" "rincron-mini/usr/bin"
	cp "assets/deb/control" "assets/deb/postinst" "assets/deb/preinst" "assets/deb/prerm" "rincron-mini/DEBIAN"
	chmod a+x "assets/deb/postinst" "assets/deb/preinst" "assets/deb/prerm"
	cp "target/release/rincron_mini" "rincron-mini/usr/bin/rincron-mini"
	cp "assets/systemd/rincron-mini.service" "rincron-mini/etc/systemd/system/rincron-mini.service"
	cp "assets/systemd/rincron-mini.user.service" "rincron-mini/etc/systemd/user/rincron-mini.service"
	dpkg-deb --build rincron-mini
	rm -rf rincron-mini

xz: build
	mkdir -p "rincron-mini/bin" "rincron-mini/service/user" "rincron-mini/service/system"
	cp "target/release/rincron_mini" "rincron-mini/bin/rincron-mini"
	cp "assets/systemd/rincron-mini.service" "rincron-mini/service/system/rincron-mini.service"
	cp "assets/systemd/rincron-mini.user.service" "rincron-mini/service/user/rincron-mini.service"
	tar -cvJf rincron-mini.tar.xz rincron-mini
	rm -rf rincron-mini

clean:
	cargo clean
	rm -rf "rincron-mini"
