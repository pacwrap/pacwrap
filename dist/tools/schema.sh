#!/bin/bash
#
#  pacwrap - filesystem.sh
#
#  This script packages a filesystem skeleton with basic configuration and
#  scripting required to initialise a container.
# 
#  Copyright (C) 2023-2024 Xavier Moffett 
#  sapphirus(at)azorium(dot)net
#
#  This program is free software: you can redistribute it and/or modify
#  it under the terms of the GNU General Public License as published by
#  the Free Software Foundation, with only version 3 of the License.
#
#  This program is distributed in the hope that it will be useful,
#  but WITHOUT ANY WARRANTY; without even the implied warranty of
#  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#  GNU General Public License for more details.
#
#  You should have received a copy of the GNU General Public License
#  along with this program.  If not, see <https://www.gnu.org/licenses/>.
#

if [[ ! -d "$PWD/dist/tools/" ]]; then echo "This script may only be executed via the workspace root directory."; exit 2; fi
if [[ ! -f ./dist/tools/common.sh ]]; then echo "Common script is missing. Ensure the source tree is intact."; exit 2; fi

source ./dist/tools/common.sh
ACTION_NOUN="Schema generation"

# 
# Environment variables
#
USR_DIR="/usr"
ETC_DIR="/etc"
DEST_DIR="$PWD/dist/schema"

#
# Main function
#
main() {
    prepare_and_validate
	populate_usr
	populate_etc
	create_archive $1
    packaged "container schema [$1]"
}

#
# Validate and prepare staging environment
#
prepare_and_validate() {		
	clean
	mkdir -p $DEST_DIR$USR_DIR $DEST_DIR$ETC_DIR $DIST_BIN

	if [[ ! -d "$DEST_DIR$LIB_DIR" ]] || [[ ! -d $DEST_DIR$BIN_DIR ]]; then
		error_fatal "'$DEST_DIR': directory not found."
	fi

    if [[ ! -d "$DIST_SRC" ]]; then
		error_fatal "'$DIST_SRC': directory not found."
	fi
}

#
# Clean build artifacts
#
clean() {
	if [[ -d "$DEST_DIR" ]]; then
		rm -r "$DEST_DIR"
		mkdir -p "$DEST_DIR"
		cleaned "container schema"
	fi
}

#
# Populate container skeleton archive
#
create_archive() {
    cd $DEST_DIR
	tar acf ../bin/filesystem.tar.zst *
}

#
# Populate usr for container filesystem
#
populate_usr() {
	mkdir -p "${DEST_DIR}/usr/share/libalpm/hooks/" \
		"${DEST_DIR}/usr/share/libalpm/scripts/" \
		"${DEST_DIR}/usr/local/bin" \
	    "${DEST_DIR}/usr/lib/"


	ln -s /usr/lib/flatpak-xdg-utils/xdg-open "${DEST_DIR}/usr/local/bin/"
	ln -s /usr/lib/flatpak-xdg-utils/xdg-email "${DEST_DIR}/usr/local/bin/"

    install -Dm 644 "$DIST_SRC/0-pacwrap-dist.hook" "${DEST_DIR}/usr/share/libalpm/hooks/0-pacwrap-dist.hook" 
    install -Dm 644 "$DIST_SRC/1-pacwrap-dist.hook" "${DEST_DIR}/usr/share/libalpm/hooks/1-pacwrap-dist.hook" 
    install -Dm 644 "$DIST_SRC/42-trust-permission.hook" "${DEST_DIR}/usr/share/libalpm/hooks/42-trust-permission.hook"
    # TODO: Perhaps identify ourselves as our own distribution of Arch Linux?
    # install -Dm 644 "$DIST_SRC/os-release" "${DEST_DIR}/usr/lib/os-release"
    install -Dm 755 "$DIST_SRC/pacwrap-dist" "${DEST_DIR}/usr/share/libalpm/scripts/pacwrap-dist"
    install -Dm 755 "$DIST_BIN/pacwrap-key" "${DEST_DIR}/usr/bin/pacwrap-key"
}

#
# Populate etc for container filesystem 
#
populate_etc() {
	local pacman_hooks=('20-systemd-sysusers'
			    '30-systemd-tmpfiles' 
			    '30-systemd-daemon-reload-system'
			    '30-systemd-daemon-reload-user'
			    '30-systemd-sysctl'
			    '30-systemd-catalog'
			    '30-systemd-update'
		    	'30-systemd-udev-reload'
			    '30-systemd-hwdb'
		    	'dbus-reload')
	
	# Systemd cannot be started securely in an unprivileged namespace, therefore
	# disable unnecessary systemd hooks in order to speed up transaction times.
	mkdir -p "${DEST_DIR}/etc/pacman.d/hooks/" "${DEST_DIR}/usr/local/bin/"
	for pacman_hook in ${pacman_hooks[@]}; do
		ln -s /dev/null "${DEST_DIR}/etc/pacman.d/hooks/${pacman_hook}.hook"; done

	# Provide our own /etc/bash.bashrc	
	cp "$DIST_SRC/bash.bashrc" "$DEST_DIR$ETC_DIR"
}

main $@
