#!/bin/zsh
set -eu
cd "$(dirname "$0")"
profile="${1:-dev}"
case "$profile" in
  dev) output=debug ;;
  release) output=release ;;
  *) print -u2 'Usage: ./build-mac.sh [dev|release]'; exit 2 ;;
esac
# Only generated Cargo artifacts live here. Keep repeated development builds
# from silently filling the disk; cargo clean takes Cargo's build lock.
trim_build_cache() {
  if [[ -d target ]]; then
    local cache_kib=$(du -sk target | awk '{print $1}')
    if (( cache_kib > 12 * 1024 * 1024 )); then
      print 'Build cache exceeds 12 GiB; clearing generated Cargo artifacts.'
      cargo clean
    fi
  fi
}
trim_build_cache
cargo build --profile "$profile" --locked
mkdir -p 'Nord Flight.app/Contents/MacOS'
mkdir -p 'Nord Flight.app/Contents/Resources'
cp packaging/Info.plist 'Nord Flight.app/Contents/Info.plist'
cp packaging/NordFlight.icns 'Nord Flight.app/Contents/Resources/NordFlight.icns'
rsync -a --delete --exclude '/branding/icon-options/' --exclude '/branding/*proposed*' assets/ 'Nord Flight.app/Contents/Resources/assets/'
cp "target/$output/gripen-flight" 'Nord Flight.app/Contents/MacOS/gripen-flight.next'
strip -x 'Nord Flight.app/Contents/MacOS/gripen-flight.next'
mv -f 'Nord Flight.app/Contents/MacOS/gripen-flight.next' 'Nord Flight.app/Contents/MacOS/gripen-flight'
codesign --force --sign - 'Nord Flight.app'
trim_build_cache
