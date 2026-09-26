/**
 * Backend-contract inventory: Extract (TS program + checker) → Analyze → Render → docs/backend/contract/.
 *
 *   bun run contract           # regenerate docs/backend/contract/*
 *   bun run contract:check     # exit 1 if any generated file is out of date
 *
 * Part 01 of plans/pending/21-rust-backend. Tune everything in ./config.ts — stages hold no configuration.
 */
import fs from 'node:fs';
import path from 'node:path';
import { analyze } from './analyze';
import { config } from './config';
import { extract } from './extract';
import { render } from './render';

const started = performance.now();
const inventory = analyze(extract());
const files = render(inventory);

const read = (rel: string) => {
  const p = path.join(config.root, rel);
  return fs.existsSync(p) ? fs.readFileSync(p, 'utf8').replace(/\r\n/g, '\n') : undefined;
};
const outDir = path.join(config.root, config.outDir);
const existing = fs.existsSync(outDir) ? fs.readdirSync(outDir).map((f) => `${config.outDir}/${f}`) : [];
const stale = Object.entries(files).filter(([rel, content]) => read(rel) !== content).map(([rel]) => rel);
const orphans = existing.filter((rel) => !(rel in files));
const ms = Math.round(performance.now() - started);
const counts = `${inventory.services.length} service fns (${inventory.services.filter((s) => s.disposition === 'port').length} port), ${inventory.tables.length} tables, ${inventory.types.length} types`;

if (process.argv.includes('--check')) {
  if (stale.length || orphans.length) {
    console.error(`backend contract is stale (${[...stale, ...orphans].join(', ')}) — run \`bun run contract\`.`);
    process.exit(1);
  }
  console.log(`backend contract is up to date: ${counts} (${ms} ms).`);
} else {
  fs.mkdirSync(outDir, { recursive: true });
  for (const rel of stale) fs.writeFileSync(path.join(config.root, rel), files[rel]);
  for (const rel of orphans) fs.rmSync(path.join(config.root, rel));
  console.log(`backend contract: ${counts} — ${stale.length + orphans.length ? `${stale.length} written, ${orphans.length} removed` : 'unchanged'} (${ms} ms).`);
}
