#!/usr/bin/env node
const { spawn } = require('child_process');
const { existsSync } = require('fs');
const { join } = require('path');

const binaryPath = join(__dirname, '..', 'dist', 'entity-cli');

if (!existsSync(binaryPath)) {
  console.error('[@entityauth/cli] Native binary not found. The postinstall step may have failed.');
  console.error('[@entityauth/cli] Try reinstalling or check your network, then run:');
  console.error('  npm rebuild @entityauth/cli --foreground-scripts');
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit'
});

child.on('exit', (code) => process.exit(code ?? 0));


