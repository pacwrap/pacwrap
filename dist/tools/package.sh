#!/usr/bin/env bash
#
#  pacwrap - package.sh
#
#  This script calls upon various binaries to build resources and package artifacts
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

set -eEo pipefail

source ./dist/tools/common.sh
export ACTION_NOUN="Packaging"

DIST_MAN="$PWD/dist/man"

validate_args "$1"
package "$DIST_MAN/pacwrap.1" "$DIST_BIN/pacwrap.1" "$1"
package "$DIST_MAN/pacwrap.yml.2" "$DIST_BIN/pacwrap.yml.2" "$1"

./dist/tools/runtime.sh "$1"

# vim:set ts=4 sw=4 et:1
