/**
 * Agent-memory indexer: Scanner → Parsers → Analyzers → Markdown renderer → AGENT_MEMORY.md.
 *
 *   bun run memory           # regenerate AGENT_MEMORY.md
 *   bun run memory:check     # exit 1 if AGENT_MEMORY.md is out of date (CI / pre-commit)
 *
 * Tune ignores, anchors and rules in ./config.ts — stages hold no configuration.
 */
import fs from 'node:fs';
import path from 'node:path';
import { analyze } from './analyze';
import { config } from './config';
import { parse } from './parse';
import { render } from './render/markdown';
import { renderRouteMap } from './render/routeMap';
import { scan } from './scan/scanner';

const started = performance.now();
const files = scan(config);
const repo = parse(files, config);
const analysis = analyze(repo, config);
const markdown = render(repo, analysis, config);
const routeMap = renderRouteMap(repo.routes);

function readCurrent(relPath: string): string {
  const p = path.join(config.root, relPath);
  return fs.existsSync(p) ? fs.readFileSync(p, 'utf8').replace(/\r\n/g, '\n') : '';
}

const currentMarkdown = readCurrent(config.output);
const currentRouteMap = readCurrent(config.routeMapOutput);
const ms = Math.round(performance.now() - started);

if (process.argv.includes('--check')) {
  const stale: string[] = [];
  if (currentMarkdown !== markdown) stale.push(config.output);
  if (currentRouteMap !== routeMap) stale.push(config.routeMapOutput);
  if (stale.length > 0) {
    console.error(`${stale.join(', ')} ${stale.length > 1 ? 'are' : 'is'} stale — run \`bun run memory\`.`);
    process.exit(1);
  }
  console.log(`${config.output} and ${config.routeMapOutput} are up to date (${ms} ms).`);
} else {
  if (currentMarkdown !== markdown) fs.writeFileSync(path.join(config.root, config.output), markdown);
  if (currentRouteMap !== routeMap) fs.writeFileSync(path.join(config.root, config.routeMapOutput), routeMap);
  const newSeam = analysis.boundaries.seam.filter((v) => !v.known).length;
  const written = [currentMarkdown !== markdown && config.output, currentRouteMap !== routeMap && config.routeMapOutput]
    .filter(Boolean)
    .join(', ');
  console.log(
    `${config.output}: ${files.length} files, ${analysis.modules.length} modules, ${repo.routes.length} routes, ` +
      `${repo.rust.commands.length} Rust commands, ${newSeam} new seam violations — ${written ? `written: ${written}` : 'unchanged'} (${ms} ms).`,
  );
}
