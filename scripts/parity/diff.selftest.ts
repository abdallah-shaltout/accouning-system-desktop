/**
 * `bun scripts/parity/diff.selftest.ts` (plan 21 Part 04, B-13): synthetic checks of the diff and
 * allowlist rules (P4-6). Exit code 1 on the first wrong answer.
 */
import { applyAllow, citesDecision, validateAllow } from './allow';
import { diffTrees, idsWithoutCreatedAt, IdMap, IMPORTED_CREATED_AT_REASON, normalize, receiptSourceId, type Diff } from './diff';

const U1 = '01900000-0000-7000-8000-000000000001';
const U2 = '01900000-0000-7000-8000-000000000002';
const U3 = '01900000-0000-7000-8000-000000000003';

let failures = 0;
function check(name: string, cond: boolean, detail?: unknown): void {
  console.log(`${cond ? 'OK  ' : 'FAIL'}  ${name}`);
  if (!cond) {
    failures++;
    if (detail !== undefined) console.log(`      ${JSON.stringify(detail)}`);
  }
}
const kinds = (ds: Diff[]) => ds.map((d) => `${d.kind}@${d.path}`);

// Ids: seeded pairs, learning, and a conflicting pair.
{
  const ids = new IdMap([['cust-1', U1]]);
  const ds = diffTrees({ a: 'cust-1', b: 'inv-9', c: 'inv-9' }, { a: U1, b: U2, c: U2 }, ids);
  check('ids: seeded pair and learned pair match', ds.length === 0 && ids.m2r.get('inv-9') === U2, kinds(ds));
  const conflict = diffTrees({ x: 'cust-1', y: 'inv-10' }, { x: U2, y: U1 }, ids);
  check('ids: seeded id facing another UUID is a conflict', conflict.some((d) => d.kind === 'id-conflict' && d.path === 'x'), kinds(conflict));
  check('ids: a UUID already paired with another mock id is a conflict', conflict.some((d) => d.kind === 'id-conflict' && d.path === 'y'), kinds(conflict));
  const fresh = diffTrees({ z: 'pay-3' }, { z: U3 }, ids);
  check('ids: unseen pair is learned, no diff', fresh.length === 0 && ids.r2m.get(U3) === 'pay-3', kinds(fresh));
  check('ids: a plain string vs a different plain string is a value diff', kinds(diffTrees({ n: 'نقداً' }, { n: 'نقدا' }, new IdMap())).join() === 'value@n');
}

// Derived ids (P4-13 b): `<parent>-l<n>` and `<transfer>-recv` translate through the parent's pair.
{
  const ids = new IdMap([['inv-5', U1], ['inv-5-l1', U3]]);
  const ok = diffTrees({ id: 'inv-5', lines: [{ id: 'inv-5-l1' }, { id: 'inv-5-l2' }] }, { id: U1, lines: [{ id: `${U1}-l1` }, { id: `${U1}-l2` }] }, ids);
  check('derived: <parent>-lN pairs through the parent (the direct line-row pair is not required)', ok.length === 0, kinds(ok));
  const direct = diffTrees({ x: 'inv-5-l1' }, { x: U3 }, ids);
  check('derived: a direct importer pair still matches', direct.length === 0, kinds(direct));
  const bad = diffTrees({ x: 'inv-5-l1' }, { x: `${U2}-l1` }, ids);
  check('derived: a line under another parent is an id-conflict', kinds(bad).join() === 'id-conflict@x', kinds(bad));
  const suffix = diffTrees({ x: 'inv-5-l1' }, { x: `${U1}-l2` }, ids);
  check('derived: a different line number is a diff', suffix.length === 1, kinds(suffix));
  const learnt = new IdMap();
  const fresh = diffTrees({ lines: [{ id: 'inv-9-l1' }] }, { lines: [{ id: `${U2}-l1` }] }, learnt);
  check('derived: an unseen parent is learned from its line id', fresh.length === 0 && learnt.m2r.get('inv-9') === U2, kinds(fresh));
  const keyed = diffTrees({ returnedQty: { 'inv-5-l2': 1 } }, { returnedQty: { [`${U1}-l2`]: 1 } }, ids);
  check('derived: a <parent>-lN object key pairs through the parent', keyed.length === 0, kinds(keyed));
  const trf = new IdMap([['trf-1', U1]]);
  check('derived: <transfer>-recv = receipt_source_id(transfer)', diffTrees({ s: 'trf-1-recv' }, { s: receiptSourceId(U1) }, trf).length === 0);
  check('derived: receipt_source_id sets the version nibble to 8 and byte 8 to 0xc0..=0xdf', receiptSourceId(U1) === '01900000-0000-8000-c000-000000000001');
  check('derived: a -recv under another transfer is an id-conflict', kinds(diffTrees({ s: 'trf-1-recv' }, { s: receiptSourceId(U2) }, trf)).join() === 'id-conflict@s');
  const comp = new IdMap([['inv-5', U1]]);
  check('composite: <invoice>:<invoice>-lN pairs part by part', diffTrees({ id: 'inv-5:inv-5-l1' }, { id: `${U1}:${U1}-l1` }, comp).length === 0);
  check('composite: literal:id pairs through the id map', diffTrees({ k: 'year-end:inv-5' }, { k: `year-end:${U1}` }, comp).length === 0);
  check('composite: a part under another parent is an id-conflict', kinds(diffTrees({ id: 'inv-5:inv-5-l1' }, { id: `${U2}:${U2}-l1` }, comp)).join() === 'id-conflict@id');
  check('composite: a differing literal part is a value diff', kinds(diffTrees({ k: 'year-end:inv-5' }, { k: `recurring:${U1}` }, comp)).join() === 'value@k');
}

// Mapped ids shown literally on Rust (the unremapped `highlight` link) stay conflicts.
{
  const ds = diffTrees({ link: { query: { highlight: 'pay-135' } } }, { link: { query: { highlight: 'pay-135' } } }, new IdMap([['pay-135', U1]]));
  check('ids: a mapped mock id repeated verbatim by Rust is an id-conflict', kinds(ds).join() === 'id-conflict@link.query.highlight', kinds(ds));
  check('ids: an unmapped equal string is equal', diffTrees({ h: 'pay-135' }, { h: 'pay-135' }, new IdMap()).length === 0);
}

// Object keys that are ids (P4-13 c).
{
  const ids = new IdMap([['branch-main', U1]]);
  const ok = diffTrees({ stockByBranch: { 'branch-main': { qty: 3 } } }, { stockByBranch: { [U1]: { qty: 3 } } }, ids);
  check('keys: a mapped key pairs with its Rust id', ok.length === 0, kinds(ok));
  const val = diffTrees({ stockByBranch: { 'branch-main': { qty: 3 } } }, { stockByBranch: { [U1]: { qty: 4 } } }, ids);
  check('keys: values under a translated key still diff (path keeps the mock key)', kinds(val).join() === 'value@stockByBranch.branch-main.qty', kinds(val));
  const learnt = new IdMap();
  const two = diffTrees({ products: { 'prd-1': { sku: 'A' }, 'prd-2': { sku: 'B' } } }, { products: { [U3]: { sku: 'B' }, [U2]: { sku: 'A' } } }, learnt);
  check('keys: unmapped keys pair with unknown UUID keys by value', two.length === 0 && learnt.m2r.get('prd-1') === U2 && learnt.m2r.get('prd-2') === U3, kinds(two));
  const lone = new IdMap();
  const one = diffTrees({ m: { 'branch-1': { qty: 1 } } }, { m: { [U2]: { qty: 2 } } }, lone);
  check('keys: a lone leftover pair is paired and its values diffed', kinds(one).join() === 'value@m.branch-1.qty' && lone.m2r.get('branch-1') === U2, kinds(one));
  const conflict = diffTrees({ m: { 'branch-main': 1 } }, { m: { [U2]: 1 } }, new IdMap([['branch-main', U1]]));
  check('keys: a mapped key facing another UUID key is missing + extra', kinds(conflict).join() === `missing@m.branch-main,extra@m.${U2}`, kinds(conflict));
  const literal = diffTrees({ m: { 'branch-main': 1 } }, { m: { 'branch-main': 1 } }, new IdMap([['branch-main', U1]]));
  check('keys: a mapped mock id used literally as a Rust key is an id-conflict', kinds(literal).join() === 'id-conflict@m.branch-main', kinds(literal));
  const stepNamed = diffTrees({ steps: { 'prd-1': { ok: true, value: 1 } } }, { steps: { 'prd-1': { ok: true, value: 1 } } }, new IdMap([['prd-1', U1]]));
  check('keys: harness containers (step names) compare literally, even when a step is named like an id', stepNamed.length === 0, kinds(stepNamed));
}

// Imported createdAt (00-import D-8, P4-13 d): only snapshot rows that had none.
{
  const snapshotIds = idsWithoutCreatedAt({ customers: [{ id: 'cus-1' }, { id: 'cus-2', createdAt: '2026-01-01T00:00:00.000Z' }] });
  check('createdAt: the snapshot set holds only rows without createdAt', snapshotIds.has('cus-1') && !snapshotIds.has('cus-2'));
  const opts = { importedWithoutCreatedAt: snapshotIds };
  const stamp = '2026-09-30T02:57:44.806Z';
  const imported = diffTrees({ rows: [{ id: 'cus-1' }] }, { rows: [{ id: U1, createdAt: stamp }] }, new IdMap([['cus-1', U1]]), opts);
  check('createdAt: Rust-only on an imported row without one is allowed (and still listed)', imported.length === 1 && imported[0].allowedBy === IMPORTED_CREATED_AT_REASON, imported);
  const created = diffTrees({ rows: [{ id: 'cus-99' }] }, { rows: [{ id: U2, createdAt: stamp }] }, new IdMap(), opts);
  check('createdAt: Rust-only on a row created during the case is a diff', created.length === 1 && !created[0].allowedBy, created);
  const had = diffTrees({ rows: [{ id: 'cus-2' }] }, { rows: [{ id: U3, createdAt: stamp }] }, new IdMap([['cus-2', U3]]), opts);
  check('createdAt: Rust-only on a snapshot row that had one is a diff', had.length === 1 && !had[0].allowedBy, had);
  const value = diffTrees({ rows: [{ id: 'cus-1', createdAt: '2026-06-30T09:00:00.000Z' }] }, { rows: [{ id: U1, createdAt: stamp }] }, new IdMap([['cus-1', U1]]), opts);
  check('createdAt: a differing value is a diff even on an imported row', kinds(value).join() === 'value@rows[0].createdAt', kinds(value));
}

// Epsilon: only on named paths, only within 1e-9.
{
  const opts = { epsilon: ['steps.*.value.rows[].qty'] };
  const inside = diffTrees({ steps: { r: { value: { rows: [{ qty: 1 }] } } } }, { steps: { r: { value: { rows: [{ qty: 1 + 1e-12 }] } } } }, new IdMap(), opts);
  check('epsilon: within 1e-9 on a named path passes', inside.length === 0, kinds(inside));
  const outside = diffTrees({ steps: { r: { value: { rows: [{ qty: 1 }] } } } }, { steps: { r: { value: { rows: [{ qty: 1.000001 }] } } } }, new IdMap(), opts);
  check('epsilon: beyond 1e-9 is a diff', kinds(outside).join() === 'value@steps.r.value.rows[0].qty', kinds(outside));
  const unnamed = diffTrees({ steps: { r: { value: { total: 1 } } } }, { steps: { r: { value: { total: 1 + 1e-12 } } } }, new IdMap(), opts);
  check('epsilon: an unnamed path is exact', unnamed.length === 1, kinds(unnamed));
}

// Arrays: ordered by default, multiset on `unordered` paths.
{
  const ordered = diffTrees({ l: [1, 2, 3] }, { l: [3, 1, 2] }, new IdMap());
  check('arrays: order matters by default', ordered.length === 3, kinds(ordered));
  const multi = diffTrees({ l: [{ id: 'a-1', n: 1 }, { id: 'a-2', n: 2 }] }, { l: [{ id: U2, n: 2 }, { id: U1, n: 1 }] }, new IdMap(), { unordered: ['l'] });
  check('arrays: multiset on an unordered path (ids learned per match)', multi.length === 0, kinds(multi));
  const multiBad = diffTrees({ l: [1, 2] }, { l: [2, 3] }, new IdMap(), { unordered: ['l'] });
  check('arrays: multiset reports both unmatched sides', kinds(multiBad).join() === 'unmatched@l[0],unmatched@l[1]', kinds(multiBad));
  const len = diffTrees({ l: [1] }, { l: [1, 2] }, new IdMap());
  check('arrays: an extra element is a diff', kinds(len).join() === 'extra@l[1]', kinds(len));
}

// [] / absent / null (P4-13 a: null ≡ absent; [] / 0 / false vs absent stay diffs).
{
  const ds = diffTrees(normalize({ a: [], b: undefined, c: null }), normalize({ c: undefined, b: null, a: [] }), new IdMap());
  check('null vs absent is equal (P4-13 a); undefined keys are absent after normalize', ds.length === 0, kinds(ds));
  const defaults = diffTrees({ n: 0, f: false, s: '' }, {}, new IdMap());
  check('0 / false / "" vs absent are diffs', kinds(defaults).join() === 'missing@n,missing@f,missing@s', kinds(defaults));
  check('null vs a value is a type diff', kinds(diffTrees({ p: null }, { p: 'x' }, new IdMap())).join() === 'type@p');
  const emptyVsAbsent = diffTrees({ attachmentIds: [] }, {}, new IdMap());
  check('[] vs absent is a diff', kinds(emptyVsAbsent).join() === 'missing@attachmentIds', kinds(emptyVsAbsent));
  check('normalize: a void result records null', normalize(undefined) === null);
}

// Instants compare as epoch ms; plain dates stay exact.
{
  check('instants: Z vs +00:00 and extra fraction digits are equal', diffTrees({ t: '2026-06-30T09:00:00.000Z' }, { t: '2026-06-30T09:00:00+00:00' }, new IdMap()).length === 0);
  check('instants: a different instant is a diff', diffTrees({ t: '2026-06-30T09:00:00.000Z' }, { t: '2026-06-30T09:00:01.000Z' }, new IdMap()).length === 1);
  check('dates: YYYY-MM-DD compares exactly', diffTrees({ d: '2026-06-30' }, { d: '2026-07-01' }, new IdMap()).length === 1);
}

// Errors compare {code, message} byte for byte.
{
  const a = { steps: { refund: { ok: false, error: { code: 'VALIDATION', message: 'الكمية المرتجعة أكبر من المباعة' }, expectedError: true } } };
  const b = { steps: { refund: { ok: false, error: { code: 'VALIDATION', message: 'الكمية المرتجعة أكبر من المباعه' }, expectedError: true } } };
  const ds = diffTrees(a, b, new IdMap());
  check('errors: a one-letter message mismatch is a diff', kinds(ds).join() === 'value@steps.refund.error.message', kinds(ds));
  const code = diffTrees(a, { steps: { refund: { ...b.steps.refund, error: { code: 'CONFLICT', message: a.steps.refund.error.message } } } }, new IdMap());
  check('errors: a code mismatch is a diff', kinds(code).join() === 'value@steps.refund.error.code', kinds(code));
}

// Allowlist: reasons must cite an id; entries cover their node and everything below it.
{
  check('allow: "looks harmless" is rejected', validateAllow([{ path: 'x', reason: 'looks harmless' }], 't').length === 1);
  check('allow: a section number alone (§8b) is rejected', !citesDecision('13b §8b says so'));
  for (const r of ['A-D1: added audit row', 'Q6 set compare', 'R-7 epsilon', '12-accounting #10: [] vs absent', '12 #10', 'D-8 backdated seed order', 'Q-I10', 'P4-6']) {
    check(`allow: "${r}" cites an id`, citesDecision(r));
  }
  const ds = diffTrees({ steps: { post: { value: { attachmentIds: [] } } } }, { steps: { post: { value: {} } } }, new IdMap());
  applyAllow(ds, [{ path: 'steps.*.value.attachmentIds', reason: '12-accounting #10: [] vs absent' }]);
  check('allow: a matching entry explains the diff', ds.length === 1 && ds[0].allowedBy === '12-accounting #10: [] vs absent', ds);
  const deeper = diffTrees({ l: { rows: [1] } }, { l: { rows: [2] } }, new IdMap());
  applyAllow(deeper, [{ path: 'l', reason: 'Q6' }]);
  check('allow: an entry covers paths below its node', deeper[0]?.allowedBy === 'Q6', deeper);
  const sibling = diffTrees({ list: 1, listing: 1 }, { list: 2, listing: 2 }, new IdMap());
  applyAllow(sibling, [{ path: 'list', reason: 'Q6' }]);
  check('allow: an entry does not cover a sibling sharing its prefix', sibling.filter((d) => !d.allowedBy).map((d) => d.path).join() === 'listing', sibling);
  const arr = diffTrees({ a: { id: 'x' }, b: { id: 'y' }, c: { id: 'z', prices: [1] } }, { a: { id: 'x', prices: [] }, b: { id: 'y', prices: [1] }, c: { id: 'z', prices: [2] } }, new IdMap());
  applyAllow(arr, [{ path: '**.prices', reason: '06 Q-7', emptyArrayOnly: true }]);
  check('allow: emptyArrayOnly covers only [] vs absent', arr.filter((d) => !d.allowedBy).map((d) => d.path).join() === 'b.prices,c.prices[0]', arr);
  const days = diffTrees(
    { a: { d: '2026-06-29T00:00:00.000Z' }, b: { d: '2026-06-29T00:00:00.000Z' }, c: { d: '2026-06-29T00:00:00.000Z' } },
    { a: { d: '2026-06-29' }, b: { d: '2026-07-02' }, c: {} },
    new IdMap(),
  );
  applyAllow(days, [{ path: '*.d', reason: '07 D-U8', dayOfInstantOnly: true }]);
  check('allow: dayOfInstantOnly covers only an instant vs its own day', days.filter((d) => !d.allowedBy).map((d) => d.path).join() === 'b.d,c.d', days);
}

console.log(failures === 0 ? '\ndiff self-test: all green' : `\ndiff self-test: ${failures} failing`);
process.exit(failures === 0 ? 0 : 1);
