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

COMMON_SCRIPT=1; readonly COMMON_SCRIPT BOLD RED GREEN RESET
