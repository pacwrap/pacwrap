#!/bin/bash
#
#  pacwrap - key.sh
#
#  This script packages pacwrap-key and defines version information within the script
# 
#  Copyright (C) 2023-2024 Xavier R.M. 
#  sapphirus(at)azorium(dot)net#
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

# 
# Environment variables
#
DIST_BIN="./dist/bin"
DIST_SRC="./dist/src/pacwrap-key"
DIST_PKG="./dist/bin/pacwrap-key"

#
# Main function
#
main() {
    validate_args $1
    prepare_and_validate
	package $1 
    packaged "pacwrap-key [$1]"
}

#
# Validate and prepare staging environment
#
prepare_and_validate() {
	[[ ! -f "$DIST_SRC" ]] && error_fatal "'$DIST_SRC': file not found"

    clean
	mkdir -p $DIST_BIN
}

#
# Clean build artifacts
#
clean() {
	if [[ -f "$DIST_PKG" ]]; then
		rm $DIST_PKG
		cleaned "pacwrap-key"
	fi
}

#
# Populate version string and package script
#
package() {
    local version_string=$(version_string $1 | head -n1 | sed -e 's/[]\/$*.^[]/\\&/g')
    local placeholder="version_string_placeholder"

    sed -e "s/$placeholder/$version_string/g" < $DIST_SRC > $DIST_PKG
}


version_string() {
    local git=$(git rev-parse --short HEAD)
    local release=
    local dev=
 
    case $1 in
        release)    release="RELEASE"
                    date=$(git log -1 --date=format:%d/%m/%Y --format=%ad);;
        debug)      release="DEV"
                    date=$(date +'%d/%m/%Y %T');;
    esac

    eval $(cat pacwrap/Cargo.toml | grep version | head -n1 | sed -e "s/version = /local version=/g")
    echo "$version-$git-$release ($date)"
}

main $@
