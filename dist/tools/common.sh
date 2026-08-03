#!/usr/bin/env bash
#
#  pacwrap - common.sh
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

[[ -n $COMMON_SCRIPT ]] && return

if [[ ! -d "$PWD/dist/tools/" ]]; then 
    echo "This script may only be executed via the workspace root directory."
    exit 2
fi

trap 'handle_failure "$ACTION_NOUN" $?' ERR TERM INT

ACTION_NOUN="Common script"
DIST_BIN="$PWD/dist/bin"
DIST_SRC="$PWD/dist/src"

if [[ -t 2 ]] && [[ -n $COLORTERM ]] && [[ $TERM != "dummy" ]] || [[ $PACWRAP_CI == 1 ]]; then
    BOLD="[1m"
    RED="[1;31m"
    GREEN="[1;32m"
    RESET="[0m"
fi

handle_failure() {
    error_fatal "$1 failure: $BASH_COMMAND exited with exit code $?."
}

error_fatal() {
    echo "$BOLD${RED}error:$RESET $*"
    exit 1	
}

error() {
    echo "$BOLD${RED}error:$RESET $*"
}

packaged() {
    echo "$GREEN$BOLD    Packaged$RESET $*"
}

cleaned() {
    echo "$BOLD$GREEN     Cleaned$RESET $*"
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

    mkdir -p "$DIST_BIN"
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
    [[ -z $1 ]] || [[ -z $2 ]] || [[ -z $3 ]] && error_fatal "Invalid arguments."

    local version version_string
    local placeholder="version_string_placeholder"

    version=$(version "$3" "$4")
    version_string=$(echo "$version" | sed -e 's/[]\/$*.^[]/\\&/g')
    [[ -z "$version" ]] || [[ -z "$version_string" ]] && error_fatal "Version string undeclared"

    sed -e "s/$placeholder/$version_string/g" < "$1" > "$2"
    packaged "${2##*/} v${version% (*}"
}

version() {
    eval "$(grep -m 1 version Cargo.toml | sed -e "s/version = /local version=/g")"

    [[ -z "$version" ]] && return

    if [[ -n "$(type -P git)" ]] && [[ -d ".git" ]]; then
        local git tag release date

        git=$(git rev-parse --short HEAD)
        tag=$(git tag --points-at)

        case $1 in
            release)    
                release="RELEASE"
                date=$(git log -1 --date=format:%d/%m/%Y --format=%cd);;
            debug)      
                release="DEV"
                date=$(date +'%d/%m/%Y %T%:z');;
        esac

        if [[ -z "$tag" ]]; then
            echo -n "$version-$git-$release"; [[ $2 ]] && echo -n " ($date)"
        else
            echo -n "$version"; [[ $2 ]] && echo -n " ($date)"
        fi
    else
        local unix_epoch date

        unix_epoch=$(stat "$DIST_SRC" --print=%Y)
        date=$(date +%d/%m/%Y --utc --date=@"$unix_epoch")

        echo -n "$version"; [[ $2 ]] && echo -n " ($date)" 
    fi

    # Ensure this function returns zero
    echo
}

COMMON_SCRIPT=1; readonly COMMON_SCRIPT BOLD RED GREEN RESET

# vim:set ts=4 sw=4 et:1
