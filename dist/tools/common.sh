#!/bin/bash
#
#  pacwrap - common.sh
#
#  Copyright (C) 2023-2024 Xavier R.M. 
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

[[ ! -z $COMMON_SCRIPT ]] && return

DIST_BIN="$PWD/dist/bin"
DIST_SRC="$PWD/dist/src"

if [[ -t 2 ]] && [[ ! -z $COLORTERM ]] && [[ $TERM != "dummy" ]]; then
    BOLD="[1m"
    RED="[1;31m"
 	GREEN="[1;32m"
    RESET="[0m"
fi	

error_fatal() {
	echo $BOLD$RED"error:$RESET $@";
	exit 1	
}

error() {
	echo $BOLD$RED"error:$RESET $@";	
}

packaged() {
	echo "$GREEN$BOLD    Packaged$RESET $@"
}

cleaned() {
	echo "$BOLD$GREEN     Cleaned$RESET $@"
}

validate_args() {
	[[ -z "$1" ]] && error_fatal "release target not specified."

    case $1 in
		release);; 
		debug)  ;;	
		*)      error_fatal "release target $1 is invalid.";;
	esac
}

layout_dir() { 
	[[ ! -d "$DIST_SRC" ]] && error_fatal "'$DIST_SRC': src directory not found."

	mkdir -p $DIST_BIN
}

#
# Populate version string in target file
#
# $1: Target File
# $2: Output File
# $3: Release
# $4: Inlcude date (optional)
#
package() {
	[[ ! -f "$1" ]] && error_fatal "'$1': file not found"
    ([[ -z $1 ]] || [[ -z $2 ]] || [[ -z $3 ]]) && error_fatal "Invalid arguments."

    local version=$(version $3 $4)
    local version_string=$(echo $version | head -n1 | sed -e 's/[]\/$*.^[]/\\&/g')
    local placeholder="version_string_placeholder"

    sed -e "s/$placeholder/$version_string/g" < $1 > $2
    packaged "${2##*/} v${version% (*}"
}

version() {
    eval $(cat pacwrap/Cargo.toml | grep version | head -n1 | sed -e "s/version = /local version=/g")

    if [[ ! -z "$(type -P git)" ]] && [[ -d ".git" ]]; then
        local git=$(git rev-parse --short HEAD)
        local release=
        local date=

        case $1 in
            release)    release="RELEASE"
                        date=$(git log -1 --date=format:%d/%m/%Y --format=%ad);;
            debug)      release="DEV"
                        date=$(date +'%d/%m/%Y %T%:z');;
        esac

        echo -n "$version-$git-$release"; [[ $2 ]] && echo -n " ($date)"
    else
        local unix_epoch=$(stat $DIST_SRC --print=%Y)
        local date=$(date +%d/%m/%Y --utc --date=@$unix_epoch)

        echo -n "$version"; [[ $2 ]] && echo -n " ($date)" 
    fi
}

COMMON_SCRIPT=1; readonly COMMON_SCRIPT BOLD RED GREEN RESET
