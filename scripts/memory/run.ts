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
import { scan } from './scan/scanner';

const started = performance.now();
const files = scan(config);
const repo = parse(files, config);
const analysis = analyze(repo, config);
const markdown = render(repo, analysis, config);

const outPath = path.join(config.root, config.output);
const current = fs.existsSync(outPath) ? fs.readFileSync(outPath, 'utf8').replace(/\r\n/g, '\n') : '';
const ms = Math.round(performance.now() - started);

if (process.argv.includes('--check')) {
  if (current !== markdown) {
    console.error(`${config.output} is stale — run \`bun run memory\`.`);
    process.exit(1);
  }
  console.log(`${config.output} is up to date (${ms} ms).`);
} else {
  if (current !== markdown) fs.writeFileSync(outPath, markdown);
  const newSeam = analysis.boundaries.seam.filter((v) => !v.known).length;
  console.log(
    `${config.output}: ${files.length} files, ${analysis.modules.length} modules, ${repo.routes.length} routes, ` +
      `${repo.rust.commands.length} Rust commands, ${newSeam} new seam violations — ${current === markdown ? 'unchanged' : 'written'} (${ms} ms).`,
  );
}
