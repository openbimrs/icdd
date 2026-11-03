//! Deterministic ICDD archive construction using the maintained `zip` crate.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Write};
use std::path::{Component, Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use crate::container::{ONTOLOGY_DIR, PAYLOAD_DOCS_DIR, PAYLOAD_TRIPLES_DIR};
use crate::error::IcddError;
use crate::index::parse_index;
use crate::linkset::parse_linkset;
use crate::schema::{ContainerIndex, DocumentKind, LinkSet};
use crate::vocab;

/// Builds a deterministic ISO 21597-1 ZIP container.
///
/// RDF/XML is validated through `oxrdfxml` before any archive bytes are emitted.
/// Payload documents remain opaque and are copied byte-for-byte.
#[derive(Debug, Clone)]
pub struct IcddArchiveBuilder {
    index_rdf: Vec<u8>,
    payloads: BTreeMap<String, Vec<u8>>,
    linksets: BTreeMap<String, Vec<u8>>,
    ontology_resources: BTreeMap<String, Vec<u8>>,
    /// The typed index and base IRI, when made with `from_index`.
    typed: Option<(ContainerIndex, String)>,
}

impl IcddArchiveBuilder {
    /// Start an archive from a complete `Index.rdf` RDF/XML document.
    pub fn new(index_rdf: impl AsRef<[u8]>) -> Result<Self, IcddError> {
        let index_rdf = index_rdf.as_ref().to_vec();
        parse_index(&index_rdf)?;
        Ok(Self {
            index_rdf,
            payloads: BTreeMap::new(),
            linksets: BTreeMap::new(),
            ontology_resources: BTreeMap::new(),
            typed: None,
        })
    }

    /// Start an archive from a typed [`ContainerIndex`], encoded with
    /// [`ContainerIndex::to_rdf_xml`].
    ///
    /// The builder keeps the typed index and `base_iri`, and
    /// [`add_typed_linkset`](Self::add_typed_linkset) validates against them, so
    /// a linkset cannot be checked against a different index or base than the
    /// one written.
    ///
    /// # Errors
    ///
    /// Everything `to_rdf_xml` rejects, plus [`IcddError::NotConformant`] when
    /// `ct:conformanceIndicator` is not `ICDD-Part1-Container`: a container
    /// without it is not a Part 1 container, so it is not written.
    pub fn from_index(index: &ContainerIndex, base_iri: &str) -> Result<Self, IcddError> {
        match index.description.conformance_indicator.as_deref() {
            Some(vocab::CONFORMANCE_INDICATOR) => {}
            other => {
                return Err(IcddError::NotConformant(format!(
                    "ct:conformanceIndicator must be '{}', found {other:?}",
                    vocab::CONFORMANCE_INDICATOR
                )))
            }
        }
        let mut builder = Self::new(index.to_rdf_xml(base_iri)?)?;
        builder.typed = Some((index.clone(), base_iri.to_string()));
        Ok(builder)
    }

    /// Add a typed linkset below `Payload triples/`, encoded with
    /// [`LinkSet::to_rdf_xml`] and stored under `linkset.filename`.
    ///
    /// Unlike the free-standing `LinkSet::to_rdf_xml`, this takes no index or
    /// base: it uses the ones given to [`from_index`](Self::from_index).
    ///
    /// # Errors
    ///
    /// Everything `LinkSet::to_rdf_xml` rejects, plus
    /// [`IcddError::NotConformant`] when the builder was not made with
    /// `from_index` (it has no typed index to check against) or when the index
    /// declares no linkset file named `linkset.filename`.
    pub fn add_typed_linkset(self, linkset: &LinkSet) -> Result<Self, IcddError> {
        let Some((index, base_iri)) = &self.typed else {
            return Err(IcddError::NotConformant(
                "add_typed_linkset needs a builder made with from_index".into(),
            ));
        };
        if !index
            .linkset_files
            .iter()
            .any(|file| file.filename.as_deref() == Some(linkset.filename.as_str()))
        {
            return Err(IcddError::NotConformant(format!(
                "index declares no linkset file named {}",
                linkset.filename
            )));
        }
        let rdf = linkset.to_rdf_xml(index, base_iri)?;
        self.add_linkset(&linkset.filename, rdf)
    }

    /// Add an opaque document below `Payload documents/`.
    pub fn add_payload(
        mut self,
        relative_path: impl AsRef<str>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<Self, IcddError> {
        insert_unique(
            &mut self.payloads,
            normalize_relative_path(relative_path.as_ref())?,
            bytes.as_ref(),
            "payload document",
        )?;
        Ok(self)
    }

    /// Add and validate an RDF/XML linkset below `Payload triples/`.
    pub fn add_linkset(
        mut self,
        relative_path: impl AsRef<str>,
        rdf_xml: impl AsRef<[u8]>,
    ) -> Result<Self, IcddError> {
        let path = normalize_relative_path(relative_path.as_ref())?;
        parse_linkset(&path, rdf_xml.as_ref())?;
        insert_unique(&mut self.linksets, path, rdf_xml.as_ref(), "linkset")?;
        Ok(self)
    }

    /// Add an RDF/XML ontology resource below `Ontology resources/`.
    pub fn add_ontology_resource(
        mut self,
        relative_path: impl AsRef<str>,
        rdf_xml: impl AsRef<[u8]>,
    ) -> Result<Self, IcddError> {
        let path = normalize_relative_path(relative_path.as_ref())?;
        // Parse with the same maintained parser used for Index.rdf. Ontology
        // graphs need not contain a ContainerDescription, so use the raw parser.
        crate::rdfgraph::RdfGraph::parse(rdf_xml.as_ref())?;
        insert_unique(
            &mut self.ontology_resources,
            path,
            rdf_xml.as_ref(),
            "ontology resource",
        )?;
        Ok(self)
    }

    /// Validate declared files and emit deterministic ZIP bytes.
    pub fn finish(self) -> Result<Vec<u8>, IcddError> {
        self.validate_declared_entries()?;

        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .last_modified_time(DateTime::default())
            .unix_permissions(0o644);
        let directory_options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(DateTime::default())
            .unix_permissions(0o755);

        writer.start_file(crate::INDEX_PATH, options)?;
        writer.write_all(&self.index_rdf)?;
        write_directory(
            &mut writer,
            ONTOLOGY_DIR,
            directory_options,
            options,
            &self.ontology_resources,
        )?;
        write_directory(
            &mut writer,
            PAYLOAD_DOCS_DIR,
            directory_options,
            options,
            &self.payloads,
        )?;
        write_directory(
            &mut writer,
            PAYLOAD_TRIPLES_DIR,
            directory_options,
            options,
            &self.linksets,
        )?;
        Ok(writer.finish()?.into_inner())
    }

    fn validate_declared_entries(&self) -> Result<(), IcddError> {
        let index = parse_index(&self.index_rdf)?;
        let mut declared_payloads = BTreeSet::new();
        for document in &index.documents {
            if document.requested {
                continue;
            }
            if let DocumentKind::Internal { filename } = &document.kind {
                let filename = normalize_relative_path(filename)?;
                declared_payloads.insert(filename.clone());
                if !self.payloads.contains_key(&filename) {
                    return Err(IcddError::NotConformant(format!(
                        "Index.rdf declares missing payload document: {filename}"
                    )));
                }
            }
        }
        if let Some(filename) = self
            .payloads
            .keys()
            .find(|filename| !declared_payloads.contains(*filename))
        {
            return Err(IcddError::NotConformant(format!(
                "archive contains undeclared payload document: {filename}"
            )));
        }

        let mut declared_linksets = BTreeSet::new();
        for linkset in &index.linkset_files {
            let raw_filename = linkset.filename.as_deref().ok_or_else(|| {
                IcddError::NotConformant(format!(
                    "Index.rdf linkset {} has no filename",
                    linkset.id
                ))
            })?;
            let filename = normalize_relative_path(raw_filename)?;
            declared_linksets.insert(filename.clone());
            if !self.linksets.contains_key(&filename) {
                return Err(IcddError::NotConformant(format!(
                    "Index.rdf declares missing linkset: {filename}"
                )));
            }
        }
        if let Some(filename) = self
            .linksets
            .keys()
            .find(|filename| !declared_linksets.contains(*filename))
        {
            return Err(IcddError::NotConformant(format!(
                "archive contains undeclared linkset: {filename}"
            )));
        }
        Ok(())
    }
}

fn insert_unique(
    entries: &mut BTreeMap<String, Vec<u8>>,
    path: String,
    bytes: &[u8],
    kind: &str,
) -> Result<(), IcddError> {
    if entries
        .keys()
        .any(|existing| existing.eq_ignore_ascii_case(&path))
    {
        return Err(IcddError::NotConformant(format!(
            "duplicate case-folded {kind} path: {path}"
        )));
    }
    entries.insert(path, bytes.to_vec());
    Ok(())
}

fn normalize_relative_path(path: &str) -> Result<String, IcddError> {
    if path.contains('\\') || path.starts_with('/') || path.ends_with('/') || path.contains("//") {
        return Err(IcddError::NotConformant(format!(
            "archive path must be a canonical relative forward-slash path: {path}"
        )));
    }
    let mut safe = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(segment) => safe.push(segment),
            Component::CurDir
            | Component::ParentDir
            | Component::RootDir
            | Component::Prefix(_) => {
                return Err(IcddError::NotConformant(format!(
                    "unsafe or non-canonical archive path: {path}"
                )));
            }
        }
    }
    if safe.as_os_str().is_empty() {
        return Err(IcddError::NotConformant("empty archive path".into()));
    }
    Ok(path.to_string())
}

fn write_directory(
    writer: &mut ZipWriter<Cursor<Vec<u8>>>,
    directory: &str,
    directory_options: SimpleFileOptions,
    file_options: SimpleFileOptions,
    entries: &BTreeMap<String, Vec<u8>>,
) -> Result<(), IcddError> {
    writer.add_directory(format!("{directory}/"), directory_options)?;
    for (path, bytes) in entries {
        writer.start_file(format!("{directory}/{path}"), file_options)?;
        writer.write_all(bytes)?;
    }
    Ok(())
}
