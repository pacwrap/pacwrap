BIN_DIR=/usr/bin

install:
	mkdir -p ${DESTDIR}${BIN_DIR}
	cp ./bin/pacwrap ${DESTDIR}${BIN_DIR}/pacwrap
	cp ./bin/pacwrap-create ${DESTDIR}${BIN_DIR}/pacwrap-create
	cp ./bin/pacwrap-exec ${DESTDIR}${BIN_DIR}/pacwrap-exec
	cp ./bin/pacwrap-sync ${DESTDIR}${BIN_DIR}/pacwrap-sync
	cp ./bin/pacwrap-utils ${DESTDIR}${BIN_DIR}/pacwrap-utils

uninstall:
	rm  ${DESTDIR}${BIN_DIR}/pachwrap
	rm  ${DESTDIR}${BIN_DIR}/pachwrap-create
	rm  ${DESTDIR}${BIN_DIR}/pachwrap-exec
	rm  ${DESTDIR}${BIN_DIR}/pachwrap-sync
	rm  ${DESTDIR}${BIN_DIR}/pachwrap-utils
