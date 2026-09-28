/**
 * Parser — frontend → Rust calls. Matches every callee name in `config.ipcCallees` (default
 * `['invoke', 'backendCall']`, 21.02-F F-6): `invoke('cmd')` / `invoke<T>('cmd')` and
 * `backendCall('cmd', ...)` / `backendCall<K>('cmd', ...)` — generics may nest `<>`.
 */
import type { Config } from '../config';
import type { IpcCall, SourceFile } from '../types';
import { scriptOf } from './tsImports';

/** From just after the callee name, skip an optional balanced `<...>` and return the quoted first-argument command name. */
function commandAfter(src: string, start: number): string | null {
  let i = start;
  while (/\s/.test(src[i] ?? '')) i++;
  if (src[i] === '<') {
    let depth = 0;
    for (; i < src.length; i++) {
      if (src[i] === '<') depth++;
      else if (src[i] === '>' && src[i - 1] !== '=' && --depth === 0) break;
    }
    i++;
  }
  return src.slice(i).match(/^\s*\(\s*['"`]([\w:]+)['"`]/)?.[1] ?? null;
}

export function parseIpc(files: SourceFile[], config: Config): IpcCall[] {
  const calls: IpcCall[] = [];
  const calleePattern = new RegExp(`\\b(?:${config.ipcCallees.map((n) => n.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')})\\b`, 'g');
  for (const file of files.filter((f) => f.lang === 'ts' || f.lang === 'vue')) {
    const src = scriptOf(file);
    for (const m of src.matchAll(calleePattern)) {
      const command = commandAfter(src, m.index + m[0].length);
      if (command) calls.push({ file: file.path, command });
    }
  }
  return calls;
}
