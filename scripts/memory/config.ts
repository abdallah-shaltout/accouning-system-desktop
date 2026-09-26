/**
 * Single source of configuration for the agent-memory indexer (`bun run memory`).
 * Every path is repo-relative and POSIX-style. Change behaviour here, never inside a stage.
 */
import path from 'node:path';
import type { Lang } from './types';

export interface Config {
  root: string;
  output: string;
  routeMapOutput: string;
  scanRoots: string[];
  extensions: Record<string, Lang>;
  ignoreDirs: string[];
  ignorePrefixes: string[];
  aliases: Record<string, string>;
  resolveSuffixes: string[];
  paths: Record<'modules' | 'mocks' | 'rust' | 'rustEntry' | 'packageJson' | 'tauriConf' | 'cargoToml' | 'coreKit' | 'docs' | 'plans' | 'diagnosticsIssues', string>;
  seam: { allowedLayers: string[]; forbiddenTarget: string; compositionRoots: string[]; knownLegacy: string[] };
  maxPageLines: number;
  landmarks: [label: string, path: string][];
}

export const config: Config = {
  /** Repo root — this file lives in scripts/memory/. */
  root: path.resolve(import.meta.dirname, '../..'),
  /** Generated file (repo-relative). */
  output: 'AGENT_MEMORY.md',
  /** Generated typed route map (repo-relative). */
  routeMapOutput: 'src/router/route-map.gen.d.ts',

  /** Directories walked by the scanner. */
  scanRoots: ['src', 'src-tauri/src', 'docs', 'plans'],
  /** Extensions read and parsed; everything else is ignored. */
  extensions: { ts: 'ts', vue: 'vue', rs: 'rust', md: 'md', json: 'json' },
  /** Directory names skipped anywhere in the tree (heavy, generated or vendored). */
  ignoreDirs: [
    'node_modules', 'dist', 'dist-ssr', 'build', 'target', 'gen', '.git', '.claude', '.vscode',
    '__pycache__', 'shots', 'references', 'logo', 'icons', 'fixtures',
  ],
  /** Path prefixes skipped (repo-relative). */
  ignorePrefixes: ['src-tauri/Microsoft.WebView2', 'scripts/shots_', '--help'],

  /** Import aliases (mirror tsconfig `paths` / vite `resolve.alias`). */
  aliases: { '@/': 'src/' },
  /** Resolution order for extension-less specifiers. */
  resolveSuffixes: ['', '.ts', '.vue', '.d.ts', '/index.ts'],

  /** Architecture anchors. */
  paths: {
    modules: 'src/modules',
    mocks: 'src/mocks',
    rust: 'src-tauri/src',
    rustEntry: 'src-tauri/src/lib.rs',
    packageJson: 'package.json',
    tauriConf: 'src-tauri/tauri.conf.json',
    cargoToml: 'src-tauri/Cargo.toml',
    coreKit: 'src/modules/core/components',
    docs: 'docs',
    plans: 'plans',
    /** 18.G "Open diagnostics" section — the same ledger `bun run diag` builds ISSUES.md from. */
    diagnosticsIssues: 'docs/diagnostics/issues',
  },

  /** Seam rule (CLAUDE.md): only these module layers may import the mock backend. */
  seam: {
    allowedLayers: ['services'],
    forbiddenTarget: 'src/mocks/',
    /** Composition roots may wire the mock backend in (boot it); they are not screens. */
    compositionRoots: ['src/main.ts'],
    /** Legacy violations already acknowledged in CLAUDE.md (file stems). Anything else is "new". */
    knownLegacy: [
      'WelcomePage', 'SetupWizardPage', 'DevMenu', 'VoucherDetailPage', 'ExpenseDetailPage',
      'AccountantHome', 'StorekeeperHome', 'PurchaseFormPage', 'PartyFormPage', 'PosPage',
      'AttachmentField', 'AttachmentViewer', 'ProductImageGallery',
    ],
  },

  /** CLAUDE.md rule 12. */
  maxPageLines: 250,

  /** "Where is X?" quick index. Missing paths are reported so the list never rots silently. */
  landmarks: [
    ['App bootstrap', 'src/main.ts'],
    ['Router (module route aggregation, guards)', 'src/router/index.ts'],
    ['Route meta typing', 'src/router/route-meta.d.ts'],
    ['Typed route names (generated)', 'src/router/route-map.gen.d.ts'],
    ['Sidebar navigation groups', 'src/modules/core/helpers/navigation.ts'],
    ['Brand / product name', 'src/modules/core/helpers/brand.ts'],
    ['Formatting (money, dates, numbers)', 'src/modules/core/helpers/format.ts'],
    ['Design tokens', 'src/assets/styles/design-system.css'],
    ['Theme / appearance controllers', 'src/modules/core/controllers/useAppearance.ts'],
    ['Native save dialog helper', 'src/modules/core/services/saveFile.ts'],
    ['PDF service (→ Rust render_pdf)', 'src/modules/core/services/pdfService.ts'],
    ['Print service (→ Rust thermal print)', 'src/modules/core/services/printService.ts'],
    ['Command palette', 'src/modules/core/commandPalette'],
    ['Dev UI gallery', 'src/modules/core/pages/DevUiPage.vue'],
    ['Mock DB + persistence', 'src/mocks/db.ts'],
    ['Mock posting engine', 'src/mocks/backend'],
    ['Mock seed data', 'src/mocks/seed'],
    ['Rust command registry', 'src-tauri/src/lib.rs'],
    ['Typst templates', 'src-tauri/templates'],
    ['Accounting invariants (verify:mocks)', 'scripts/verify/run.ts'],
    ['e2e runner', 'scripts/e2e/run.py'],
    ['UI guard scripts', 'scripts/check-rtl.js'],
    ['Design system doc', 'docs/design_system.md'],
    ['Posting rules', 'docs/v2/02-accounting-review.md'],
    ['Rounding rule (round2/round4, half away from zero)', 'src/modules/core/helpers/numbers.ts'],
    ['Backend contract inventory (generated, `bun run contract`)', 'docs/backend/contract/README.md'],
  ],
};
