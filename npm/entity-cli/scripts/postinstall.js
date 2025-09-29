const fs = require('fs');
const path = require('path');
const https = require('https');

const pkg = require('../package.json');

function fail(message) {
  console.error(`[entity-cli] ${message}`);
  process.exit(1);
}

if (process.platform !== 'darwin' || process.arch !== 'arm64') {
  fail('This package only supports macOS Apple Silicon (darwin/arm64).');
}

const version = pkg.version;
const ownerRepo = 'Entity-CLI/entity-cli';
const assetName = 'entity-cli-darwin-arm64';
const url = `https://github.com/${ownerRepo}/releases/download/v${version}/${assetName}`;

const distDir = path.join(__dirname, '..', 'dist');
const outPath = path.join(distDir, 'entity-cli');

function mkdirp(p) {
  fs.mkdirSync(p, { recursive: true });
}

function downloadWithRedirect(url, dest, cb, redirects = 0) {
  if (redirects > 5) return cb(new Error('Too many redirects'));
  https.get(url, (res) => {
    if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
      const next = res.headers.location.startsWith('http')
        ? res.headers.location
        : new URL(res.headers.location, url).toString();
      res.resume();
      return downloadWithRedirect(next, dest, cb, redirects + 1);
    }
    if (res.statusCode !== 200) {
      return cb(new Error(`Unexpected status: ${res.statusCode}`));
    }
    const file = fs.createWriteStream(dest);
    res.pipe(file);
    file.on('finish', () => file.close(cb));
  }).on('error', (err) => cb(err));
}

try {
  mkdirp(distDir);
  const tmp = `${outPath}.tmp`;
  downloadWithRedirect(url, tmp, (err) => {
    if (err) {
      fail(`Failed to download binary from ${url}: ${err.message}`);
    }
    try {
      fs.chmodSync(tmp, 0o755);
      fs.renameSync(tmp, outPath);
      console.log(`[entity-cli] Installed native binary to ${outPath}`);
      process.exit(0);
    } catch (e) {
      fail(`Failed to finalize binary install: ${e.message}`);
    }
  });
} catch (e) {
  fail(e.message);
}


