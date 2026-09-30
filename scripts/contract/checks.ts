/**
 * `bun run contract:check` rule stage (21.04 phase A, A-2 + A-4): everything a plain staleness diff
 * of the generated docs cannot see. Two rules, both over the already-built `Inventory`:
 *
 *   A-2 — switch-line coverage. Every `port` function must reach at least one `backendCall(...)`
 *   that sits inside a `usesRust('<domain>')` guard (directly, through a ternary, or through the
 *   `if (!usesRust(...)) return/throw;` guard-clause idiom — see `extract.ts`'s `visit`), and every
 *   such site's domain must match the command's own prefix, unless the function or the (prefix,
 *   domain) pair is on a reasoned exception list. A domain-less `backendCall` (no enclosing guard at
 *   all) must be named on `config.ungatedSwitchSites` with a reason.
 *
 *   A-4 — mock-read rule for stay-frontend functions. A `frontend`/`dev-only`-disposition function
 *   whose closure still reads a MockDb table or a `src/mocks/backend/**` function is only allowed
 *   when `config.overrides` already carries a reasoned entry for it (the override's own reason IS the
 *   allowlist reason — CLAUDE.md "every tolerated difference traces back to a written decision").
 *
 * Both rules produce `{ rule, source, detail }` violations; `run.ts --check` prints them and exits 1.
 */
import { config } from './config';
import type { Inventory, ServiceFn } from './types';

export interface CheckViolation {
  rule: 'A-2' | 'A-4';
  source: string;
  detail: string;
}

function checkSwitchLineCoverage(services: ServiceFn[]): CheckViolation[] {
  const violations: CheckViolation[] = [];
  for (const s of services) {
    if (s.disposition !== 'port') continue;
    const sites = s.switchSites ?? [];
    if (sites.length === 0) {
      // A `port`-disposition function with a reasoned override (config.overrides) is allowed to have
      // no switch line of its own when the override's own reason explains why — e.g.
      // core.getThresholds/setThresholds read through useSettingsStore(), whose load()/update() call
      // the already-gated settings.getSettings/updateSettings switch lines (A-5's loader pattern).
      if (Object.prototype.hasOwnProperty.call(config.overrides, s.source)) continue;
      violations.push({
        rule: 'A-2',
        source: s.source,
        detail: 'disposition is `port` but no backendCall was found (directly or through a local helper) inside a usesRust(...) guard — add the switch line, or add a config.overrides entry if this function is not actually Rust-backed',
      });
      continue;
    }
    for (const site of sites) {
      if (!site.guardDomain) {
        const reason = config.ungatedSwitchSites[site.command];
        if (!reason) {
          violations.push({
            rule: 'A-2',
            source: s.source,
            detail: `backendCall('${site.command}', …) at line ${site.line} has no enclosing usesRust(...) guard and is not on config.ungatedSwitchSites — it would run on Rust unconditionally, even in browser/mock mode`,
          });
        }
        continue;
      }
      const prefix = site.command.split('_')[0];
      const expectedDomain = config.switchDomainPrefixExceptions[prefix] ?? prefix;
      if (expectedDomain !== site.guardDomain) {
        violations.push({
          rule: 'A-2',
          source: s.source,
          detail: `backendCall('${site.command}', …) at line ${site.line} is guarded by usesRust('${site.guardDomain}'), but its command prefix ('${prefix}') expects domain '${expectedDomain}' — add a config.switchDomainPrefixExceptions entry if this is deliberate (e.g. attachments living under 'core'), otherwise fix the guard`,
        });
      }
    }
  }
  return violations;
}

function checkStayFrontendMockReads(services: ServiceFn[]): CheckViolation[] {
  const violations: CheckViolation[] = [];
  for (const s of services) {
    if (s.disposition !== 'frontend' && s.disposition !== 'dev-only') continue;
    const readsMock = s.closure.tables.length > 0 || s.closure.mockFns.length > 0;
    if (!readsMock) continue;
    const hasOverride = Object.prototype.hasOwnProperty.call(config.overrides, s.source);
    const hasAllowlistReason = Object.prototype.hasOwnProperty.call(config.stayFrontendMockReadAllowlist, s.source);
    if (!hasOverride && !hasAllowlistReason) {
      violations.push({
        rule: 'A-4',
        source: s.source,
        detail: `disposition '${s.disposition}' still reads mock business data after the flip (tables: ${s.closure.tables.join(', ') || 'none'}; mock fns: ${s.closure.mockFns.length}) with no reasoned config.overrides or config.stayFrontendMockReadAllowlist entry — it would show stale IndexedDB data next to MariaDB data (P4-1)`,
      });
    }
  }
  return violations;
}

export function runChecks(inv: Inventory): CheckViolation[] {
  return [...checkSwitchLineCoverage(inv.services), ...checkStayFrontendMockReads(inv.services)];
}
