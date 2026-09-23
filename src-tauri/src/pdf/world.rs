//! A minimal `typst::World` implementation for the PDF spike.
//!
//! Holds everything Typst needs as in-memory "virtual files": the template
//! source (`main.typ`), a JSON data file (`data.json`), a QR PNG (`qr.png`),
//! and embedded fonts. No filesystem access beyond what was baked in via
//! `include_bytes!` at compile time — matches the "No Typst packages:
//! everything must work offline" rule in docs/v2/12-documents-pdf-excel.md §2.

use std::collections::HashMap;
use std::path::PathBuf;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt};

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
