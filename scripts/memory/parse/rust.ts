/** Parser — Rust module tree, `#[tauri::command]` functions, handler registry and plugins. */
import type { Config } from '../config';
import type { RustCommand, RustInfo, SourceFile } from '../types';

const MOD = /^\s*(pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;/gm;
const COMMAND = /#\[tauri::command[^\]]*\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)/g;
const HANDLER = /generate_handler!\s*\[([\s\S]*?)\]/;
const PLUGIN = /\.plugin\(\s*tauri_plugin_(\w+)/g;

/** `src-tauri/src/pdf/render.rs` → `pdf::render`; `mod.rs`, `lib.rs`, `main.rs` collapse to their parent. */
function modulePath(file: string, rustRoot: string): string {
  const parts = file.slice(rustRoot.length + 1).replace(/\.rs$/, '').split('/');
  if (['mod', 'lib', 'main'].includes(parts[parts.length - 1])) parts.pop();
  return parts.join('::');
}

export function parseRust(files: SourceFile[], config: Config): RustInfo {
  const rs = files.filter((f) => f.lang === 'rust');
  const mods: RustInfo['mods'] = [];
  const commands: RustCommand[] = [];
  for (const file of rs) {
    for (const m of file.content.matchAll(MOD)) mods.push({ file: file.path, name: m[2], public: !!m[1] });
    const prefix = modulePath(file.path, config.paths.rust);
    for (const m of file.content.matchAll(COMMAND)) {
      commands.push({ name: m[1], qualified: prefix ? `${prefix}::${m[1]}` : m[1], file: file.path });
    }
  }
  const entry = rs.find((f) => f.path === config.paths.rustEntry)?.content ?? '';
  const registered = (entry.match(HANDLER)?.[1] ?? '')
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
  const plugins = [...entry.matchAll(PLUGIN)].map((m) => m[1]);
  return { mods, commands, registered, plugins };
}
