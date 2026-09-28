//! `typst::World` implementations for the PDF engine.
//!
//! `SpikeWorld` is the original Phase 0 gate world (kept as-is so the spike
//! binary/command still work unchanged). `RenderWorld` is the production
//! Phase 11a world: it holds a swappable main template (either a built-in
//! `.typ` file or custom source from the advanced editor), the shared
//! `lib.typ`, `data.json`/`opts.json`, an optional logo image, an optional
//! QR SVG, and all embedded font families — as in-memory "virtual files",
//! matching the spike's "no filesystem access beyond what was baked in"
//! approach (docs/v2/12-documents-pdf-excel.md §2: "No Typst packages:
//! everything must work offline").

use std::collections::HashMap;
use std::path::PathBuf;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};

use super::fonts::FontSpec;

pub struct SpikeWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main_id: FileId,
    source: Source,
    files: HashMap<FileId, Bytes>,
}

impl SpikeWorld {
    /// Builds a world with one main template, one JSON data file, one QR PNG,
    /// and the given font byte buffers (each may contain multiple named
    /// instances / a variable font — `Font::new` with index 0 is used for
    /// each buffer, which is sufficient for the single-face TTFs used here).
    pub fn new(
        template_source: String,
        data_json: String,
        qr_png: Vec<u8>,
        font_bytes: Vec<Vec<u8>>,
    ) -> Result<Self, String> {
        let main_path = VirtualPath::new("/main.typ")
            .map_err(|e| format!("invalid virtual path: {e}"))?;
        let main_id = RootedPath::new(VirtualRoot::Project, main_path).intern();
        let source = Source::new(main_id, template_source);

        let mut files = HashMap::new();
        files.insert(
            Self::intern("/data.json")?,
            Bytes::from_string(data_json),
        );
        files.insert(Self::intern("/qr.png")?, Bytes::new(qr_png));

        let mut fonts = Vec::new();
        for bytes in font_bytes {
            let data = Bytes::new(bytes);
            if let Some(font) = Font::new(data, 0) {
                fonts.push(font);
            } else {
                return Err("failed to parse embedded font".to_string());
            }
        }
        if fonts.is_empty() {
            return Err("no fonts loaded".to_string());
        }

        let book = FontBook::from_fonts(&fonts);

        Ok(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main_id,
            source,
            files,
        })
    }

    fn intern(path: &str) -> Result<FileId, String> {
        let vpath =
            VirtualPath::new(path).map_err(|e| format!("invalid virtual path: {e}"))?;
        Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
    }
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(PathBuf::from(id.vpath().get_with_slash()))
}

impl typst::World for SpikeWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.source.clone())
        } else {
            Err(not_found(id))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.get(&id).cloned().ok_or_else(|| not_found(id))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        Datetime::from_ymd(2026, 9, 23)
    }
}

/// One in-memory virtual file: a Typst source file (parsed, reparented on
/// edit) or a raw byte asset (JSON data, images, fonts don't go through this
/// map — see `fonts` below).
enum VirtualFile {
    Source(Source),
    Bytes(Bytes),
}

/// The production Phase 11a world. Holds:
/// - `main.typ`: either a built-in template's source or custom source from
///   the advanced editor.
/// - `lib.typ`: the shared header/party-box/table/totals/QR/footer helpers,
///   always present so both built-in and custom templates can `#import
///   "lib.typ": ...`.
/// - `data.json` / `opts.json`: the `DocumentPayload` / `TemplateOptions`
///   JSON, passed through opaquely from the TS caller.
/// - `logo.png` (optional): the company logo, decoded from a data: URL.
/// - `qr.svg` (optional): the pre-rendered QR SVG markup from `zatcaQr.ts` + `uqr`.
/// - All embedded font families (Cairo, Noto Naskh Arabic, IBM Plex Sans
///   Arabic, Tajawal).
pub struct RenderWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main_id: FileId,
    files: HashMap<FileId, VirtualFile>,
}

impl RenderWorld {
    pub fn new(
        main_source: String,
        lib_source: String,
        data_json: String,
        opts_json: String,
        logo_png: Option<Vec<u8>>,
        qr_svg: Option<String>,
        font_specs: Vec<FontSpec>,
    ) -> Result<Self, String> {
        let main_path = VirtualPath::new("/main.typ")
            .map_err(|e| format!("invalid virtual path: {e}"))?;
        let main_id = RootedPath::new(VirtualRoot::Project, main_path).intern();

        let mut files: HashMap<FileId, VirtualFile> = HashMap::new();
        files.insert(main_id, VirtualFile::Source(Source::new(main_id, main_source)));

        let lib_id = Self::intern("/lib.typ")?;
        files.insert(lib_id, VirtualFile::Source(Source::new(lib_id, lib_source)));

        files.insert(Self::intern("/data.json")?, VirtualFile::Bytes(Bytes::from_string(data_json)));
        files.insert(Self::intern("/opts.json")?, VirtualFile::Bytes(Bytes::from_string(opts_json)));

        if let Some(png) = logo_png {
            files.insert(Self::intern("/logo.png")?, VirtualFile::Bytes(Bytes::new(png)));
        }
        // Always provide qr.svg so templates can reference it unconditionally;
        // an empty/1x1 transparent SVG when there's no real QR to show.
        let qr_svg = qr_svg.unwrap_or_else(|| {
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\"></svg>".to_string()
        });
        files.insert(Self::intern("/qr.svg")?, VirtualFile::Bytes(Bytes::from_string(qr_svg)));

        let mut fonts = Vec::new();
        for spec in font_specs {
            let data = Bytes::new(spec.bytes.to_vec());
            if let Some(font) = Font::new(data, 0) {
                fonts.push(font);
            } else {
                return Err(format!("failed to parse embedded font for family '{}'", spec.family));
            }
        }
        if fonts.is_empty() {
            return Err("no fonts loaded".to_string());
        }

        let book = FontBook::from_fonts(&fonts);

        Ok(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main_id,
            files,
        })
    }

    fn intern(path: &str) -> Result<FileId, String> {
        let vpath = VirtualPath::new(path).map_err(|e| format!("invalid virtual path: {e}"))?;
        Ok(RootedPath::new(VirtualRoot::Project, vpath).intern())
    }

    /// The main source, for inline-compile-error line lookups after a failed
    /// compile (the advanced editor needs `world.source(world.main())`, but
    /// callers that already hold the world's main id can use this directly).
    pub fn main_id(&self) -> FileId {
        self.main_id
    }
}

impl typst::World for RenderWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        match self.files.get(&id) {
            Some(VirtualFile::Source(s)) => Ok(s.clone()),
            _ => Err(not_found(id)),
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        match self.files.get(&id) {
            Some(VirtualFile::Bytes(b)) => Ok(b.clone()),
            Some(VirtualFile::Source(s)) => Ok(Bytes::from_string(s.text().to_string())),
            None => Err(not_found(id)),
        }
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<typst::foundations::Duration>) -> Option<Datetime> {
        // Real wall-clock date for production renders (the spike pinned this
        // for determinism; production documents should show the real date).
        let now = time::OffsetDateTime::now_utc();
        Datetime::from_ymd(now.year(), now.month() as u8, now.day())
    }
}
