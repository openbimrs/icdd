//! Open an `.icdd` ZIP, validate the ISO 21597-1 folder structure, and provide
//! byte access to the root `Index.rdf`, the linkset files, and payload
//! documents. Lenient by design (like `sol-smc`'s reader): we locate members by
//! role, not by exact-case name, because the official ISO reference containers
//! use `index.rdf` (lowercase) and mixed-case base namespaces.

use super::error::IcddError;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Component, Path};
use zip::ZipArchive;

/// A validated view over the ZIP entries of an ICDD container.
pub struct ContainerZip<R: Read + std::io::Seek> {
    zip: ZipArchive<R>,
    /// Lowercase archive path to its unique original spelling.
    entries: BTreeMap<String, String>,
    /// The archive path of the root index (`index.rdf` / `Index.rdf`).
    index_path: String,
}

/// The three mandatory top-level folder names (ISO 21597-1 Clause 6).
pub const ONTOLOGY_DIR: &str = "Ontology resources";
/// Payload documents folder.
pub const PAYLOAD_DOCS_DIR: &str = "Payload documents";
/// Payload triples (linksets) folder.
pub const PAYLOAD_TRIPLES_DIR: &str = "Payload triples";

/// Maximum number of ZIP entries accepted in one container.
pub const MAX_ARCHIVE_ENTRIES: usize = 100_000;
/// Maximum uncompressed size returned by one in-memory payload read.
pub const MAX_IN_MEMORY_PAYLOAD_BYTES: u64 = 1024 * 1024 * 1024;
/// Maximum uncompressed size streamed for one extracted payload.
pub const MAX_STREAMED_PAYLOAD_BYTES: u64 = 16 * 1024 * 1024 * 1024;
/// Maximum total uncompressed payload bytes extracted by one call.
pub const MAX_TOTAL_EXTRACTED_BYTES: u64 = 64 * 1024 * 1024 * 1024;
/// Maximum uncompressed size accepted for `Index.rdf`.
pub const MAX_INDEX_RDF_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum number of linkset RDF graphs parsed from one container.
pub const MAX_LINKSET_COUNT: usize = 10_000;
/// Maximum aggregate uncompressed bytes of eagerly parsed RDF metadata.
pub const MAX_TOTAL_RDF_BYTES: u64 = 256 * 1024 * 1024;
/// Maximum uncompressed size accepted for one linkset RDF graph.
pub const MAX_LINKSET_RDF_BYTES: u64 = 64 * 1024 * 1024;

impl<R: Read + std::io::Seek> ContainerZip<R> {
    /// Open + validate a container from a seekable reader.
    pub fn open(reader: R) -> Result<Self, IcddError> {
        let mut zip = ZipArchive::new(reader)?;
        if zip.len() > MAX_ARCHIVE_ENTRIES {
            return Err(IcddError::NotConformant(format!(
                "archive contains {} entries, exceeding the {MAX_ARCHIVE_ENTRIES}-entry limit",
                zip.len()
            )));
        }

        let mut entries = BTreeMap::new();
        let mut index_paths = Vec::new();
        for i in 0..zip.len() {
            let name = zip.by_index(i)?.name().to_string();
            validate_entry_name(&name)?;
            let folded = name.to_ascii_lowercase();
            if let Some(previous) = entries.insert(folded, name.clone()) {
                return Err(IcddError::NotConformant(format!(
                    "duplicate case-folded ZIP entry names: {previous:?} and {name:?}"
                )));
            }
            if !name.contains('/') && name.eq_ignore_ascii_case("index.rdf") {
                index_paths.push(name);
            }
        }
        if index_paths.len() != 1 {
            return Err(IcddError::NotConformant(format!(
                "expected exactly one case-insensitive Index.rdf at the container root, found {}",
                index_paths.len()
            )));
        }
        let index_path = index_paths.pop().expect("length checked above");

        Ok(ContainerZip {
            zip,
            entries,
            index_path,
        })
    }

    /// Read the root `Index.rdf` bytes.
    pub fn index_bytes(&mut self) -> Result<Vec<u8>, IcddError> {
        self.entry_bytes_limited(&self.index_path.clone(), MAX_INDEX_RDF_BYTES)
    }

    /// Read a payload document by its `ct:filename` (relative to
    /// `Payload documents/`). Tolerant of a leading `Payload documents/` already
    /// being present and of forward/back slashes.
    pub fn payload_bytes(&mut self, filename: &str) -> Result<Vec<u8>, IcddError> {
        let path = self.resolve(PAYLOAD_DOCS_DIR, filename)?;
        self.entry_bytes_limited(&path, MAX_IN_MEMORY_PAYLOAD_BYTES)
    }

    /// Stream a payload document to a writer without materializing it in memory.
    pub fn copy_payload_to(
        &mut self,
        filename: &str,
        writer: &mut impl std::io::Write,
    ) -> Result<u64, IcddError> {
        let path = self.resolve(PAYLOAD_DOCS_DIR, filename)?;
        let mut file = self.zip.by_name(&path)?;
        if file.size() > MAX_STREAMED_PAYLOAD_BYTES {
            return Err(IcddError::NotConformant(format!(
                "archive entry {path:?} exceeds the {MAX_STREAMED_PAYLOAD_BYTES}-byte extraction limit"
            )));
        }
        let copied = std::io::copy(
            &mut (&mut file).take(MAX_STREAMED_PAYLOAD_BYTES + 1),
            writer,
        )?;
        if copied > MAX_STREAMED_PAYLOAD_BYTES {
            return Err(IcddError::NotConformant(format!(
                "archive entry {path:?} exceeds the {MAX_STREAMED_PAYLOAD_BYTES}-byte extraction limit"
            )));
        }
        Ok(copied)
    }

    /// Read a linkset file by its `ct:filename` (relative to `Payload triples/`).
    pub fn linkset_bytes(&mut self, filename: &str) -> Result<Vec<u8>, IcddError> {
        let path = self.resolve(PAYLOAD_TRIPLES_DIR, filename)?;
        self.entry_bytes_limited(&path, MAX_LINKSET_RDF_BYTES)
    }

    /// The set of top-level folders present (for conformance reporting).
    pub fn top_folders(&self) -> Vec<String> {
        let mut set = std::collections::BTreeSet::new();
        for name in self.entries.values() {
            if let Some((top, _)) = name.split_once('/') {
                set.insert(top.to_string());
            }
        }
        set.into_iter().collect()
    }

    fn entry_bytes_limited(&mut self, path: &str, limit: u64) -> Result<Vec<u8>, IcddError> {
        let mut file = self.zip.by_name(path)?;
        if file.size() > limit {
            return Err(IcddError::NotConformant(format!(
                "archive entry {path:?} exceeds the {limit}-byte metadata limit"
            )));
        }
        let capacity = usize::try_from(file.size())
            .unwrap_or(usize::MAX)
            .min(limit as usize);
        let mut bytes = Vec::with_capacity(capacity);
        file.read_to_end(&mut bytes)?;
        if bytes.len() as u64 > limit {
            return Err(IcddError::NotConformant(format!(
                "archive entry {path:?} exceeds the {limit}-byte metadata limit"
            )));
        }
        Ok(bytes)
    }

    /// Resolve a canonical `ct:filename` relative to its required ICDD folder.
    /// Exact spelling is preferred; one case-insensitive full-path match is
    /// accepted because official examples vary the case of `Index.rdf`.
    fn resolve(&self, folder: &str, name: &str) -> Result<String, IcddError> {
        validate_declared_path(name)?;
        let prefixed = format!("{folder}/");
        let candidate = if name
            .get(..prefixed.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(&prefixed))
        {
            name.to_string()
        } else {
            format!("{folder}/{name}")
        };
        self.entries
            .get(&candidate.to_ascii_lowercase())
            .cloned()
            .ok_or_else(|| {
                IcddError::NotConformant(format!("payload entry not found: {candidate}"))
            })
    }

    /// Validate a declared payload path without requiring the entry to exist.
    pub fn validate_payload_reference(&self, filename: &str) -> Result<(), IcddError> {
        validate_declared_path(filename)
    }

    /// Whether a declared internal payload resolves to one unique archive entry.
    pub fn contains_payload(&self, filename: &str) -> bool {
        self.resolve(PAYLOAD_DOCS_DIR, filename).is_ok()
    }
}

fn validate_entry_name(name: &str) -> Result<(), IcddError> {
    let path = name.strip_suffix('/').unwrap_or(name);
    validate_declared_path(path).map_err(|_| {
        IcddError::NotConformant(format!("unsafe or noncanonical ZIP entry path: {name:?}"))
    })
}

fn validate_declared_path(name: &str) -> Result<(), IcddError> {
    if name.is_empty()
        || name.contains('\\')
        || name.starts_with('/')
        || name.ends_with('/')
        || name.contains("//")
    {
        return Err(IcddError::NotConformant(format!(
            "unsafe or noncanonical archive path: {name:?}"
        )));
    }
    if Path::new(name)
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(IcddError::NotConformant(format!(
            "unsafe or noncanonical archive path: {name:?}"
        )));
    }
    Ok(())
}
