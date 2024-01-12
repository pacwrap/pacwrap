#!/bin/bash
cd ./dist/$1/
tar acvf $1-$2-$3.tar.zst dist_src
SUM=($(sha512sum $1-$2-$3.tar.zst))
cp PKGBUILD PKGBUILD.tmp
sed -e "s/sha512sums=(.*)/sha512sums=('${SUM[0]}')/g;s/pkgver=.*/pkgver=$2/g;s/pkgrel=.*/pkgrel=$3/g" < "PKGBUILD.tmp" > "PKGBUILD"
makepkg -sf
rm -r src pkg PKGBUILD.tmp
