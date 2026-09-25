/** Parser — frontend → Rust calls: `invoke('cmd')` / `invoke<T>('cmd')`, generics may nest `<>`. */
import type { IpcCall, SourceFile } from '../types';
import { scriptOf } from './tsImports';

/** From just after `invoke`, skip an optional balanced `<...>` and return the quoted command name. */
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

export function parseIpc(files: SourceFile[]): IpcCall[] {
  const calls: IpcCall[] = [];
  for (const file of files.filter((f) => f.lang === 'ts' || f.lang === 'vue')) {
    const src = scriptOf(file);
    for (const m of src.matchAll(/\binvoke\b/g)) {
      const command = commandAfter(src, m.index + m[0].length);
      if (command) calls.push({ file: file.path, command });
    }
  }
  return calls;
}
