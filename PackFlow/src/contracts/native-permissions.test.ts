import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const source = (file: string) =>
  readFileSync(new URL(`../../src-tauri/${file}`, import.meta.url), 'utf8');

describe('desktop command permissions', () => {
  it('declares and grants every native handler to the main window', () => {
    const handlers = source('src/lib.rs').match(/tauri::generate_handler!\[([^\]]+)\]/s);
    expect(handlers).not.toBeNull();
    const commands = handlers![1]
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    const declared = source('build.rs').match(/\.commands\(&\[([^\]]+)\]/s);
    expect(declared).not.toBeNull();
    const manifestCommands = [...declared![1].matchAll(/"([a-z_]+)"/g)].map((m) => m[1]);
    const capability = JSON.parse(source('capabilities/main.json'));
    expect(capability.windows).toEqual(['main']);
    for (const command of commands) {
      expect(manifestCommands, `${command} missing from build manifest`).toContain(command);
      expect(capability.permissions, `${command} unavailable to main window`).toContain(
        `allow-${command.replaceAll('_', '-')}`,
      );
    }
    expect([...manifestCommands].sort()).toEqual([...commands].sort());
  });
});
