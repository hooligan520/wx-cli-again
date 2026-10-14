#!/usr/bin/env node
'use strict';

const { execFileSync } = require('child_process');
const path = require('path');
const fs = require('fs');

const PLATFORM_PACKAGES = {
  'darwin-arm64': '@jackwener/wx-cli-darwin-arm64',
  'darwin-x64':   '@jackwener/wx-cli-darwin-x64',
  'linux-x64':    '@jackwener/wx-cli-linux-x64',
  'linux-arm64':  '@jackwener/wx-cli-linux-arm64',
  'win32-x64':    '@jackwener/wx-cli-win32-x64',
};

const platformKey = `${process.platform}-${process.arch}`;
const ext = process.platform === 'win32' ? '.exe' : '';

/**
 * 随包分发的原生二进制（`bin/native/wx`）。
 *
 * 由 `npm/pack-local.sh` 从 `target/release/wx` 拷进来，用于
 * 「用本地编译的 fork 覆盖全局 @jackwener/wx-cli」——既不需要联网、
 * 也不需要先把平台包发布到 registry。
 */
function bundledBinary() {
  const p = path.join(__dirname, 'native', `wx${ext}`);
  if (!fs.existsSync(p)) return null;
  if (process.platform !== 'win32') {
    try { fs.chmodSync(p, 0o755); } catch { /* 只读挂载等场景忽略 */ }
  }
  return p;
}

function platformBinary() {
  const pkg = PLATFORM_PACKAGES[platformKey];
  if (!pkg) {
    console.error(`wx-cli: unsupported platform ${platformKey}`);
    process.exit(1);
  }

  try {
    return require.resolve(`${pkg}/bin/wx${ext}`);
  } catch { /* 落到下面的兜底 */ }

  try {
    const modPath = path.join(
      path.dirname(require.resolve(`${pkg}/package.json`)),
      `bin/wx${ext}`
    );
    if (fs.existsSync(modPath)) return modPath;
  } catch { /* 平台包未安装 */ }

  console.error(`wx-cli: binary not found for ${platformKey}`);
  console.error('Try: npm install -g @jackwener/wx-cli');
  console.error('Or, for a local build: npm/pack-local.sh && npm install -g npm/wx-cli');
  process.exit(1);
}

function getBinaryPath() {
  // 1) 显式覆盖优先 —— 多版本切换 / 测试用
  if (process.env.WX_CLI_BINARY) {
    const p = process.env.WX_CLI_BINARY;
    if (fs.existsSync(p)) return p;
    console.error(`wx-cli: WX_CLI_BINARY 指向的文件不存在: ${p}`);
    process.exit(1);
  }

  // 2) 随包二进制（本地打包安装）
  const bundled = bundledBinary();
  if (bundled) return bundled;

  // 3) 平台包（npm 正式发布形态）
  return platformBinary();
}

try {
  execFileSync(getBinaryPath(), process.argv.slice(2), {
    stdio: 'inherit',
    env: { ...process.env },
  });
} catch (e) {
  if (e && e.status != null) process.exit(e.status);
  throw e;
}
