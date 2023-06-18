BIN_DIR=/usr/bin

install:
	mkdir -p ${DESTDIR}${BIN_DIR}
	cp ./target/release/pacwrap ${DESTDIR}${BIN_DIR}/pacwrap
	cp ./bin/pacwrap-common ${DESTDIR}${BIN_DIR}/pacwrap-common
	cp ./bin/pacwrap-create ${DESTDIR}${BIN_DIR}/pacwrap-create
	cp ./bin/pacwrap-sync ${DESTDIR}${BIN_DIR}/pacwrap-sync
	cp ./bin/pacwrap-utils ${DESTDIR}${BIN_DIR}/pacwrap-utils
	cp ./bin/pacwrap-ps ${DESTDIR}${BIN_DIR}/pacwrap-ps
	cp ./bin/pacwrap-man ${DESTDIR}${BIN_DIR}/pacwrap-man


uninstall:
	rm  ${DESTDIR}${BIN_DIR}/pacwrap
	rm  ${DESTDIR}${BIN_DIR}/pacwrap-common
	rm  ${DESTDIR}${BIN_DIR}/pacwrap-create
	rm  ${DESTDIR}${BIN_DIR}/pacwrap-sync
	rm  ${DESTDIR}${BIN_DIR}/pacwrap-utils
	rm  ${DESTDIR}${BIN_DIR}/pacwrap-ps
	rm  ${DESTDIR}${BIN_DIR}/pacwrap-man
