//! Open an `.icdd` ZIP, validate the ISO 21597-1 folder structure, and provide
//! byte access to the root `Index.rdf`, the linkset files, and payload
//! documents. Lenient by design (like `sol-smc`'s reader): we locate members by
//! role, not by exact-case name, because the official ISO reference containers
//! use `index.rdf` (lowercase) and mixed-case base namespaces.

use super::error::IcddError;
use std::io::Read;
use zip::ZipArchive;

/// A validated view over the ZIP entries of an ICDD container.
pub struct ContainerZip<R: Read + std::io::Seek> {
    zip: ZipArchive<R>,
    /// The archive path of the root index (`index.rdf` / `Index.rdf`).
    index_path: String,
}

/// The three mandatory top-level folder names (ISO 21597-1 Clause 6).
pub const ONTOLOGY_DIR: &str = "Ontology resources";
/// Payload documents folder.
pub const PAYLOAD_DOCS_DIR: &str = "Payload documents";
/// Payload triples (linksets) folder.
pub const PAYLOAD_TRIPLES_DIR: &str = "Payload triples";

/// Maximum uncompressed size accepted for `Index.rdf`.
pub const MAX_INDEX_RDF_BYTES: u64 = 16 * 1024 * 1024;
/// Maximum uncompressed size accepted for one linkset RDF graph.
pub const MAX_LINKSET_RDF_BYTES: u64 = 64 * 1024 * 1024;

impl<R: Read + std::io::Seek> ContainerZip<R> {
    /// Open + validate a container from a seekable reader.
    pub fn open(reader: R) -> Result<Self, IcddError> {
        let mut zip = ZipArchive::new(reader)?;

        // Find the root index: a `*.rdf` at the archive root (no `/`), named
        // `index.rdf` case-insensitively. Fall back to any root `*.rdf` that is
        // not one of the ontology files.
        let mut index_path: Option<String> = None;
        let mut root_rdfs: Vec<String> = Vec::new();
        for i in 0..zip.len() {
            let name = zip.by_index(i)?.name().to_string();
            if name.contains('/') {
                continue; // not at root
            }
            if name.eq_ignore_ascii_case("index.rdf") {
                index_path = Some(name);
                break;
            }
            if name.to_ascii_lowercase().ends_with(".rdf") {
                root_rdfs.push(name);
            }
        }
        let index_path = index_path
            .or_else(|| root_rdfs.into_iter().next())
            .ok_or_else(|| IcddError::NotConformant("no Index.rdf at the container root".into()))?;

        Ok(ContainerZip { zip, index_path })
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
        self.entry_bytes(&path)
    }

    /// Read a linkset file by its `ct:filename` (relative to `Payload triples/`).
    pub fn linkset_bytes(&mut self, filename: &str) -> Result<Vec<u8>, IcddError> {
        let path = self.resolve(PAYLOAD_TRIPLES_DIR, filename)?;
        self.entry_bytes_limited(&path, MAX_LINKSET_RDF_BYTES)
    }

    /// All linkset file paths actually present under `Payload triples/`
    /// (`*.rdf`), regardless of whether the index references them. Used as a
    /// fallback when the index's `ct:containsLinkset` list is incomplete.
    pub fn linkset_paths(&mut self) -> Vec<String> {
        let prefix = format!("{PAYLOAD_TRIPLES_DIR}/");
        (0..self.zip.len())
            .filter_map(|i| self.zip.by_index(i).ok().map(|f| f.name().to_string()))
            .filter(|n| n.starts_with(&prefix) && n.to_ascii_lowercase().ends_with(".rdf"))
            .collect()
    }

    /// The set of top-level folders present (for conformance reporting).
    pub fn top_folders(&mut self) -> Vec<String> {
        let mut set = std::collections::BTreeSet::new();
        for i in 0..self.zip.len() {
            if let Ok(f) = self.zip.by_index(i) {
                if let Some((top, _)) = f.name().split_once('/') {
                    set.insert(top.to_string());
                }
            }
        }
        set.into_iter().collect()
    }

    /// Extract a raw archive entry by its exact path.
    pub fn entry_bytes(&mut self, path: &str) -> Result<Vec<u8>, IcddError> {
        let mut f = self.zip.by_name(path)?;
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf)?;
        Ok(buf)
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

    /// Resolve a `ct:filename`/`ct:foldername` (which is relative to the payload
    /// folder) to a real archive path, tolerating slashes and an already-present
    /// folder prefix; falls back to a case-insensitive suffix match.
    fn resolve(&mut self, folder: &str, name: &str) -> Result<String, IcddError> {
        let norm = name.replace('\\', "/");
        let norm = norm.trim_start_matches('/');
        let candidate = if norm.starts_with(folder) {
            norm.to_string()
        } else {
            format!("{folder}/{norm}")
        };
        // Exact hit?
        for i in 0..self.zip.len() {
            if self.zip.by_index(i)?.name() == candidate {
                return Ok(candidate);
            }
        }
        // Case-insensitive suffix fallback (handles case/encoding drift).
        let want = candidate.to_ascii_lowercase();
        for i in 0..self.zip.len() {
            let n = self.zip.by_index(i)?.name().to_string();
            if n.to_ascii_lowercase() == want
                || n.to_ascii_lowercase()
                    .ends_with(&format!("/{}", norm.to_ascii_lowercase()))
            {
                return Ok(n);
            }
        }
        Err(IcddError::NotConformant(format!(
            "payload entry not found: {candidate}"
        )))
    }
}
