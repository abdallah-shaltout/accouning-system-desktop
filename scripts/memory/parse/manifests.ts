/** Parser — package.json, tauri.conf.json, Cargo.toml and docs titles. */
import type { Config } from '../config';
import { readJson, readText } from '../scan/scanner';
import type { DocInfo, Manifests, SourceFile } from '../types';

interface PackageJson {
  name: string;
  scripts?: Record<string, string>;
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
}

/** Dependency names from every `[dependencies]` / `[target.*.dependencies]` table. */
function cargoDeps(toml: string): string[] {
  const deps: string[] = [];
  let inDeps = false;
  for (const line of toml.split('\n')) {
    const header = line.match(/^\s*\[([^\]]+)\]/);
    if (header) {
      inDeps = /(^|\.)dependencies$/.test(header[1]) && !header[1].startsWith('build-');
      continue;
    }
    const dep = inDeps && line.match(/^\s*([\w-]+)\s*=/);
    if (dep) deps.push(dep[1]);
  }
  return deps;
}

export function parseManifests(config: Config): Manifests {
  const pkg = readJson<PackageJson>(config, config.paths.packageJson);
  const tauri = readJson<{ productName?: string; identifier?: string; version?: string }>(config, config.paths.tauriConf);
  const toml = readText(config, config.paths.cargoToml);
  return {
    name: pkg?.name ?? '',
    scripts: pkg?.scripts ?? {},
    deps: Object.keys(pkg?.dependencies ?? {}),
    devDeps: Object.keys(pkg?.devDependencies ?? {}),
    tauri: { productName: tauri?.productName, identifier: tauri?.identifier, version: tauri?.version },
    cargoDeps: cargoDeps(toml),
    cargoBins: [...toml.matchAll(/\[\[bin\]\]\s*\n\s*name\s*=\s*"([^"]+)"/g)].map((m) => m[1]),
  };
}

export function parseDocs(files: SourceFile[]): DocInfo[] {
  return files
    .filter((f) => f.lang === 'md')
    .map((f) => ({ path: f.path, title: f.content.match(/^#\s+(.+)$/m)?.[1].trim() ?? '' }));
}
