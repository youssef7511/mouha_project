import { readFileSync } from 'node:fs';
import { test, expect } from 'vitest';
const read = (p: string) => readFileSync(new URL(`../src-tauri/${p}`, import.meta.url), 'utf8');
test('all native phone commands are declared and granted only to the local window', () => {
  const commands = read('src/lib.rs')
    .match(/tauri::generate_handler!\[([^\]]+)\]/s)![1]
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  const manifest = read('build.rs');
  const cap = JSON.parse(read('capabilities/main.json'));
  expect(cap.windows).toEqual(['main']);
  expect(cap.remote).toBeUndefined();
  for (const name of commands) {
    expect(manifest).toContain(`"${name}"`);
    expect(cap.permissions).toContain(`allow-${name.replaceAll('_', '-')}`);
  }
  expect(read('src/lib.rs')).toContain('tauri::mobile_entry_point');
  expect(JSON.parse(read('tauri.conf.json')).identifier).toBe('com.packflow.mobile');
});
