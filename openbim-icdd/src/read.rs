//! ICDD container reader — open + decode `Index.rdf` and the linksets.
//!
//! The submodule split keeps container, index, linkset, RDF graph, and vocabulary
//! responsibilities independently testable.
//!
//! ## Example
//! ```no_run
//! use openbim_icdd::IcddContainer;
//! let mut c = IcddContainer::open_path("model.icdd").unwrap();
//! println!("{} documents, {} linksets", c.index().documents.len(), c.linksets().len());
//! // Clone the doc refs first so the immutable borrow ends before payload_bytes().
//! let ifcs: Vec<_> = c.ifc_documents().into_iter().cloned().collect();
//! for ifc in &ifcs {
//!     let bytes = c.payload_bytes(ifc).unwrap();
//!     // hand `bytes` to an IFC consumer ...
//! }
//! ```

use super::container::{self, ContainerZip};
use super::error::IcddError;
use super::schema::*;
use super::{index, linkset, vocab};

use std::fs::File;
use std::io::{BufReader, Cursor, Read, Seek};
use std::path::Path;

/// A parsed, validated ICDD container. Owns the open ZIP for lazy payload byte
/// access; the RDF (index + linksets) is decoded eagerly at open time.
pub struct IcddContainer<R: Read + Seek> {
    zip: ContainerZip<R>,
    index: ContainerIndex,
    linksets: Vec<LinkSet>,
    top_folders: Vec<String>,
}

impl IcddContainer<BufReader<File>> {
    /// Open a container from a filesystem path.
    pub fn open_path(path: impl AsRef<Path>) -> Result<Self, IcddError> {
        let f = File::open(path)?;
        Self::open(BufReader::new(f))
    }
}

fn safe_relative_payload_path(path: &str) -> Result<std::path::PathBuf, IcddError> {
    use std::path::{Component, Path, PathBuf};

    let normalized = path.replace('\\', "/");
    let mut safe = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(segment) => safe.push(segment),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(IcddError::NotConformant(format!(
                    "unsafe payload path: {path}"
                )));
            }
        }
    }
    if safe.as_os_str().is_empty() {
        return Err(IcddError::NotConformant("empty payload path".into()));
    }
    Ok(safe)
}

impl IcddContainer<Cursor<Vec<u8>>> {
    /// Open a container from an in-memory byte buffer (e.g. an HTTP upload).
    pub fn open_bytes(bytes: Vec<u8>) -> Result<Self, IcddError> {
        Self::open(Cursor::new(bytes))
    }
}

impl<R: Read + Seek> IcddContainer<R> {
    /// Open + parse a container from any seekable reader.
    pub fn open(reader: R) -> Result<Self, IcddError> {
        let mut zip = ContainerZip::open(reader)?;
        let top_folders = zip.top_folders();

        // Parse the root Index.rdf (Container ontology).
        let index_bytes = zip.index_bytes()?;
        let index = index::parse_index(&index_bytes)?;

        // Parse every linkset. Prefer the index's referenced filenames; union
        // with any *.rdf actually present under Payload triples/ (some real
        // containers under-list them).
        let mut linkset_paths: Vec<String> = index
            .linkset_files
            .iter()
            .filter_map(|l| l.filename.clone())
            .collect();
        for p in zip.linkset_paths() {
            // p is a full archive path; store the payload-triples-relative name.
            let rel = p
                .strip_prefix(&format!("{}/", container::PAYLOAD_TRIPLES_DIR))
                .unwrap_or(&p)
                .to_string();
            if !linkset_paths.iter().any(|x| x == &rel || x == &p) {
                linkset_paths.push(rel);
            }
        }

        let mut linksets = Vec::new();
        for name in linkset_paths {
            match zip.linkset_bytes(&name) {
                Ok(bytes) => linksets.push(linkset::parse_linkset(&name, &bytes)?),
                Err(_) => { /* referenced-but-absent linkset: skip, stay lenient */ }
            }
        }

        Ok(IcddContainer {
            zip,
            index,
            linksets,
            top_folders,
        })
    }

    /// Read the original `Index.rdf` bytes, including unknown extension triples.
    pub fn index_rdf_bytes(&mut self) -> Result<Vec<u8>, IcddError> {
        self.zip.index_bytes()
    }

    /// Read one original linkset RDF/XML file by its relative filename.
    pub fn linkset_rdf_bytes(&mut self, filename: &str) -> Result<Vec<u8>, IcddError> {
        self.zip.linkset_bytes(filename)
    }

    /// The parsed container index (manifest + documents + linkset refs).
    pub fn index(&self) -> &ContainerIndex {
        &self.index
    }

    /// The parsed linksets (the cross-document links — the ICDD-only capability).
    pub fn linksets(&self) -> &[LinkSet] {
        &self.linksets
    }

    /// The top-level ZIP folders present (for conformance reporting).
    pub fn top_folders(&self) -> &[String] {
        &self.top_folders
    }

    /// The IFC payload documents (what we hand to `ifc2smc`). By default only
    /// **available** payloads are returned — an IFC document that is a
    /// `ct:requested` slot (a placeholder for a not-yet-delivered file, with no
    /// bytes in `Payload documents/`) is excluded. Use
    /// [`Self::ifc_documents_including_requested`] to see slots too.
    pub fn ifc_documents(&self) -> Vec<&Document> {
        self.index
            .documents
            .iter()
            .filter(|d| d.is_ifc() && !d.requested && d.internal_path().is_some())
            .collect()
    }

    /// Every IFC-typed document, INCLUDING `ct:requested` slots and external
    /// references (used for reporting what a container declares vs. delivers).
    pub fn ifc_documents_including_requested(&self) -> Vec<&Document> {
        self.index.documents.iter().filter(|d| d.is_ifc()).collect()
    }

    /// Read the bytes of an internal payload document.
    pub fn payload_bytes(&mut self, doc: &Document) -> Result<Vec<u8>, IcddError> {
        let path = doc.internal_path().ok_or_else(|| {
            IcddError::NotConformant(format!(
                "document {} is not an internal document (no in-container bytes)",
                doc.id
            ))
        })?;
        self.zip.payload_bytes(path)
    }

    /// Extract every internal payload document to `dir`, preserving relative
    /// paths. Returns the list of `(document, written_path)` pairs.
    pub fn extract_payloads(
        &mut self,
        dir: impl AsRef<Path>,
    ) -> Result<Vec<(String, std::path::PathBuf)>, IcddError> {
        let dir = dir.as_ref();
        let docs: Vec<(String, String)> = self
            .index
            .documents
            .iter()
            .filter_map(|d| d.internal_path().map(|p| (d.id.clone(), p.to_string())))
            .collect();
        let mut out = Vec::new();
        for (id, rel) in docs {
            let bytes = self.zip.payload_bytes(&rel)?;
            let target = dir.join(safe_relative_payload_path(&rel)?);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target, bytes)?;
            out.push((id, target));
        }
        Ok(out)
    }

    /// Conformance check per ISO 21597-1 Clause 5. Returns the list of
    /// unmet-requirement messages (empty = conformant). Non-fatal: `open`
    /// succeeds on lenient parse; call this to REPORT conformance.
    pub fn conformance_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();
        for folder in [
            container::ONTOLOGY_DIR,
            container::PAYLOAD_DOCS_DIR,
            container::PAYLOAD_TRIPLES_DIR,
        ] {
            if !self.top_folders.iter().any(|f| f == folder) {
                issues.push(format!("missing top-level folder '{folder}'"));
            }
        }
        match &self.index.description.conformance_indicator {
            Some(v) if v == vocab::CONFORMANCE_INDICATOR => {}
            Some(v) => issues.push(format!(
                "ct:conformanceIndicator is '{v}', expected '{}'",
                vocab::CONFORMANCE_INDICATOR
            )),
            None => issues.push("ct:conformanceIndicator is absent".into()),
        }
        issues
    }
}
