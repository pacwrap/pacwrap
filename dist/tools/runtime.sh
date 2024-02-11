#!/bin/bash
#
#  pacwrap - runtime.sh
#
#  This script packages the most minimal userspace environment possible 
#  allowing pacwrap's agent binary to execute in an otherwise empty container.
# 
#  Copyright (C) 2023-2024 Xavier R.M. 
#  sapphirus(at)azorium(dot)net
#
#
#    This program is free software: you can redistribute it and/or modify
#    it under the terms of the GNU General Public License as published by
#    the Free Software Foundation, with only version 3 of the License.
#
#    This program is distributed in the hope that it will be useful,
#    but WITHOUT ANY WARRANTY; without even the implied warranty of
#    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#    GNU General Public License for more details.
#
#    You should have received a copy of the GNU General Public License
#    along with this program.  If not, see <https://www.gnu.org/licenses/>.

if ! [[ -z $COLORTERM ]] || [[ $TERM == "dummy" ]]; then
	BOLD=$(tput bold)
	RED=$(tput setaf 1)
	GREEN=$(tput setaf 2)
	RESET=$(tput sgr0)
fi

# 
# Environment variables
#
LIB_DIR="/lib"
BIN_DIR="/bin"
ETC_DIR="/etc"
DEST_DIR="$PWD/dist/runtime"
DIST_SRC="$PWD/dist/src"
FAKEROOT="/libfakeroot"
FAKEROOT_DIR="/usr/lib/libfakeroot"
FAKEROOT_SRC="$FAKEROOT_DIR/libfakeroot.so"
FAKEROOT_DEST="$DEST_DIR$LIB_DIR$FAKEROOT"
FAKECHROOT="/fakechroot"
FAKECHROOT_SRC="$FAKEROOT_DIR$FAKECHROOT/libfakechroot.so"
FAKECHROOT_DEST="$DEST_DIR$LIB_DIR$FAKEROOT$FAKECHROOT"
PROFILE_PS1="PS1='$(echo '$USER \\W>\\$') '";

# !! File an issue/PR if there's an incompatibility !!
#
# Array of bin utilities to include within the runtime environment
#
BIN_UTILS="bash busybox faked fakeroot find gpg grep getopt sed"
#
# Array of coreutils to include within the runtime environment
#
COREUTILS="cat chgrp chmod chown chroot cp cut dd df dir du head id install link ln ls mkdir mktemp mv pathchk pwd readlink realpath rm rmdir shred sort split stat sum tail tee touch tr truncate tsort unlink wc"

#
# Copy libraries
#
# $@: takes an array of system library paths
#
copy_libs() {
	for path in ${@}; do 
		ldd $path | sed -e "s/.*=> //g;s/ (.*)//g;s/\t.*//g" | xargs cp -Lt $DEST_DIR$LIB_DIR
    done
}

#
# Copy binaries
#
# $@: takes an array of system binaries located in /usr/bin
#
copy_bins() {
	for bin in ${@}; do 
		cp /usr/bin/$bin $DEST_DIR$BIN_DIR/$bin

        # Remove debuglink section, to ensure the Arch Build System doesn't complain 
        objcopy --remove-section=.gnu_debuglink $DEST_DIR$BIN_DIR/$bin 2>/dev/null
    done	
}

#
# Validate and prepare staging environment
#
prepare_and_validate() {
	clean
	mkdir -p $DEST_DIR$LIB_DIR$FAKEROOT$FAKECHROOT $DEST_DIR$BIN_DIR $DEST_DIR$ETC_DIR

	if [[ ! -d "$DEST_DIR$LIB_DIR" ]] || [[ ! -d $DEST_DIR$BIN_DIR ]]; then
		echo $BOLD$RED"error:$RESET '$DEST_DIR': directory not found.";	
		exit 1
	fi
}

#
# Clean build artifacts
#
clean() {
	if [[ -d "$DEST_DIR" ]]; then
		rm -r "$DEST_DIR"
		mkdir -p "$DEST_DIR"
		echo "$BOLD$GREEN     Cleaned$RESET container runtime"
	fi
}

#
# Populate libraries for container runtime
#
populate_lib() {
	copy_libs ./target/$1/pacwrap-agent /usr/bin/gpg /usr/bin/bash /usr/bin/ls /usr/bin/grep
	cp -L $FAKEROOT_SRC $FAKEROOT_DEST
	cp -L $FAKECHROOT_SRC $FAKECHROOT_DEST
	ln -s .$FAKEROOT/libfakeroot.so $DEST_DIR$LIB_DIR/libfakeroot.so
	ln -s .$FAKEROOT$FAKECHROOT/libfakechroot.so $DEST_DIR$LIB_DIR/libfakechroot.so

    # Remove debuglink section, to ensure the Arch Build System doesn't complain
    for lib in $(find $DEST_DIR$LIB_DIR -maxdepth 3 -printf "%p "); do
        objcopy --remove-section=.gnu_debuglink $lib 2>/dev/null
    done
}

#
# Populate binaries for container runtime 
#
populate_bin() {
	cp ./target/$1/pacwrap-agent $DEST_DIR$BIN_DIR/agent
	copy_bins $BIN_UTILS $COREUTILS 
	ln -s bash $DEST_DIR$BIN_DIR/sh
	ln -s ld-linux-x86-64.so.2 $DEST_DIR$BIN_DIR/ld-linux.so.2
	ln -s ../lib64/ld-linux-x86-64.so.2 $DEST_DIR$BIN_DIR/ld.so
}

#
# Populate /etc directory for container runtime
#
populate_etc() {
	echo -e "#\n# /etc/bash.bashrc\n#\n# pacwrap runtime\n#\n\n${PROFILE_PS1}\nbind -x $'\"\\C-l\":clear;'\ncd \$HOME\n" > $DEST_DIR$ETC_DIR/bash.bashrc
	sed -n 12,20p $DIST_SRC/bash.bashrc >> $DEST_DIR$ETC_DIR/bash.bashrc
	echo -e "#\n# /etc/profile - busybox env\n#\n# pacwrap runtime\n#\n\n$PROFILE_PS1\n" > $DEST_DIR$ETC_DIR/profile
	echo -e 'printf "\033]0;%s@%s\007" "${USER}" "${HOSTNAME%%.*}"\ncd $HOME' >> $DEST_DIR$ETC_DIR/profile
}

#
# Populate busybox links
#
busybox_links() {
	for applet in $(busybox --list); do 
		ln -s busybox ./dist/runtime/bin/$applet 2>/dev/null
	done
}

#
# Main function
#
main() {
	if [[ -z $1 ]]; then
		echo $BOLD$RED"error:$RESET target not specified.";
		exit 1
	fi

	case $1 in
		release);; 
		debug);;	
		*)	echo $BOLD$RED"error:$RESET target $1 is invalid.";
			exit 1;;
	esac

	prepare_and_validate
	populate_lib $1
	populate_bin $1
	populate_etc
	busybox_links

	echo "$GREEN$BOLD    Packaged$RESET container runtime [$1]"
}

main $@
