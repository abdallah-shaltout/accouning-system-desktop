/** Data contracts passed between pipeline stages: scan → parse → analyze → render. */

export type Lang = 'ts' | 'vue' | 'rust' | 'md' | 'json';

/** Stage 1 output: one scanned file. `path` is repo-relative, POSIX. */
export interface SourceFile {
  path: string;
  lang: Lang;
  lines: number;
  content: string;
}

export interface ImportEdge {
  from: string;
  spec: string;
  /** Resolved repo-relative file, or null for packages / assets. */
  target: string | null;
  /** npm package name when the specifier is bare. */
  pkg: string | null;
  typeOnly: boolean;
  dynamic: boolean;
}

export interface ExportSymbol {
  name: string;
  kind: 'function' | 'const' | 'class' | 'type' | 'default';
}

export interface RouteInfo {
  file: string;
  path: string;
  name?: string;
  title?: string;
  area?: string;
  component?: string;
}

export interface RustCommand {
  name: string;
  /** Module path as used in `generate_handler!`, e.g. `pdf::render::render_pdf`. */
  qualified: string;
  file: string;
}

export interface RustInfo {
  mods: { file: string; name: string; public: boolean }[];
  commands: RustCommand[];
  /** Entries of `generate_handler![...]`, verbatim. */
  registered: string[];
  plugins: string[];
}

export interface IpcCall {
  file: string;
  command: string;
}

export interface DocInfo {
  path: string;
  title: string;
}

export interface Manifests {
  name: string;
  scripts: Record<string, string>;
  deps: string[];
  devDeps: string[];
  tauri: { productName?: string; identifier?: string; version?: string };
  cargoDeps: string[];
  cargoBins: string[];
}

/** Stage 2 output. */
export interface ParsedRepo {
  files: SourceFile[];
  imports: ImportEdge[];
  exports: Map<string, ExportSymbol[]>;
  routes: RouteInfo[];
  rust: RustInfo;
  ipc: IpcCall[];
  docs: DocInfo[];
  manifests: Manifests;
}

/** Where a file sits in the architecture. */
export interface Location {
  area: 'module' | 'mocks' | 'app' | 'rust' | 'docs';
  module?: string;
  layer?: string;
}

export interface ModuleSummary {
  name: string;
  files: number;
  lines: number;
  layers: Record<string, number>;
  services: { file: string; api: string[] }[];
  routes: RouteInfo[];
  hasPaletteCommands: boolean;
}

export interface ModuleEdge {
  from: string;
  to: string;
  count: number;
}

export interface SeamViolation {
  file: string;
  targets: string[];
  known: boolean;
}

export interface Analysis {
  modules: ModuleSummary[];
  moduleEdges: ModuleEdge[];
  packages: { pkg: string; files: number }[];
  mocks: { file: string; api: string[]; importers: string[] }[];
  kit: Record<string, string[]>;
  boundaries: {
    seam: SeamViolation[];
    ipc: {
      registeredNotDefined: string[];
      definedNotRegistered: string[];
      registeredNotInvoked: string[];
      invokedNotRegistered: string[];
    };
    oversizedPages: { file: string; lines: number }[];
    crossModulePageImports: { from: string; target: string }[];
  };
  landmarks: { label: string; path: string; exists: boolean }[];
  totals: { files: number; lines: number; byLang: Record<string, number> };
}
