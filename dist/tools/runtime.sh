#!/bin/bash
#
#  pacwrap - runtime.sh
#
#  This script packages the most minimal userspace environment possible 
#  allowing pacwrap's agent binary to execute in an otherwise empty container.
# 
#  Copyright (C) 2023 Xavier R.M. 
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

BOLD=$(tput bold)
RED=$(tput setaf 1)
GREEN=$(tput setaf 2)
RESET=$(tput sgr0)
LIB_DIR="/lib"
BIN_DIR="/bin"
DEST_DIR="./dist/runtime"

# Validate parameters

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

# Cleanup and prepare container runtime

./dist/tools/clean.sh runtime 0> /dev/null
mkdir -p $DEST_DIR$LIB_DIR $DEST_DIR$BIN_DIR

# Validate preparation environment

if [[ ! -d "$DEST_DIR$LIB_DIR" ]] || [[ ! -d $DEST_DIR$BIN_DIR ]]; then
	echo $BOLD$RED"error:$RESET '$DEST_DIR': directory not found.";	
	exit 1
fi

# Populate libraries for container runtime

ldd ./target/$1/pacwrap-agent | sed -e "s/.*=> //g;s/ (.*)//g;s/\t.*//g" | xargs cp -Lt $DEST_DIR$LIB_DIR
ldd /usr/bin/gpg | sed -e "s/.*=> //g;s/ (.*)//g;s/\t.*//g" | xargs cp -Lt $DEST_DIR$LIB_DIR
ldd /usr/bin/bash | sed -e "s/.*=> //g;s/ (.*)//g;s/\t.*//g" | xargs cp -Lt $DEST_DIR$LIB_DIR
cp -L /usr/lib/libfakeroot/libfakeroot.so $DEST_DIR$LIB_DIR
cp -L /usr/lib/libfakeroot/fakechroot/libfakechroot.so $DEST_DIR$LIB_DIR

# Populate binaries for container runtime

ln -s ../lib64/ld-linux-x86-64.so.2 $DEST_DIR$BIN_DIR/ld.so
cp ./target/$1/pacwrap-agent $DEST_DIR$BIN_DIR/agent
cp /usr/bin/gpg $DEST_DIR$BIN_DIR/gpg

echo "$GREEN$BOLD    Packaged$RESET container runtime [$1]"
