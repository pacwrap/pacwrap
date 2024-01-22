#!/bin/bash
#
#  pacwrap - repo.sh
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

DEST_REPO="./dist/repo/"

if [[ ! -d $DEST_REPO ]]; then
	mkdir -p $DEST_REPO
fi

./dist/tools/clean.sh repo
./dist/tools/package.sh pacwrap-base-dist $1 $2

if [[ ! -z "$PACWRAP_DIST_REPO" ]]; then
  repose pacwrap -vzfr ./dist/repo/
fi
