#!/bin/bash
#
#  pacwrap - clean.sh
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

if [[ ! -d "$PWD/dist/tools/" ]]; then echo "This script may only be executed via the workspace root directory."; exit 2; fi
if [[ ! -f ./dist/tools/common.sh ]]; then echo "Common script is missing. Ensure the source tree is intact."; exit 2; fi

source ./dist/tools/common.sh

DIST_BIN="$PWD/dist/bin"
DIST_RUNTIME="$PWD/dist/runtime"
DIST_SCHEMA="$PWD/dist/schema"

runtime() {
	if [[ -d "$DIST_RUNTIME" ]]; then
		rm -r "$DIST_RUNTIME"
		mkdir -p "$DIST_RUNTIME"
		cleaned "container runtime"
    fi
}

filesystem() {
	if [[ -d "$DIST_SCHEMA" ]]; then
		rm -r "$DIST_SCHEMA"
		mkdir -p "$DIST_SCHEMA"
        cleaned "container schema"
    fi
}

bin() {
	if [[ -d "$DIST_BIN" ]]; then
		rm -r "$DIST_BIN"
		mkdir -p "$DIST_BIN"
        cleaned "bin artifacts"
    fi
}

main() {
	for var in "$@"; do case $var in
		schema)     filesystem;;
		runtime)    runtime;;
        bin)        bin;;
        all)        bin
                    filesystem
					runtime;;
		*)          error_fatal "Invalid parameter '$1'";;
	esac; done
}

main $@
