//! Error type for the ICDD reader.

use std::io;

/// Errors that can occur opening or parsing an ISO 21597-1 container.
#[derive(Debug, thiserror::Error)]
pub enum IcddError {
    /// I/O failure reading the file / ZIP entry.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// The ZIP archive is malformed or not a ZIP.
    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// RDF/XML parse failure inside `Index.rdf` or a linkset.
    #[error("rdf parse error: {0}")]
    Rdf(String),

    /// A conformance requirement was not met (ISO 21597-1 Clause 5).
    #[error("not a conformant ICDD container: {0}")]
    NotConformant(String),
}
