/** Stage 3 — derive architecture facts from parsed data. */
import type { Config } from '../config';
import { exists } from '../scan/scanner';
import type { Analysis, ParsedRepo } from '../types';
import { analyzeBoundaries } from './boundaries';
import { analyzeMocks, analyzeModuleEdges, analyzePackages } from './dependencies';
import { analyzeKit } from './kit';
import { analyzeModules } from './modules';

export function analyze(repo: ParsedRepo, config: Config): Analysis {
  const byLang: Record<string, number> = {};
  for (const f of repo.files) byLang[f.lang] = (byLang[f.lang] ?? 0) + 1;
  return {
    modules: analyzeModules(repo, config),
    moduleEdges: analyzeModuleEdges(repo, config),
    packages: analyzePackages(repo),
    mocks: analyzeMocks(repo, config),
    kit: analyzeKit(repo, config),
    boundaries: analyzeBoundaries(repo, config),
    landmarks: config.landmarks.map(([label, path]) => ({ label, path, exists: exists(config, path) })),
    totals: {
      files: repo.files.length,
      lines: repo.files.reduce((n, f) => n + f.lines, 0),
      byLang,
    },
  };
}
