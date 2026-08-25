//! The neutral in-memory IR for an ICDD container. Format-agnostic: no RDF
//! terms leak up (they're resolved in `index.rs`/`linkset.rs`), and no IFC
//! coupling leaks in (payload IFCs are yielded as bytes/paths for callers to
//! pass into an IFC pipeline). It follows the same codec/model boundary as CSET
//! and SMC.

use serde::Serialize;

/// A parsed ISO 21597-1 container: its manifest, documents, and linksets.
/// Payload bytes are read lazily from the still-open ZIP (see
/// [`super::IcddContainer`]); this struct is the decoded RDF graph only.
#[derive(Debug, Clone, Serialize)]
pub struct ContainerIndex {
    /// The single `ct:ContainerDescription` manifest.
    pub description: ContainerDescription,
    /// Every `ct:*Document` individual listed via `ct:containsDocument`.
    pub documents: Vec<Document>,
    /// Every `ct:Linkset` reference listed via `ct:containsLinkset` (the file
    /// references — the parsed `Link`s live in [`LinkSet`], keyed by filename).
    pub linkset_files: Vec<LinksetRef>,
}

/// The `ct:ContainerDescription` manifest object.
#[derive(Debug, Clone, Serialize)]
pub struct ContainerDescription {
    /// The RDF node id (`#id...`).
    pub id: String,
    /// `ct:conformanceIndicator` — must equal `"ICDD-Part1-Container"`.
    pub conformance_indicator: Option<String>,
    /// `ct:description`.
    pub description: Option<String>,
    /// `ct:creationDate` (xsd:dateTime, kept as the raw string).
    pub creation_date: Option<String>,
}

/// A `ct:Linkset` reference (the file, not its parsed content).
#[derive(Debug, Clone, Serialize)]
pub struct LinksetRef {
    /// RDF node id.
    pub id: String,
    /// `ct:filename` — path under `Payload triples/`.
    pub filename: Option<String>,
    /// `ct:name`.
    pub name: Option<String>,
}

/// A `ct:*Document` individual (Internal / External / Folder), with the
/// Secured/Encrypted mix-ins folded into flags.
#[derive(Debug, Clone, Serialize)]
pub struct Document {
    /// RDF node id (`#id...`) — link elements reference this via `ls:hasDocument`.
    pub id: String,
    /// Which concrete document class this is.
    pub kind: DocumentKind,
    /// `ct:name`.
    pub name: Option<String>,
    /// `ct:description`.
    pub description: Option<String>,
    /// `ct:filetype` — e.g. `"ifc"`, `"pdf"`, `"xls"`, `"shp"`.
    pub filetype: Option<String>,
    /// `ct:format` — IANA media type, e.g. `application/x-extension-ifc`.
    pub format: Option<String>,
    /// `(algorithm, value)` if this is a `ct:SecuredDocument`.
    pub checksum: Option<Checksum>,
    /// True if this is (also) a `ct:EncryptedDocument`.
    pub encrypted: bool,
    /// `ct:requested` — a "slot" for a not-yet-delivered document.
    pub requested: bool,
}

/// The concrete document class and its location payload.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentKind {
    /// `ct:InternalDocument` — a file inside the container.
    Internal {
        /// `ct:filename` — path under `Payload documents/`, `/`-separated.
        filename: String,
    },
    /// `ct:ExternalDocument` — a document outside the container.
    External {
        /// `ct:url` (anyURI).
        url: String,
    },
    /// `ct:FolderDocument` — a multi-file document in one folder.
    Folder {
        /// `ct:foldername` — path under `Payload documents/`.
        foldername: String,
    },
}

/// A `ct:SecuredDocument` checksum.
#[derive(Debug, Clone, Serialize)]
pub struct Checksum {
    /// `ct:checksumAlgorithm`.
    pub algorithm: String,
    /// `ct:checksum`.
    pub value: String,
}

impl Document {
    /// True if this document is an IFC payload (by filetype / media type /
    /// filename extension) — a payload a caller may pass into an IFC pipeline.
    pub fn is_ifc(&self) -> bool {
        let ft = self.filetype.as_deref().unwrap_or("").to_ascii_lowercase();
        if ft == "ifc" || ft.starts_with("ifc-") || ft.starts_with("ifc ") {
            return true;
        }
        if let Some(fmt) = &self.format {
            if fmt.to_ascii_lowercase().contains("ifc") {
                return true;
            }
        }
        self.internal_path()
            .map(|p| p.to_ascii_lowercase().ends_with(".ifc"))
            .unwrap_or(false)
    }

    /// The in-container path (under `Payload documents/`) for an internal
    /// document; `None` for external/folder documents.
    pub fn internal_path(&self) -> Option<&str> {
        match &self.kind {
            DocumentKind::Internal { filename } => Some(filename),
            _ => None,
        }
    }
}

/// A parsed link dataset (one `Payload triples/*.rdf` file).
#[derive(Debug, Clone, Serialize)]
pub struct LinkSet {
    /// The linkset filename (relative to `Payload triples/`).
    pub filename: String,
    /// The `ls:Link` individuals it defines.
    pub links: Vec<Link>,
}

/// An `ls:Link` — a set of ≥1 `ls:LinkElement` relating documents/elements.
#[derive(Debug, Clone, Serialize)]
pub struct Link {
    /// RDF node id.
    pub id: String,
    /// Whether the link is directed (has from/to elements) or a plain set.
    pub directed: bool,
    /// All link elements (undirected view).
    pub elements: Vec<LinkElement>,
    /// Node ids that are `ls:hasFromLinkElement` (subset of `elements`), if directed.
    pub from: Vec<String>,
    /// Node ids that are `ls:hasToLinkElement` (subset of `elements`), if directed.
    pub to: Vec<String>,
}

/// An `ls:LinkElement` — a reference to a document (+ optional identifier into it).
#[derive(Debug, Clone, Serialize)]
pub struct LinkElement {
    /// RDF node id.
    pub id: String,
    /// `ls:hasDocument` → the referenced [`Document::id`].
    pub document_id: Option<String>,
    /// `ls:hasIdentifier` → how to locate an element WITHIN that document.
    pub identifier: Option<ElementIdentifier>,
}

/// An `ls:Identifier` (String / URI / Query based).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ElementIdentifier {
    /// `ls:StringBasedIdentifier` — e.g. an IFC GUID or a spreadsheet row id.
    String {
        /// `ls:identifier` — the actual id string.
        value: String,
        /// `ls:identifierField` — which field(s) hold the id (optional).
        field: Option<String>,
    },
    /// `ls:URIBasedIdentifier`.
    Uri {
        /// `ls:uri`.
        uri: String,
    },
    /// `ls:QueryBasedIdentifier`.
    Query {
        /// `ls:queryLanguage`.
        language: Option<String>,
        /// `ls:queryExpression`.
        expression: Option<String>,
    },
}
