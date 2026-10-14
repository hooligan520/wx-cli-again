#!/usr/bin/env bash
#
# 把本地编译出来的 wx 二进制打成可直接 `npm install -g` 的包，
# 用来「用 fork 覆盖全局 @jackwener/wx-cli」。
#
#   ./npm/pack-local.sh                 # 用 target/release/wx
#   ./npm/pack-local.sh /path/to/wx     # 指定二进制
#   ./npm/pack-local.sh --install       # 打包完顺手 npm install -g
#
# 说明：
#   npm 上 @jackwener/wx-cli 只发到 0.3.0，`npm/` 里的 optionalDependencies
#   指的是尚未发布的 0.6.x 平台包，所以直接装会拿不到二进制。
#   本脚本把二进制放进 `npm/wx-cli/bin/native/`，`bin/wx.js` 会优先用它。
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PKG_DIR="$ROOT/npm/wx-cli"
NATIVE_DIR="$PKG_DIR/bin/native"

BIN=""
DO_INSTALL=0
for arg in "$@"; do
  case "$arg" in
    --install) DO_INSTALL=1 ;;
    -h|--help) sed -n '2,14p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) BIN="$arg" ;;
  esac
done

if [ -z "$BIN" ]; then
  BIN="$ROOT/target/release/wx"
fi

if [ ! -f "$BIN" ]; then
  echo "找不到二进制: $BIN" >&2
  echo "先编译： cargo build --release" >&2
  exit 1
fi

# ---- 平台 key（决定 bin/native/wx 还是 wx.exe） ----
case "$(uname -s)" in
  Darwin) OS=darwin ;;
  Linux)  OS=linux ;;
  *)      OS=win32 ;;
esac
case "$(uname -m)" in
  arm64|aarch64) ARCH=arm64 ;;
  x86_64|amd64)  ARCH=x64 ;;
  *)             ARCH="$(uname -m)" ;;
esac
# macOS 上 arm64 对应 darwin-arm64；linux 用 arm64 而不是 aarch64
KEY="$OS-$ARCH"

EXT=""
if [ "$OS" = "win32" ]; then EXT=".exe"; fi

# ---- 版本号与 Cargo.toml 对齐，避免包版本漂移 ----
CARGO_VER="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
if [ -n "$CARGO_VER" ]; then
  for f in "$PKG_DIR/package.json" "$ROOT"/npm/platforms/*/package.json; do
    [ -f "$f" ] || continue
    cur="$(sed -n 's/.*"version": *"\([^"]*\)".*/\1/p' "$f" | head -1)"
    if [ "$cur" != "$CARGO_VER" ]; then
      python3 - "$f" "$CARGO_VER" <<'PY'
import json, sys
p, ver = sys.argv[1], sys.argv[2]
with open(p, encoding='utf-8') as fp:
    d = json.load(fp)
d['version'] = ver
with open(p, 'w', encoding='utf-8') as fp:
    json.dump(d, fp, indent=2, ensure_ascii=False)
    fp.write('\n')
PY
      echo "版本已同步: $f -> $CARGO_VER"
    fi
  done
fi

mkdir -p "$NATIVE_DIR"
cp -f "$BIN" "$NATIVE_DIR/wx$EXT"
chmod 755 "$NATIVE_DIR/wx$EXT"

SHA="$(shasum -a 256 "$NATIVE_DIR/wx$EXT" | awk '{print $1}')"
echo "已打包: $NATIVE_DIR/wx$EXT"
echo "  平台   : $KEY"
echo "  版本   : ${CARGO_VER:-unknown}"
echo "  sha256 : $SHA"

if [ "$DO_INSTALL" = "1" ]; then
  echo
  echo "==> npm install -g $PKG_DIR"
  npm install -g "$PKG_DIR"
  echo
  echo "==> which -a wx"
  which -a wx || true
  echo
  echo "==> wx --version"
  wx --version || true
else
  echo
  echo "安装： npm install -g $PKG_DIR"
fi
