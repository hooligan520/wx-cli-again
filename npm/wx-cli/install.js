#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');

const PLATFORM_PACKAGES = {
  'darwin-arm64': '@jackwener/wx-cli-darwin-arm64',
  'darwin-x64':   '@jackwener/wx-cli-darwin-x64',
  'linux-x64':    '@jackwener/wx-cli-linux-x64',
  'linux-arm64':  '@jackwener/wx-cli-linux-arm64',
  'win32-x64':    '@jackwener/wx-cli-win32-x64',
};

const platformKey = `${process.platform}-${process.arch}`;
const ext = process.platform === 'win32' ? '.exe' : '';

function chmod755(p) {
  if (process.platform === 'win32') return;
  fs.chmodSync(p, 0o755);
}

// 随包二进制（本地打包）：存在就完事，不需要平台包。
const bundled = path.join(__dirname, 'bin', 'native', `wx${ext}`);
if (fs.existsSync(bundled)) {
  chmod755(bundled);
  console.log(`wx-cli: using bundled binary ${bundled}`);
  process.exit(0);
}

const pkg = PLATFORM_PACKAGES[platformKey];
if (!pkg) {
  console.log(`wx-cli: no binary for ${platformKey}, skipping`);
  process.exit(0);
}

try {
  chmod755(require.resolve(`${pkg}/bin/wx${ext}`));
} catch {
  console.log(`wx-cli: platform package ${pkg} not installed`);
}
