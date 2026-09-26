/**
 * Extract stage: builds one TypeScript program over `src/` and reads facts straight from the AST +
 * type checker — every wrapped service function, the body facts of every function it can reach,
 * the exported module types, and path-string links. No regex over source text for anything the
 * checker can resolve (imports, re-exports through `@/mocks`, inferred return types).
 */
import path from 'node:path';
import ts from 'typescript';
import { config } from './config';
import type { FnFacts, Param, PathLink, TypeDecl, TypeField } from './types';

type FnNode = ts.FunctionDeclaration | ts.FunctionExpression | ts.ArrowFunction | ts.MethodDeclaration;

export interface Extracted {
  program: ts.Program;
  checker: ts.TypeChecker;
  /** Facts per function id, filled lazily as bodies are reached. */
  facts: Map<string, FnFacts>;
  /** Wrapped service functions (raw — the analyze stage adds closures and dispositions). */
  wrapped: { source: string; file: string; line: number; name: string; params: Param[]; returns: string; fnId: string; calls: string[] }[];
  unwrapped: { module: string; file: string; name: string }[];
  types: TypeDecl[];
  pathLinks: PathLink[];
  tables: string[];
  /** Function id → wrap() source, so service→service calls can be named. */
  wrappedByFnId: Map<string, string>;
}

const rel = (file: string) => path.relative(config.root, file).split(path.sep).join('/');
const inSrc = (file: string) => rel(file).startsWith('src/');
const lineOf = (node: ts.Node) => node.getSourceFile().getLineAndCharacterOfPosition(node.getStart()).line + 1;

function createProgram(): ts.Program {
  const cfgPath = path.join(config.root, config.tsconfig);
  const raw = ts.readConfigFile(cfgPath, ts.sys.readFile);
  const parsed = ts.parseJsonConfigFileContent(raw.config, ts.sys, config.root);
  return ts.createProgram(parsed.fileNames, { ...parsed.options, noEmit: true });
}

/** The function node + display name a declaration stands for (declaration, `const f = …`, or `wrap('…', function f…)`). */
function fnOfDeclaration(decl: ts.Declaration): { node: FnNode; name: string } | undefined {
  if (ts.isFunctionDeclaration(decl) && decl.body && decl.name) return { node: decl, name: decl.name.text };
  if (ts.isVariableDeclaration(decl) && decl.initializer && ts.isIdentifier(decl.name)) {
    let init: ts.Expression = decl.initializer;
    if (ts.isCallExpression(init) && ts.isIdentifier(init.expression) && init.expression.text === config.wrapFn) init = init.arguments[1];
    if (init && (ts.isFunctionExpression(init) || ts.isArrowFunction(init))) return { node: init, name: decl.name.text };
  }
  if ((ts.isFunctionExpression(decl) || ts.isMethodDeclaration(decl)) && decl.body && decl.name && ts.isIdentifier(decl.name)) return { node: decl, name: decl.name.text };
  return undefined;
}

export function extract(): Extracted {
  const program = createProgram();
  const checker = program.getTypeChecker();
  const facts = new Map<string, FnFacts>();
  const fnIdOfNode = new Map<ts.Node, string>();

  const resolve = (node: ts.Node): ts.Declaration | undefined => {
    let sym = checker.getSymbolAtLocation(node);
    if (sym && sym.flags & ts.SymbolFlags.Alias) sym = checker.getAliasedSymbol(sym);
    return sym?.declarations?.[0];
  };
  const isMockDb = (id: ts.Identifier) => {
    const decl = resolve(id);
    return !!decl && rel(decl.getSourceFile().fileName) === config.mockDbFile && id.text === 'db';
  };
  /**
   * `db.<table>` at the root of a property/element access chain, if any. Only follows calls that
   * hand back a reference into the table (`find`, `at`…) — `db.x.filter(…).sort()` sorts a copy.
   */
  const tableOf = (expr: ts.Expression): { table: string; copy: boolean } | undefined => {
    let e: ts.Expression = expr;
    let copy = false;
    while (ts.isPropertyAccessExpression(e) || ts.isElementAccessExpression(e) || ts.isCallExpression(e) || ts.isNonNullExpression(e) || ts.isParenthesizedExpression(e)) {
      if (ts.isPropertyAccessExpression(e) && ts.isIdentifier(e.expression) && isMockDb(e.expression)) return { table: e.name.text, copy };
      if (ts.isCallExpression(e)) {
        const method = ts.isPropertyAccessExpression(e.expression) ? e.expression.name.text : '';
        if (!config.referenceMethods.includes(method)) copy = true;
        if (!ts.isPropertyAccessExpression(e.expression)) return undefined;
      }
      e = e.expression;
    }
    return undefined;
  };
  const rootIdent = (expr: ts.Expression): ts.Identifier | undefined => {
    let e: ts.Expression = expr;
    while (ts.isPropertyAccessExpression(e) || ts.isElementAccessExpression(e) || ts.isNonNullExpression(e) || ts.isParenthesizedExpression(e)) e = e.expression;
    return ts.isIdentifier(e) ? e : undefined;
  };

  const nodeById = new Map<string, { node: FnNode; name: string }>();
  function idFor(node: FnNode, name: string): string {
    const known = fnIdOfNode.get(node);
    if (known) return known;
    const id = `${rel(node.getSourceFile().fileName)}#${name}`;
    fnIdOfNode.set(node, id);
    nodeById.set(id, { node, name });
    return id;
  }

  /** Body facts for one function; follows nothing (the analyze stage walks `calls`). */
  function factsFor(node: FnNode, name: string): FnFacts {
    const id = idFor(node, name);
    const cached = facts.get(id);
    if (cached) return cached;
    const f: FnFacts = { id, file: rel(node.getSourceFile().fileName), name, line: lineOf(node), tables: [], writes: [], calls: [], mockRefs: [], invokes: [], platform: [], round2: 0, round4: 0 };
    facts.set(id, f);
    const tables = new Set<string>(), writes = new Set<string>(), calls = new Set<string>(), mockRefs = new Set<string>(), invokes = new Set<string>(), platform = new Set<string>();
    const aliases = new Map<string, { table: string; copy: boolean }>(); // local var → table it was read from
    /** `onRoot`: a mutating method called on `target` itself (so a copied array doesn't count). */
    const markWrite = (target: ts.Expression, onRoot = false) => {
      const t = tableOf(target);
      if (t) {
        if (!(t.copy && onRoot)) writes.add(t.table);
        return;
      }
      const root = rootIdent(target);
      const alias = root && aliases.get(root.text);
      if (alias && !(alias.copy && onRoot && ts.isIdentifier(target))) writes.add(alias.table);
    };

    const visit = (n: ts.Node, inWriteScope: boolean): void => {
      if (ts.isPropertyAccessExpression(n) && ts.isIdentifier(n.expression) && isMockDb(n.expression)) {
        tables.add(n.name.text);
        if (inWriteScope) writes.add(n.name.text);
      }
      if (ts.isVariableDeclaration(n) && n.initializer && ts.isIdentifier(n.name)) {
        const t = tableOf(n.initializer);
        if (t) aliases.set(n.name.text, t);
      }
      if (ts.isForOfStatement(n) && ts.isVariableDeclarationList(n.initializer)) {
        const t = tableOf(n.expression);
        const d = n.initializer.declarations[0];
        if (t && d && ts.isIdentifier(d.name)) aliases.set(d.name.text, { table: t.table, copy: false }); // loop items are references
      }
      if (ts.isBinaryExpression(n) && n.operatorToken.kind >= ts.SyntaxKind.FirstAssignment && n.operatorToken.kind <= ts.SyntaxKind.LastAssignment) markWrite(n.left);
      if ((ts.isPrefixUnaryExpression(n) || ts.isPostfixUnaryExpression(n)) && (n.operator === ts.SyntaxKind.PlusPlusToken || n.operator === ts.SyntaxKind.MinusMinusToken)) markWrite(n.operand);
      if (ts.isDeleteExpression(n)) markWrite(n.expression);

      if (ts.isCallExpression(n)) {
        const callee = n.expression;
        if (ts.isPropertyAccessExpression(callee) && config.mutatingMethods.includes(callee.name.text)) markWrite(callee.expression, true);
        if (ts.isPropertyAccessExpression(callee) && callee.name.text === 'assign' && ts.isIdentifier(callee.expression) && callee.expression.text === 'Object' && n.arguments[0]) markWrite(n.arguments[0]);
        if (callee.kind === ts.SyntaxKind.ImportKeyword && n.arguments[0] && ts.isStringLiteral(n.arguments[0]) && n.arguments[0].text.startsWith('@tauri-apps/')) platform.add(n.arguments[0].text);
        const target = ts.isIdentifier(callee) ? callee : ts.isPropertyAccessExpression(callee) ? callee.name : undefined;
        if (target) {
          if (target.text === 'round2') f.round2++;
          if (target.text === 'round4') f.round4++;
          const decl = resolve(target);
          const declFile = decl ? rel(decl.getSourceFile().fileName) : '';
          // `invoke` is usually destructured from a dynamic `import('@tauri-apps/api/core')`, so it
          // resolves to a local binding — match it by name + literal command instead.
          if (target.text === 'invoke' && n.arguments[0] && ts.isStringLiteralLike(n.arguments[0])) invokes.add(n.arguments[0].text);
          else if (declFile.includes('node_modules/@tauri-apps/')) platform.add(declFile.split('node_modules/')[1].split('/').slice(0, 2).join('/'));
          const fn = decl && inSrc(decl.getSourceFile().fileName) ? fnOfDeclaration(decl) : undefined;
          if (fn) calls.add(idFor(fn.node, fn.name));
        }
        if (ts.isIdentifier(callee) && config.writeScopes.includes(callee.text)) {
          n.arguments.forEach((a) => visit(a, true));
          return;
        }
      }
      if (ts.isIdentifier(n) && !(ts.isPropertyAccessExpression(n.parent) && n.parent.name === n)) {
        if (config.browserGlobals.includes(n.text) && !resolve(n)?.getSourceFile().fileName.includes('/src/')) platform.add(n.text);
        const decl = resolve(n);
        if (decl && rel(decl.getSourceFile().fileName).startsWith(config.mocksDir + '/') && n.text !== 'db' && !fnOfDeclaration(decl) && ts.isVariableDeclaration(decl)) mockRefs.add(n.text);
      }
      ts.forEachChild(n, (c) => visit(c, inWriteScope));
    };
    if (node.body) visit(node.body, false);
    Object.assign(f, {
      tables: [...tables].sort(), writes: [...writes].sort(), calls: [...calls].sort(), mockRefs: [...mockRefs].sort(),
      invokes: [...invokes].sort(), platform: [...platform].sort(),
    });
    return f;
  }

  /** Fill facts for everything reachable from a function's calls (each body is parsed once). */
  function reach(node: FnNode, name: string) {
    const stack = [{ node, name }];
    while (stack.length) {
      const cur = stack.pop()!;
      for (const c of factsFor(cur.node, cur.name).calls) {
        const target = nodeById.get(c);
        if (target && !facts.has(c)) stack.push(target);
      }
    }
  }

  const wrapped: Extracted['wrapped'] = [];
  const wrappedByFnId = new Map<string, string>();
  const unwrapped: Extracted['unwrapped'] = [];
  const types: TypeDecl[] = [];
  const pathLinks: PathLink[] = [];
  let tables: string[] = [];

  for (const sf of program.getSourceFiles()) {
    const file = rel(sf.fileName);
    if (!file.startsWith('src/')) continue;
    const parts = file.split('/');
    const moduleName = file.startsWith(config.modulesDir + '/') ? parts[2] : '';

    if (file === config.mockDbFile) {
      sf.forEachChild((n) => {
        if (ts.isInterfaceDeclaration(n) && n.name.text === config.mockDbInterface)
          tables = n.members.filter(ts.isPropertySignature).map((m) => (m.name as ts.Identifier).text).sort();
      });
    }

    // Every function in the mock backend gets facts even if no service reaches it (the mock table).
    if (file.startsWith(config.mockBackendDir + '/')) {
      sf.forEachChild((n) => {
        if (ts.isFunctionDeclaration(n) && n.body && n.name) reach(n, n.name.text);
      });
    }

    const isService = moduleName && parts[3] === 'services';
    if (isService) {
      sf.forEachChild((n) => {
        if (!ts.isVariableStatement(n) && !ts.isFunctionDeclaration(n) && !ts.isClassDeclaration(n)) return;
        const exported = ts.getCombinedModifierFlags(n as ts.Declaration) & ts.ModifierFlags.Export;
        if (!exported) return;
        const decls = ts.isVariableStatement(n) ? n.declarationList.declarations : [n as ts.FunctionDeclaration | ts.ClassDeclaration];
        for (const d of decls) {
          const init = ts.isVariableDeclaration(d) ? d.initializer : undefined;
          const isWrap = init && ts.isCallExpression(init) && ts.isIdentifier(init.expression) && init.expression.text === config.wrapFn && ts.isStringLiteralLike(init.arguments[0]);
          const name = d.name && ts.isIdentifier(d.name) ? d.name.text : '?';
          if (!isWrap) { unwrapped.push({ module: moduleName, file, name }); continue; }
          const call = init as ts.CallExpression;
          const fnNode = call.arguments[1] as ts.FunctionExpression | ts.ArrowFunction;
          const source = (call.arguments[0] as ts.StringLiteral).text;
          const fnId = idFor(fnNode, name);
          reach(fnNode, name);
          wrappedByFnId.set(fnId, source);
          const sig = checker.getSignatureFromDeclaration(fnNode);
          const flags = ts.TypeFormatFlags.NoTruncation | ts.TypeFormatFlags.UseAliasDefinitionsOutsideCurrentScope;
          const params: Param[] = fnNode.parameters.map((p) => ({
            name: p.name.getText(),
            type: checker.typeToString(checker.getTypeAtLocation(p), p, flags),
            optional: !!p.questionToken || !!p.initializer,
          }));
          const returns = sig ? checker.typeToString(sig.getReturnType(), fnNode, flags) : 'unknown';
          wrapped.push({ source, file, line: lineOf(d), name, params, returns, fnId, calls: facts.get(fnId)!.calls });
        }
      });
    }

    if (moduleName && parts[3] === 'types') collectTypes(sf, moduleName, file, types);
    if (file.startsWith(config.mocksDir + '/') || isService) collectPathLinks(sf, file, pathLinks);
  }

  return { program, checker, facts, wrapped, unwrapped, types, pathLinks, tables, wrappedByFnId };
}

function hintFor(name: string, type: string): string | undefined {
  const t = type.replace(/\s|\|undefined|\|null/g, '');
  if (config.hints.route.test(name) || /AppRoute/.test(type)) return 'route';
  if (t === 'number' && config.hints.decimal.test(name)) return 'decimal';
  if (t === 'string' && (name === 'id' || /Id$/.test(name))) return 'uuid';
  if (t === 'string[]' && /Ids$/.test(name)) return 'uuid[]';
  if (t === 'string' && config.hints.date.test(name)) return 'date';
  if (/^('[^']*'\|?)+$/.test(t)) return 'enum';
  return undefined;
}

function fieldsOf(members: ts.NodeArray<ts.TypeElement>): TypeField[] {
  return members.filter(ts.isPropertySignature).map((m) => {
    const name = m.name.getText().replace(/['"]/g, '');
    const type = m.type ? m.type.getText().replace(/\s+/g, ' ') : 'unknown';
    return { name, type, optional: !!m.questionToken, hint: hintFor(name, type) };
  });
}

function collectTypes(sf: ts.SourceFile, module: string, file: string, out: TypeDecl[]) {
  sf.forEachChild((n) => {
    const exported = (ts.isInterfaceDeclaration(n) || ts.isTypeAliasDeclaration(n)) && ts.getCombinedModifierFlags(n) & ts.ModifierFlags.Export;
    if (!exported) return;
    if (ts.isInterfaceDeclaration(n)) out.push({ module, file, line: lineOf(n), name: n.name.text, kind: 'interface', fields: fieldsOf(n.members) });
    else if (ts.isTypeAliasDeclaration(n))
      out.push(ts.isTypeLiteralNode(n.type)
        ? { module, file, line: lineOf(n), name: n.name.text, kind: 'type', fields: fieldsOf(n.type.members) }
        : { module, file, line: lineOf(n), name: n.name.text, kind: 'type', text: n.type.getText().replace(/\s+/g, ' ') });
  });
}

/** String/template literals that look like app paths (`/invoices/…`) — must become named route objects (rule 25). */
function collectPathLinks(sf: ts.SourceFile, file: string, out: PathLink[]) {
  const visit = (n: ts.Node) => {
    if (ts.isImportDeclaration(n) || ts.isExportDeclaration(n)) return;
    const head = ts.isStringLiteralLike(n) ? n.text : ts.isTemplateExpression(n) ? n.head.text : undefined;
    if (head !== undefined && /^\/[a-z][a-z0-9-]*(\/|$)/.test(head)) {
      if (!ts.isLiteralTypeNode(n.parent)) out.push({ file, line: lineOf(n), text: n.getText().slice(0, 80) });
      return;
    }
    ts.forEachChild(n, visit);
  };
  visit(sf);
}
