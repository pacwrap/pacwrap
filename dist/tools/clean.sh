#!/bin/bash
#
#  pacwrap - clean.sh
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

if ! [[ -z $COLORTERM ]] || [[ $TERM == "dummy" ]]; then	
	BOLD=$(tput bold)
	GREEN=$(tput setaf 2)
	RED=$(tput setaf 1)
	RESET=$(tput sgr0)
fi

DIST_RUNTIME="./dist/runtime"
DIST_BASE="./dist/pacwrap-base-dist"
DIST_REPO="./dist/repo"

runtime() {
	if [[ -d "$DIST_RUNTIME" ]]; then
		rm -r "$DIST_RUNTIME"
		mkdir -p "$DIST_RUNTIME"
		echo "$BOLD$GREEN     Cleaned$RESET container runtime"
	fi
}

repo() {
	if [[ -d "$DIST_REPO" ]] && [[ -d "$DIST_BASE" ]]; then
		rm "$DIST_REPO/*" "$DIST_BASE/*.tar.zst"
		echo "$BOLD$GREEN     Cleaned$RESET repositories"
	fi
}

invalid() {
	echo $BOLD$RED"error:$RESET Invalid parameter '$1'"
}

main() {
	for var in "$@"; do case $var in 
		repo) repo;;
		runtime) runtime;;
		*) invalid $var;;
	esac; done
}

main $@
