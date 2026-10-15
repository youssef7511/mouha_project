import { execFileSync } from 'node:child_process';
import { readdir, readFile, lstat, realpath } from 'node:fs/promises';
import path from 'node:path';

const forbidden = (name) => {
  const parts = name.replaceAll('\\', '/').split('/');
  const base = parts.at(-1).toLowerCase();
  return (
    parts.some((part) =>
      ['.dev-data', '.private-notes', 'backups', 'test-results'].includes(part),
    ) ||
    /\.(?:sqlite(?:-wal|-shm)?|db|pfbackup|xlsx|xls|p12|pfx|pem|key|mobileprovision)$/i.test(
      base,
    ) ||
    (base.startsWith('.env') && base !== '.env.example') ||
    base === 'demo-seed-v1.json' ||
    base === 'salaries_workbook_review.md'
  );
};

const repo = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();
const tracked = execFileSync('git', ['-C', repo, 'ls-files', '-z'], { encoding: 'utf8' })
  .split('\0')
  .filter(Boolean);
const rejected = tracked.filter(forbidden);
if (rejected.length) throw new Error(`Private data must not be tracked:\n${rejected.join('\n')}`);

for (const file of ['src-tauri/tauri.conf.json', 'src-tauri/tauri.test.conf.json']) {
  const config = JSON.parse(await readFile(file, 'utf8'));
  if (config.bundle?.resources && Object.keys(config.bundle.resources).length)
    throw new Error('Additional bundle resources require explicit review before release.');
}
const bundleIndex = process.argv.indexOf('--bundle');
if (bundleIndex !== -1) {
  const root = process.argv[bundleIndex + 1];
  if (!root || !(await lstat(root)).isDirectory())
    throw new Error('Application bundle is missing.');
  async function scan(directory) {
    for (const item of await readdir(directory, { withFileTypes: true })) {
      const full = path.join(directory, item.name);
      const relative = path.relative(root, full);
      if (forbidden(relative) || item.name === 'seed-demo.mjs')
        throw new Error(`Private/test data in bundle: ${relative}`);
      if (item.isSymbolicLink()) {
        const resolved = path.relative(await realpath(root), await realpath(full));
        if (resolved.startsWith('..') || path.isAbsolute(resolved))
          throw new Error(`Bundle symlink leaves the application: ${relative}`);
        continue;
      }
      if (item.isDirectory()) await scan(full);
    }
  }
  await scan(root);
}
console.log('Clean-source and bundle data checks passed.');
