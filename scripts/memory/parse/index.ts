/** Stage 2 — turn scanned files into structured facts. */
import type { Config } from '../config';
import type { ParsedRepo, SourceFile } from '../types';
import { parseIpc } from './ipc';
import { parseDocs, parseManifests } from './manifests';
import { parseRoutes } from './routes';
import { parseRust } from './rust';
import { parseExports, parseImports } from './tsImports';

export function parse(files: SourceFile[], config: Config): ParsedRepo {
  return {
    files,
    imports: parseImports(files, config),
    exports: parseExports(files),
    routes: parseRoutes(files),
    rust: parseRust(files, config),
    ipc: parseIpc(files),
    docs: parseDocs(files),
    manifests: parseManifests(config),
  };
}
