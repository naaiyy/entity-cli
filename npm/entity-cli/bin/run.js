#!/usr/bin/env node
const { spawn } = require('child_process');
const { existsSync } = require('fs');
const { join } = require('path');

const binaryPath = join(__dirname, '..', 'dist', 'entity-cli');

if (!existsSync(binaryPath)) {
  console.error('[entity-cli] Native binary not found. The postinstall step may have failed.');
  console.error('[entity-cli] Try reinstalling or check your network, then run:');
  console.error('  npm rebuild entity-cli --foreground-scripts');
  process.exit(1);
}

const env = { ...process.env, ENTITY_CLI_EXECUTABLE: 'entitygraph-cli' };
const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  env
});

child.on('exit', (code) => process.exit(code ?? 0));


