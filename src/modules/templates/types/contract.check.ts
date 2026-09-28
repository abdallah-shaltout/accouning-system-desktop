/**
 * Drift check (21.02-F, F-4 / 03-domains/15-templates.md §2): proves the ts-rs-generated
 * `templates` DTOs (`templates/types/gen/*`, written by `bun run bindings` from
 * `src-tauri/src/domains/templates/dto.rs`) have exactly the same shape as the hand-written TS
 * types in `./index.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect } from '@/modules/core/types/contract';
import type { BaseTemplateId, DocumentKind, PdfTemplate } from './index';

import type { BaseTemplateId as GenBaseTemplateId } from './gen/BaseTemplateId';
import type { DocumentKind as GenDocumentKind } from './gen/DocumentKind';
import type { PdfTemplate as GenPdfTemplate } from './gen/PdfTemplate';

export type _DocumentKind = Expect<Equals<GenDocumentKind, DocumentKind>>;
export type _BaseTemplateId = Expect<Equals<GenBaseTemplateId, BaseTemplateId>>;

// `PdfTemplate.options` is opaque JSON on the Rust side, typed via `#[ts(type = "import('../index')
// .TemplateOptions")]` (T-5) — the generated field is already `TemplateOptions`, so no separate
// conversion is needed here beyond the whole-struct equality below.
// contract-ok: PdfTemplate.options is opaque JSON typed as TemplateOptions (T-5)
export type _PdfTemplate = Expect<Equals<GenPdfTemplate, PdfTemplate>>;

// `LabelOptions`/`LabelPreset` are not DTOs — no service function returns them (AT§2), so they have
// no generated counterpart and are intentionally not checked here.
