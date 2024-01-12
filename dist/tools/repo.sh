echo "Preparing repo.."
./dist/tools/clean.sh
echo "Packaging.."
./dist/tools/package.sh pacwrap-base-dist $1 $2
echo "Building repo.."
cp ./dist/*/*.pkg.tar.zst ./dist/dist-repo/
repose pacwrap -zfr ./dist/dist-repo/
