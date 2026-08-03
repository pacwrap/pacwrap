#!/usr/bin/env bash
#
#  pacwrap - prepare.sh
#
#  This script calls upon various binaries to build resources and package artifacts
# 
#  Copyright (C) 2023-2024 Xavier Moffett 
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

set -eEo pipefail

source ./dist/tools/common.sh
export ACTION_NOUN="Preparation"

validate_args "$1"
layout_dir
package "$DIST_SRC"/pacwrap-key "$DIST_BIN"/pacwrap-key "$1" 0

./dist/tools/schema.sh "$1"

# vim:set ts=4 sw=4 et:1
