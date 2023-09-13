version=0.3.1-rc2
arch=$(shell uname --machine)
package_name="rincron-mini.${version}.${arch}"

build:
	cargo build --release

deb: build
	mkdir -p "${package_name}/DEBIAN" "${package_name}/etc/systemd/system" "${package_name}/etc/systemd/user" "${package_name}/usr/bin"
	cp "assets/deb/control" "assets/deb/postinst" "assets/deb/preinst" "assets/deb/prerm" "${package_name}/DEBIAN"
	chmod a+x "assets/deb/postinst" "assets/deb/preinst" "assets/deb/prerm"
	cp "target/release/rincron_mini" "${package_name}/usr/bin/rincron-mini"
	cp "assets/systemd/rincron-mini.service" "${package_name}/etc/systemd/system/rincron-mini.service"
	cp "assets/systemd/rincron-mini.user.service" "${package_name}/etc/systemd/user/rincron-mini.service"
	dpkg-deb --build ${package_name}
	rm -rf rincron-mini

xz: build
	mkdir -p "${package_name}/bin" "${package_name}/service/user" "${package_name}/service/system"
	cp "target/release/rincron_mini" "${package_name}/bin/rincron-mini"
	cp "assets/systemd/rincron-mini.service" "${package_name}/service/system/rincron-mini.service"
	cp "assets/systemd/rincron-mini.user.service" "${package_name}/service/user/rincron-mini.service"
	tar -cvJf ${package_name}.tar.xz ${package_name}
	rm -rf ${package_name}

clean:
	cargo clean
	rm -rf "rincron-mini"
