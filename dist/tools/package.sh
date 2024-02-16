#!/bin/bash
#
#  pacwrap - package.sh
#
#  This script calls upon various binaries to build resources and package artifacts
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

if [[ ! -d "$PWD/dist/tools/" ]]; then echo "This script may only be executed via the workspace root directory."; exit 2; fi
if [[ ! -f ./dist/tools/common.sh ]]; then echo "Common script is missing. Ensure the source tree is intact."; exit 2; fi

source ./dist/tools/common.sh

validate_args $1
./target/$1/pacwrap --help=all --format=man > ./dist/bin/pacwrap.1; if [[ $? != 0 ]]; then error_fatal "Generation of manpage failed."; fi
./target/$1/pacwrap --help=pacwrap.yml --format=man > ./dist/bin/pacwrap.yml.2; if [[ $? != 0 ]]; then error_fatal "Generation of manpage failed"; fi
./dist/tools/runtime.sh $1; if [[ $? != 0 ]]; then error_fatal "Build of container runtime failed."; fi
