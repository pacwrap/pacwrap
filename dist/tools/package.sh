#!/bin/bash
#
#  pacwrap - package.sh
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

cd ./dist/$1/
tar acvf $1-$2-$3.tar.zst dist_src
SUM=($(sha512sum $1-$2-$3.tar.zst))
cp PKGBUILD PKGBUILD.tmp
sed -e "s/sha512sums=(.*)/sha512sums=('${SUM[0]}')/g;s/pkgver=.*/pkgver=$2/g;s/pkgrel=.*/pkgrel=$3/g" < "PKGBUILD.tmp" > "PKGBUILD"
makepkg -scf --config ../config/makepkg.conf 
cp $1-$2-$3-any.pkg.tar.zst ../dist-repo/
rm PKGBUILD.tmp
