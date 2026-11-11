//! In-place photo metadata backed by XMP.
//!
//! Browsing never depends on external tools: sidecars and XMP embedded in
//! JPEGs are parsed natively. Writing is delegated to ExifTool so every
//! property Lumio does not own (Darktable history, masks, private namespaces
//! and unknown tags) survives. Writes are staged, verified and only then
//! committed atomically.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde::{Deserialize, Serialize};

use crate::error::{LumioError, Result};
use crate::path_utils::modified_ms;

pub const LUMIO_NAMESPACE: &str = "https://lumio.app/ns/1.0/";

const RDF_NAMESPACE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";
const XMP_JPEG_HEADER: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";

const KEY_RATING: &str = "{http://ns.adobe.com/xap/1.0/}Rating";
const KEY_LABEL: &str = "{http://ns.adobe.com/xap/1.0/}Label";
const KEY_SUBJECT: &str = "{http://purl.org/dc/elements/1.1/}subject";
const KEY_HIERARCHICAL: &str = "{http://ns.adobe.com/lightroom/1.0/}hierarchicalSubject";
const KEY_CULL: &str = "{https://lumio.app/ns/1.0/}Cull";
const KEY_FAVORITE: &str = "{https://lumio.app/ns/1.0/}Favorite";

const WRITABLE_KEYS: &[&str] = &[
    KEY_RATING,
    KEY_LABEL,
    KEY_SUBJECT,
    KEY_HIERARCHICAL,
    KEY_CULL,
    KEY_FAVORITE,
];

/// Empty packet used when a photo has no sidecar yet and metadata is the first
/// thing Lumio writes. ExifTool adds only the requested properties.
const EMPTY_XMP_PACKET: &str = "\u{feff}<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n\
 <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
  <rdf:Description rdf:about=\"\"\n\
    xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"\n\
    xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\n\
    xmlns:lr=\"http://ns.adobe.com/lightroom/1.0/\"\n\
    xmlns:lumio=\"https://lumio.app/ns/1.0/\">\n\
  </rdf:Description>\n\
 </rdf:RDF>\n\
</x:xmpmeta>\n\
<?xpacket end=\"w\"?>\n";

/// ExifTool configuration that teaches the writer about the private `lumio`
/// namespace. It is generated in the application cache at startup.
pub const EXIFTOOL_CONFIG: &str = r#"%Image::ExifTool::UserDefined = (
    'Image::ExifTool::XMP::Main' => {
        lumio => {
            SubDirectory => { TagTable => 'Image::ExifTool::UserDefined::lumio' },
        },
    },
);
%Image::ExifTool::UserDefined::lumio = (
    GROUPS => { 0 => 'XMP', 1 => 'XMP-lumio', 2 => 'Image' },
    NAMESPACE => { 'lumio' => 'https://lumio.app/ns/1.0/' },
    Cull => { Writable => 'string' },
    Favorite => { Writable => 'boolean' },
);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorLabel {
    Red,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl ColorLabel {
    pub fn as_xmp(self) -> &'static str {
        match self {
            Self::Red => "Red",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
        }
    }

    pub fn from_xmp(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "red" => Some(Self::Red),
            "yellow" => Some(Self::Yellow),
            "green" => Some(Self::Green),
            "blue" => Some(Self::Blue),
            "purple" => Some(Self::Purple),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CullFlag {
    Pick,
    Reject,
}

impl CullFlag {
    fn as_xmp(self) -> &'static str {
        match self {
            Self::Pick => "pick",
            Self::Reject => "reject",
        }
    }

    fn from_xmp(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pick" => Some(Self::Pick),
            "reject" => Some(Self::Reject),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MetadataSource {
    Sidecar,
    Embedded,
    #[default]
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataIssue {
    pub code: String,
    pub message: String,
}

impl MetadataIssue {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PhotoMetadata {
    /// 0-5 stars. XMP `-1` (rejected) is read as zero stars plus a reject flag.
    pub rating: u8,
    pub label: Option<ColorLabel>,
    pub cull: Option<CullFlag>,
    pub favorite: bool,
    pub keywords: Vec<String>,
    pub hierarchical_keywords: Vec<String>,
    pub source: MetadataSource,
    /// BLAKE3 fingerprint of the metadata packet this value was read from.
    /// Empty when the photo has no XMP yet.
    pub revision: String,
    pub issues: Vec<MetadataIssue>,
}

impl PhotoMetadata {
    pub fn has_edit(&self) -> bool {
        self.rating > 0
            || self.label.is_some()
            || self.cull.is_some()
            || self.favorite
            || !self.keywords.is_empty()
            || !self.hierarchical_keywords.is_empty()
    }
}

/// A partial update. Missing fields are left alone; `clear_*` removes a value.
/// An empty keyword list clears the corresponding XMP bag.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataUpdate {
    pub rating: Option<u8>,
    pub label: Option<ColorLabel>,
    #[serde(default)]
    pub clear_label: bool,
    pub cull: Option<CullFlag>,
    #[serde(default)]
    pub clear_cull: bool,
    pub favorite: Option<bool>,
    pub keywords: Option<Vec<String>>,
    pub hierarchical_keywords: Option<Vec<String>>,
}

impl MetadataUpdate {
    pub fn is_empty(&self) -> bool {
        self.rating.is_none()
            && self.label.is_none()
            && !self.clear_label
            && self.cull.is_none()
            && !self.clear_cull
            && self.favorite.is_none()
            && self.keywords.is_none()
            && self.hierarchical_keywords.is_none()
    }

    fn normalized(mut self) -> Self {
        self.rating = self.rating.map(|rating| rating.min(5));
        self.keywords = self.keywords.map(normalize_keywords);
        self.hierarchical_keywords = self.hierarchical_keywords.map(normalize_keywords);
        self
    }

    fn touches(&self, key: &str) -> bool {
        match key {
            KEY_RATING => self.rating.is_some(),
            KEY_LABEL => self.label.is_some() || self.clear_label,
            KEY_CULL => self.cull.is_some() || self.clear_cull,
            KEY_FAVORITE => self.favorite.is_some(),
            KEY_SUBJECT => self.keywords.is_some(),
            KEY_HIERARCHICAL => self.hierarchical_keywords.is_some(),
            _ => false,
        }
    }
}

fn normalize_keywords(values: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.to_ascii_lowercase()) {
            normalized.push(value);
        }
    }
    normalized
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataWriteResult {
    pub metadata: PhotoMetadata,
    pub target: PathBuf,
    pub created_sidecar: bool,
    pub file_size: u64,
    pub modified_ms: u64,
}

/// Generate or refresh the ExifTool user-defined tag configuration.
pub fn write_exiftool_config(path: &Path) -> Result<()> {
    if let Ok(existing) = fs::read_to_string(path) {
        if existing == EXIFTOOL_CONFIG {
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| LumioError::io(parent, error))?;
    }
    fs::write(path, EXIFTOOL_CONFIG).map_err(|error| LumioError::io(path, error))
}

// --------------------------------------------------------------------------
// XMP parsing
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct RawNode {
    namespace: String,
    local: String,
    attrs: Vec<(String, String)>,
    children: Vec<RawNode>,
    text: String,
}

impl RawNode {
    fn key(&self) -> String {
        if self.namespace.is_empty() {
            self.local.clone()
        } else {
            format!("{{{}}}{}", self.namespace, self.local)
        }
    }

    fn is(&self, namespace: &str, local: &str) -> bool {
        self.namespace == namespace && self.local == local
    }

    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone)]
struct XmpProperty {
    key: String,
    canonical: String,
    text: Option<String>,
    items: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
struct XmpDocument {
    properties: Vec<XmpProperty>,
}

impl XmpDocument {
    fn simple(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|property| property.key == key)
            .and_then(|property| property.text.as_deref())
    }

    fn list(&self, key: &str) -> Vec<String> {
        self.properties
            .iter()
            .find(|property| property.key == key)
            .and_then(|property| property.items.clone())
            .unwrap_or_default()
    }

    /// Snapshot of every property Lumio does not write, keyed by expanded name
    /// and canonicalised so XML formatting and Description layout can change.
    fn preservation_snapshot(&self) -> BTreeMap<String, Vec<String>> {
        let mut snapshot: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for property in &self.properties {
            if WRITABLE_KEYS.contains(&property.key.as_str()) {
                continue;
            }
            snapshot
                .entry(property.key.clone())
                .or_default()
                .push(property.canonical.clone());
        }
        for values in snapshot.values_mut() {
            values.sort();
        }
        snapshot
    }
}

fn local_name(name: &str) -> (String, String) {
    match name.split_once(':') {
        Some((prefix, local)) => (prefix.to_string(), local.to_string()),
        None => (String::new(), name.to_string()),
    }
}

fn resolve_name(scope: &HashMap<String, String>, name: &str) -> (String, String) {
    let (prefix, local) = local_name(name);
    let namespace = scope.get(&prefix).cloned().unwrap_or(prefix);
    (namespace, local)
}

fn parse_xmp_bytes(bytes: &[u8]) -> std::result::Result<XmpDocument, String> {
    let text =
        std::str::from_utf8(bytes).map_err(|error| format!("XMP is not valid UTF-8: {error}"))?;
    let text = text.trim_start_matches('\u{feff}');
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(false);

    let mut scopes: Vec<HashMap<String, String>> = vec![HashMap::from([(
        "xml".to_string(),
        XML_NAMESPACE.to_string(),
    )])];
    let mut stack: Vec<RawNode> = Vec::new();
    let mut roots: Vec<RawNode> = Vec::new();

    loop {
        let event = reader
            .read_event()
            .map_err(|error| format!("XMP contains malformed XML: {error}"))?;
        match event {
            Event::Start(start) => {
                let (scope, node) = start_node(start, &scopes)?;
                scopes.push(scope);
                stack.push(node);
            }
            Event::Empty(start) => {
                let (_scope, node) = start_node(start, &scopes)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    roots.push(node);
                }
            }
            Event::End(_) => {
                let Some(node) = stack.pop() else {
                    continue;
                };
                if scopes.len() > 1 {
                    scopes.pop();
                }
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                } else {
                    roots.push(node);
                }
            }
            Event::Text(value) => {
                if let Some(node) = stack.last_mut() {
                    // `xml10_content` only normalises line endings. XMP values
                    // routinely contain `&amp;`, `&quot;` and numeric character
                    // references, so decode predefined entities as well.
                    let content = value.xml10_content();
                    let decoded = quick_xml::escape::unescape(&content)
                        .map_err(|error| format!("XMP contains an invalid entity: {error}"))?;
                    node.text.push_str(&decoded);
                }
            }
            Event::CData(value) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(&value.xml10_content());
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(node) = stack.last_mut() {
                    if let Some(character) = reference.resolve_char_ref().map_err(|error| {
                        format!("XMP contains an invalid character reference: {error}")
                    })? {
                        node.text.push(character);
                    } else if let Some(value) =
                        quick_xml::escape::resolve_predefined_entity(&reference)
                    {
                        node.text.push_str(value);
                    } else {
                        return Err(format!(
                            "XMP contains an unknown entity reference: &{};",
                            &*reference
                        ));
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    extract_document(&roots)
}

fn start_node(
    start: BytesStart<'_>,
    scopes: &[HashMap<String, String>],
) -> std::result::Result<(HashMap<String, String>, RawNode), String> {
    let mut scope = scopes
        .last()
        .cloned()
        .unwrap_or_else(|| HashMap::from([("xml".to_string(), XML_NAMESPACE.to_string())]));
    let mut pending_attrs: Vec<(String, String)> = Vec::new();
    for attribute in start.attributes() {
        let attribute =
            attribute.map_err(|error| format!("XMP has a malformed attribute: {error}"))?;
        let key = attribute.key.as_ref().to_string();
        let value = attribute
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|error| format!("XMP has a malformed attribute value: {error}"))?
            .into_owned();
        if key == "xmlns" {
            scope.insert(String::new(), value);
        } else if let Some(prefix) = key.strip_prefix("xmlns:") {
            scope.insert(prefix.to_string(), value);
        } else {
            pending_attrs.push((key, value));
        }
    }

    let (namespace, local) = resolve_name(&scope, start.name().as_ref());
    let attrs = pending_attrs
        .into_iter()
        .map(|(name, value)| {
            let (prefix, local) = local_name(&name);
            let namespace = scope.get(&prefix).cloned().unwrap_or(prefix);
            let key = if namespace.is_empty() {
                local
            } else {
                format!("{{{namespace}}}{local}")
            };
            (key, value)
        })
        .collect();
    Ok((
        scope,
        RawNode {
            namespace,
            local,
            attrs,
            children: Vec::new(),
            text: String::new(),
        },
    ))
}

fn find_rdf(nodes: &[RawNode]) -> Option<&RawNode> {
    for node in nodes {
        if node.is(RDF_NAMESPACE, "RDF") {
            return Some(node);
        }
        if let Some(found) = find_rdf(&node.children) {
            return Some(found);
        }
    }
    None
}

fn extract_document(roots: &[RawNode]) -> std::result::Result<XmpDocument, String> {
    let mut document = XmpDocument::default();
    let mut descriptions: Vec<&RawNode> = Vec::new();
    if let Some(rdf) = find_rdf(roots) {
        for child in &rdf.children {
            if child.is(RDF_NAMESPACE, "Description") {
                descriptions.push(child);
            }
        }
    } else {
        // Tolerate a bare rdf:Description packet.
        fn collect_descriptions<'a>(nodes: &'a [RawNode], out: &mut Vec<&'a RawNode>) {
            for node in nodes {
                if node.is(RDF_NAMESPACE, "Description") {
                    out.push(node);
                }
                collect_descriptions(&node.children, out);
            }
        }
        collect_descriptions(roots, &mut descriptions);
    }
    if descriptions.is_empty() {
        return Err("No RDF metadata description was found in the XMP.".into());
    }

    for description in descriptions {
        add_description(description, &mut document);
    }
    Ok(document)
}

fn add_description(description: &RawNode, document: &mut XmpDocument) {
    for (key, value) in &description.attrs {
        if is_structural_attr(key) {
            continue;
        }
        document.properties.push(XmpProperty {
            key: key.clone(),
            canonical: value.clone(),
            text: Some(value.clone()),
            items: None,
        });
    }
    for child in &description.children {
        if child.is(RDF_NAMESPACE, "Description") {
            add_description(child, document);
            continue;
        }
        add_property(child, document);
    }
}

fn add_property(node: &RawNode, document: &mut XmpDocument) {
    document.properties.push(XmpProperty {
        key: node.key(),
        canonical: canonical_value(node),
        text: leaf_text(node),
        items: list_items(node),
    });
}

fn is_structural_attr(key: &str) -> bool {
    matches!(
        key,
        "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}about"
            | "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}parseType"
            | "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}nodeID"
            | "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}ID"
            | "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}bagID"
            | "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}resource"
    )
}

fn content_attrs(node: &RawNode) -> Vec<(String, String)> {
    node.attrs
        .iter()
        .filter(|(key, _)| {
            !is_structural_attr(key) && key != "{http://www.w3.org/1999/02/22-rdf-syntax-ns#}li"
        })
        .cloned()
        .collect()
}

fn normalize_text(value: &str) -> String {
    value.trim().to_string()
}

fn canonical_value(node: &RawNode) -> String {
    if let Some(resource) = node.attr(&format!("{{{RDF_NAMESPACE}}}resource")) {
        return format!("resource({resource})");
    }
    if node.is(RDF_NAMESPACE, "Description") {
        return canonical_struct(node);
    }
    if node.attr(&format!("{{{RDF_NAMESPACE}}}parseType")) == Some("Resource") {
        return canonical_struct(node);
    }

    if node.children.len() == 1 {
        let child = &node.children[0];
        if child.is(RDF_NAMESPACE, "Bag") {
            return canonical_list("bag", child, true);
        }
        if child.is(RDF_NAMESPACE, "Seq") {
            return canonical_list("seq", child, false);
        }
        if child.is(RDF_NAMESPACE, "Alt") {
            return canonical_list("alt", child, false);
        }
        if child.is(RDF_NAMESPACE, "Description") {
            return canonical_struct(child);
        }
    }

    if node.children.is_empty() {
        let attrs = content_attrs(node);
        let text = normalize_text(&node.text);
        if attrs.is_empty() {
            return text;
        }
        let mut attrs: Vec<String> = attrs
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        attrs.sort();
        return format!("{}|{}", attrs.join(","), text);
    }

    canonical_struct(node)
}

fn canonical_list(kind: &str, node: &RawNode, sort: bool) -> String {
    let mut items: Vec<String> = node
        .children
        .iter()
        .filter(|child| child.is(RDF_NAMESPACE, "li"))
        .map(canonical_li)
        .collect();
    if sort {
        items.sort();
    }
    format!("{kind}[{}]", items.join("|"))
}

fn canonical_li(node: &RawNode) -> String {
    if node.children.is_empty()
        && node
            .attr(&format!("{{{RDF_NAMESPACE}}}parseType"))
            .is_none()
    {
        let attrs = content_attrs(node);
        let text = normalize_text(&node.text);
        if attrs.is_empty() {
            return text;
        }
        let mut attrs: Vec<String> = attrs
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect();
        attrs.sort();
        return format!("{}|{}", attrs.join(","), text);
    }
    canonical_struct(node)
}

/// Canonical representation with property order removed. Text leaves are
/// trimmed because ExifTool is free to re-indent XML without changing values.
fn canonical_struct(node: &RawNode) -> String {
    let mut entries: Vec<(String, String)> = Vec::new();
    for (key, value) in content_attrs(node) {
        entries.push((key, value));
    }
    for child in &node.children {
        if child.is(RDF_NAMESPACE, "Description") {
            let inner = canonical_struct(child);
            entries.push(("{description}".into(), inner));
            continue;
        }
        entries.push((child.key(), canonical_value(child)));
    }
    let text = normalize_text(&node.text);
    if !text.is_empty() {
        entries.push(("#text".into(), text));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let body: Vec<String> = entries
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();
    format!("{{{}}}", body.join(";"))
}

fn leaf_text(node: &RawNode) -> Option<String> {
    if !node.children.is_empty() {
        return None;
    }
    if node.attr(&format!("{{{RDF_NAMESPACE}}}resource")).is_some() {
        return None;
    }
    Some(normalize_text(&node.text))
}

/// If the property holds an RDF Bag/Seq/Alt of simple text values, return the
/// item strings in document order. Used by keywords and hierarchical subjects.
fn list_items(node: &RawNode) -> Option<Vec<String>> {
    let container = node.children.first()?;
    if !matches!(
        (container.namespace.as_str(), container.local.as_str()),
        (RDF_NAMESPACE, "Bag") | (RDF_NAMESPACE, "Seq") | (RDF_NAMESPACE, "Alt")
    ) {
        return None;
    }
    let mut items = Vec::new();
    for child in &container.children {
        if !child.is(RDF_NAMESPACE, "li") || !child.children.is_empty() {
            return None;
        }
        items.push(normalize_text(&child.text));
    }
    Some(items)
}

// --------------------------------------------------------------------------
// Reading
// --------------------------------------------------------------------------

fn xmp_revision(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn sidecar_path_for(primary: &Path) -> PathBuf {
    let file_name = primary
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "photo".into());
    primary.with_file_name(format!("{file_name}.xmp"))
}

fn is_jpeg(path: &Path) -> bool {
    path.extension()
        .map(|value| {
            value.to_string_lossy().eq_ignore_ascii_case("jpg")
                || value.to_string_lossy().eq_ignore_ascii_case("jpeg")
        })
        .unwrap_or(false)
}

fn effective_sidecar(primary: &Path, sidecar: Option<&Path>) -> Option<PathBuf> {
    if let Some(sidecar) = sidecar {
        return Some(sidecar.to_path_buf());
    }
    let derived = sidecar_path_for(primary);
    if derived.is_file() {
        return Some(derived);
    }
    // Scans also accept `<stem>.xmp` (for example `IMG_0001.xmp` beside
    // `IMG_0001.CR2`). Honour it here so a sidecar that appeared while the
    // folder was open becomes authoritative instead of being ignored.
    let stem_sidecar = primary.with_extension("xmp");
    if stem_sidecar != derived && stem_sidecar.is_file() {
        return Some(stem_sidecar);
    }
    None
}

/// Read metadata for browsing. Never fails hard: unreadable or malformed XMP is
/// reported through `issues` so the rest of the photo remains browsable.
pub fn read_photo_metadata(primary: &Path, sidecar: Option<&Path>) -> PhotoMetadata {
    if let Some(sidecar) = effective_sidecar(primary, sidecar) {
        match fs::read(&sidecar) {
            Ok(bytes) => {
                let revision = xmp_revision(&bytes);
                return metadata_from_bytes(&bytes, MetadataSource::Sidecar, revision);
            }
            Err(error) => {
                let mut metadata = PhotoMetadata::default();
                metadata.issues.push(MetadataIssue::new(
                    "sidecarUnreadable",
                    format!("The XMP sidecar could not be read: {error}"),
                ));
                return metadata;
            }
        }
    }

    if is_jpeg(primary) {
        match read_jpeg_xmp(primary) {
            Ok(Some(packet)) => {
                let revision = xmp_revision(&packet);
                return metadata_from_bytes(&packet, MetadataSource::Embedded, revision);
            }
            Ok(None) => return PhotoMetadata::default(),
            Err(error) => {
                let mut metadata = PhotoMetadata::default();
                metadata.issues.push(MetadataIssue::new(
                    "embeddedUnreadable",
                    format!("Embedded XMP could not be read: {error}"),
                ));
                return metadata;
            }
        }
    }

    PhotoMetadata::default()
}

fn metadata_from_bytes(bytes: &[u8], source: MetadataSource, revision: String) -> PhotoMetadata {
    let mut metadata = PhotoMetadata {
        source,
        revision,
        ..PhotoMetadata::default()
    };
    let document = match parse_xmp_bytes(bytes) {
        Ok(document) => document,
        Err(message) => {
            metadata
                .issues
                .push(MetadataIssue::new("invalidXmp", message));
            return metadata;
        }
    };
    metadata_from_document(&document, metadata)
}

fn metadata_from_document(document: &XmpDocument, mut metadata: PhotoMetadata) -> PhotoMetadata {
    if let Some(raw) = document.simple(KEY_RATING) {
        match raw.trim().parse::<i32>() {
            Ok(-1) => {
                // Adobe's rejected convention predates the independent Lumio
                // cull flag. Preserve the meaning on read and migrate it to the
                // independent flag on the next write.
                if document.simple(KEY_CULL).is_none() {
                    metadata.cull = Some(CullFlag::Reject);
                }
            }
            Ok(value) if (0..=5).contains(&value) => metadata.rating = value as u8,
            Ok(value) => {
                metadata.rating = value.clamp(0, 5) as u8;
                metadata.issues.push(MetadataIssue::new(
                    "invalidRating",
                    format!("The rating {value} is outside the supported 0-5 range."),
                ));
            }
            Err(_) => metadata.issues.push(MetadataIssue::new(
                "invalidRating",
                format!("The rating value “{raw}” is not a number."),
            )),
        }
    }

    if let Some(raw) = document.simple(KEY_LABEL) {
        if !raw.trim().is_empty() {
            match ColorLabel::from_xmp(raw) {
                Some(label) => metadata.label = Some(label),
                None => metadata.issues.push(MetadataIssue::new(
                    "unknownLabel",
                    format!("The XMP label “{raw}” is not one of Lumio's five colors."),
                )),
            }
        }
    }

    if let Some(raw) = document.simple(KEY_CULL) {
        match CullFlag::from_xmp(raw) {
            Some(cull) => metadata.cull = Some(cull),
            None if !raw.trim().is_empty() => metadata.issues.push(MetadataIssue::new(
                "unknownCull",
                format!("The Lumio cull flag “{raw}” is not recognised."),
            )),
            None => {}
        }
    }

    if let Some(raw) = document.simple(KEY_FAVORITE) {
        metadata.favorite = matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes" | "on"
        );
    }

    metadata.keywords = document.list(KEY_SUBJECT);
    metadata.hierarchical_keywords = document.list(KEY_HIERARCHICAL);
    metadata
}

// --------------------------------------------------------------------------
// JPEG segment handling
// --------------------------------------------------------------------------

struct JpegSegment {
    marker: u8,
    /// Marker byte, two-byte length and payload.
    raw: Vec<u8>,
    /// Payload after the length field.
    payload: Vec<u8>,
}

/// Split JPEG headers into segments while retaining the complete bytes of each
/// segment. Scan data after SOS is returned separately as `tail`.
fn jpeg_segments(bytes: &[u8]) -> io::Result<(Vec<JpegSegment>, Vec<u8>)> {
    if bytes.len() < 2 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The file is not a JPEG image.",
        ));
    }
    let mut segments = Vec::new();
    let mut position = 2;
    loop {
        if position >= bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The JPEG ended before a scan was found.",
            ));
        }
        if bytes[position] != 0xff {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The JPEG contains an invalid marker.",
            ));
        }
        while position < bytes.len() && bytes[position] == 0xff {
            position += 1;
        }
        let Some(&marker) = bytes.get(position) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The JPEG ended inside a marker.",
            ));
        };
        position += 1;
        if marker == 0xd9 {
            return Ok((segments, bytes[position..].to_vec()));
        }
        if marker == 0xda {
            // Entropy-coded scan data follows and may contain stuffed 0xFF
            // bytes. Everything from here to EOI belongs to the tail.
            return Ok((segments, bytes[position - 2..].to_vec()));
        }
        if (0xd0..=0xd7).contains(&marker) || marker == 0x01 {
            let raw = bytes[position - 2..position].to_vec();
            segments.push(JpegSegment {
                marker,
                raw,
                payload: Vec::new(),
            });
            continue;
        }
        if position + 2 > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The JPEG segment length is missing.",
            ));
        }
        let length = u16::from_be_bytes([bytes[position], bytes[position + 1]]) as usize;
        if length < 2 || position + length > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The JPEG segment length is invalid.",
            ));
        }
        let start = position - 2;
        let end = position + length;
        let raw = bytes[start..end].to_vec();
        let payload = bytes[position + 2..end].to_vec();
        segments.push(JpegSegment {
            marker,
            raw,
            payload,
        });
        position = end;
    }
}

fn read_jpeg_xmp(path: &Path) -> io::Result<Option<Vec<u8>>> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut header = [0u8; 2];
    reader.read_exact(&mut header)?;
    if header != [0xff, 0xd8] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The file is not a JPEG image.",
        ));
    }
    loop {
        let mut byte = [0u8; 1];
        if reader.read_exact(&mut byte).is_err() {
            return Ok(None);
        }
        if byte[0] != 0xff {
            return Ok(None);
        }
        loop {
            if reader.read_exact(&mut byte).is_err() {
                return Ok(None);
            }
            if byte[0] != 0xff {
                break;
            }
        }
        let marker = byte[0];
        if marker == 0xd9 || marker == 0xda {
            return Ok(None);
        }
        if (0xd0..=0xd7).contains(&marker) || marker == 0x01 {
            continue;
        }
        let mut length_bytes = [0u8; 2];
        if reader.read_exact(&mut length_bytes).is_err() {
            return Ok(None);
        }
        let length = u16::from_be_bytes(length_bytes) as usize;
        if length < 2 {
            return Ok(None);
        }
        let payload_length = length - 2;
        if marker == 0xe1 {
            let mut payload = vec![0u8; payload_length];
            reader.read_exact(&mut payload)?;
            if let Some(packet) = payload.strip_prefix(XMP_JPEG_HEADER) {
                return Ok(Some(packet.to_vec()));
            }
        } else {
            let mut remaining = payload_length as u64;
            let mut buffer = [0u8; 8192];
            while remaining > 0 {
                let take = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(0);
                reader.read_exact(&mut buffer[..take])?;
                remaining -= take as u64;
            }
        }
    }
}

/// All JPEG bytes except the XMP APP1 segments. Comparing this before and after
/// a staged write proves ExifTool did not touch EXIF, ICC profiles, thumbnails
/// or the compressed scan data.
fn jpeg_bytes_without_xmp(bytes: &[u8]) -> io::Result<Vec<u8>> {
    if bytes.len() < 2 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "The file is not a JPEG image.",
        ));
    }
    let (segments, tail) = jpeg_segments(bytes)?;
    let mut output = vec![0xff, 0xd8];
    for segment in segments {
        if segment.marker == 0xe1 && segment.payload.starts_with(XMP_JPEG_HEADER) {
            continue;
        }
        output.extend_from_slice(&segment.raw);
    }
    output.extend_from_slice(&tail);
    Ok(output)
}

// --------------------------------------------------------------------------
// Writing
// --------------------------------------------------------------------------

enum WriteTarget {
    Sidecar { path: PathBuf, existed: bool },
    EmbeddedJpeg { path: PathBuf },
}

impl WriteTarget {
    fn path(&self) -> &Path {
        match self {
            Self::Sidecar { path, .. } => path,
            Self::EmbeddedJpeg { path } => path,
        }
    }
}

fn resolve_write_target(primary: &Path, sidecar: Option<&Path>) -> WriteTarget {
    if let Some(sidecar) = effective_sidecar(primary, sidecar) {
        let existed = sidecar.is_file();
        return WriteTarget::Sidecar {
            path: sidecar,
            existed,
        };
    }
    if is_jpeg(primary) {
        return WriteTarget::EmbeddedJpeg {
            path: primary.to_path_buf(),
        };
    }
    WriteTarget::Sidecar {
        path: sidecar_path_for(primary),
        existed: false,
    }
}

fn temp_path_near(target: &Path, extension: &str) -> PathBuf {
    let unique = uuid::Uuid::new_v4();
    target.with_file_name(format!(".lumio-{unique}.{extension}"))
}

fn run_exiftool(
    exiftool: &Path,
    config: Option<&Path>,
    args: &[String],
) -> std::result::Result<(), String> {
    let mut command = Command::new(exiftool);
    if let Some(config) = config {
        command.arg("-config").arg(config);
    }
    let output = command
        .args(args)
        .output()
        .map_err(|error| format!("ExifTool could not be started: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if !stderr.trim().is_empty() {
        stderr.trim()
    } else {
        stdout.trim()
    };
    Err(format!("ExifTool could not write metadata: {detail}"))
}

fn exiftool_args(target: &WriteTarget, update: &MetadataUpdate) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if matches!(target, WriteTarget::Sidecar { .. }) {
        // Sidecars are written into a staged copy, so this only affects the
        // temporary file. Embedded JPEGs use `-o` instead: combining that with
        // `-overwrite_original` makes ExifTool move the source out of the way.
        args.push("-overwrite_original".into());
    }
    args.push("-charset".into());
    args.push("iptc=UTF8".into());

    if let Some(rating) = update.rating {
        args.push(format!("-XMP:Rating={rating}"));
    }
    if update.clear_label {
        args.push("-XMP:Label=".into());
    } else if let Some(label) = update.label {
        args.push(format!("-XMP:Label={}", label.as_xmp()));
    }
    if update.clear_cull {
        args.push("-XMP-lumio:Cull=".into());
    } else if let Some(cull) = update.cull {
        args.push(format!("-XMP-lumio:Cull={}", cull.as_xmp()));
    }
    if let Some(favorite) = update.favorite {
        if favorite {
            args.push("-XMP-lumio:Favorite=true".into());
        } else {
            args.push("-XMP-lumio:Favorite=".into());
        }
    }
    if let Some(keywords) = &update.keywords {
        args.push("-XMP-dc:Subject=".into());
        for keyword in keywords {
            args.push(format!("-XMP-dc:Subject={keyword}"));
        }
    }
    if let Some(keywords) = &update.hierarchical_keywords {
        args.push("-XMP-lr:HierarchicalSubject=".into());
        for keyword in keywords {
            args.push(format!("-XMP-lr:HierarchicalSubject={keyword}"));
        }
    }
    args
}

fn has_fatal_metadata_issues(metadata: &PhotoMetadata) -> bool {
    metadata.issues.iter().any(|issue| {
        matches!(
            issue.code.as_str(),
            "invalidXmp" | "sidecarUnreadable" | "embeddedUnreadable"
        )
    })
}

fn metadata_after_read(target: &WriteTarget) -> PhotoMetadata {
    match target {
        WriteTarget::Sidecar { path, .. } => {
            if !path.exists() {
                return PhotoMetadata::default();
            }
            match fs::read(path) {
                Ok(bytes) => {
                    metadata_from_bytes(&bytes, MetadataSource::Sidecar, xmp_revision(&bytes))
                }
                Err(error) => {
                    let mut metadata = PhotoMetadata::default();
                    metadata.issues.push(MetadataIssue::new(
                        "sidecarUnreadable",
                        format!("The XMP sidecar could not be read: {error}"),
                    ));
                    metadata
                }
            }
        }
        WriteTarget::EmbeddedJpeg { path } => match read_jpeg_xmp(path) {
            Ok(Some(packet)) => {
                metadata_from_bytes(&packet, MetadataSource::Embedded, xmp_revision(&packet))
            }
            Ok(None) => PhotoMetadata::default(),
            Err(error) => {
                let mut metadata = PhotoMetadata::default();
                metadata.issues.push(MetadataIssue::new(
                    "embeddedUnreadable",
                    format!("Embedded XMP could not be read: {error}"),
                ));
                metadata
            }
        },
    }
}

fn set_keywords_match(expected: &[String], actual: &[String]) -> bool {
    let mut expected = expected.to_vec();
    let mut actual = actual.to_vec();
    expected.sort();
    actual.sort();
    expected == actual
}

fn verify_target_fields(
    target: &WriteTarget,
    update: &MetadataUpdate,
) -> std::result::Result<(), String> {
    let metadata = metadata_after_read(target);
    if has_fatal_metadata_issues(&metadata) {
        return Err(format!(
            "The XMP could not be parsed after writing: {}",
            metadata
                .issues
                .iter()
                .map(|issue| issue.message.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    if let Some(rating) = update.rating {
        if metadata.rating != rating {
            return Err(format!(
                "The rating was not written correctly (expected {rating}, found {}).",
                metadata.rating
            ));
        }
    }
    if update.clear_label || update.label.is_some() {
        if metadata.label != update.label {
            return Err("The color label was not written correctly.".into());
        }
    }
    if update.clear_cull || update.cull.is_some() {
        if metadata.cull != update.cull {
            return Err("The cull flag was not written correctly.".into());
        }
    }
    if let Some(favorite) = update.favorite {
        if metadata.favorite != favorite {
            return Err("The favorite flag was not written correctly.".into());
        }
    }
    if let Some(keywords) = &update.keywords {
        if !set_keywords_match(keywords, &metadata.keywords) {
            return Err("The flat keywords were not written correctly.".into());
        }
    }
    if let Some(keywords) = &update.hierarchical_keywords {
        if !set_keywords_match(keywords, &metadata.hierarchical_keywords) {
            return Err("The hierarchical keywords were not written correctly.".into());
        }
    }
    Ok(())
}

fn verify_preservation(
    before: &XmpDocument,
    after: &XmpDocument,
    update: &MetadataUpdate,
) -> std::result::Result<(), String> {
    let before = before.preservation_snapshot();
    let after = after.preservation_snapshot();

    let mut problems = Vec::new();
    for (key, values) in &before {
        match after.get(key) {
            Some(after_values) if after_values == values => {}
            Some(_) => problems.push(format!("{key} changed")),
            None => problems.push(format!("{key} was removed")),
        }
    }
    for key in after.keys() {
        if !before.contains_key(key) && !update.touches(key) {
            problems.push(format!("{key} was added"));
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Writing metadata would modify unrelated XMP properties: {}.",
            problems.join(", ")
        ))
    }
}

fn file_times(path: &Path) -> io::Result<Option<(SystemTime, SystemTime)>> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    let accessed = metadata.accessed().unwrap_or(modified);
    Ok(Some((modified, accessed)))
}

fn apply_file_times(path: &Path, times: Option<(SystemTime, SystemTime)>) -> io::Result<()> {
    let Some((modified, accessed)) = times else {
        return Ok(());
    };
    let file = fs::OpenOptions::new().write(true).open(path)?;
    let mut file_times = fs::FileTimes::new();
    file_times = file_times.set_modified(modified);
    file_times = file_times.set_accessed(accessed);
    file.set_times(file_times)
}

fn replace_file(staged: &Path, target: &Path) -> io::Result<()> {
    #[cfg(windows)]
    if target.exists() {
        fs::remove_file(target)?;
    }
    fs::rename(staged, target)
}

fn copy_permissions(from: &Path, to: &Path) -> io::Result<()> {
    let permissions = fs::metadata(from)?.permissions();
    fs::set_permissions(to, permissions)
}

fn write_sidecar(
    target: &Path,
    existed: bool,
    primary: &Path,
    update: &MetadataUpdate,
    exiftool: &Path,
    config: Option<&Path>,
) -> Result<()> {
    let staged = temp_path_near(target, "xmp");
    let result = (|| -> Result<()> {
        if existed {
            fs::copy(target, &staged).map_err(|error| LumioError::io(&staged, error))?;
            copy_permissions(target, &staged).map_err(|error| LumioError::io(&staged, error))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| LumioError::io(parent, error))?;
            }
            fs::write(&staged, EMPTY_XMP_PACKET).map_err(|error| LumioError::io(&staged, error))?;
        }

        let before = match fs::read(&staged) {
            Ok(bytes) => {
                parse_xmp_bytes(&bytes).map_err(|message| LumioError::Metadata(message))?
            }
            Err(error) => return Err(LumioError::io(&staged, error)),
        };
        let mut args = exiftool_args(
            &WriteTarget::Sidecar {
                path: target.to_path_buf(),
                existed,
            },
            update,
        );
        args.push(staged.to_string_lossy().into_owned());
        run_exiftool(exiftool, config, &args).map_err(LumioError::Metadata)?;

        let after_bytes = fs::read(&staged).map_err(|error| LumioError::io(&staged, error))?;
        let after = parse_xmp_bytes(&after_bytes).map_err(LumioError::Metadata)?;
        verify_preservation(&before, &after, update).map_err(LumioError::Metadata)?;
        verify_target_fields(
            &WriteTarget::Sidecar {
                path: staged.clone(),
                existed: true,
            },
            update,
        )
        .map_err(LumioError::Metadata)?;

        // A fresh sidecar inherits the primary file's permissions so backup
        // and sync tools treat it like the photo it describes.
        if !existed {
            copy_permissions(primary, &staged).map_err(|error| LumioError::io(&staged, error))?;
        }
        replace_file(&staged, target).map_err(|error| LumioError::io(target, error))?;
        Ok(())
    })();
    let _ = fs::remove_file(&staged);
    result
}

fn write_embedded_jpeg(
    target: &Path,
    update: &MetadataUpdate,
    exiftool: &Path,
    config: Option<&Path>,
) -> Result<()> {
    let original = fs::read(target).map_err(|error| LumioError::io(target, error))?;
    let original_without_xmp = jpeg_bytes_without_xmp(&original)
        .map_err(|error| LumioError::Metadata(error.to_string()))?;
    let times = file_times(target).map_err(|error| LumioError::io(target, error))?;
    let staged = temp_path_near(target, "jpg");
    let _ = fs::remove_file(&staged);

    let result = (|| -> Result<()> {
        let mut args = exiftool_args(
            &WriteTarget::EmbeddedJpeg {
                path: target.to_path_buf(),
            },
            update,
        );
        args.push("-o".into());
        args.push(staged.to_string_lossy().into_owned());
        args.push(target.to_string_lossy().into_owned());
        run_exiftool(exiftool, config, &args).map_err(LumioError::Metadata)?;
        if !staged.is_file() {
            return Err(LumioError::Metadata(format!(
                "ExifTool did not create the staged JPEG at {} (arguments: {}).",
                staged.display(),
                args.join(" ")
            )));
        }

        let staged_bytes = fs::read(&staged).map_err(|error| LumioError::io(&staged, error))?;
        let staged_without_xmp = jpeg_bytes_without_xmp(&staged_bytes)
            .map_err(|error| LumioError::Metadata(format!("staged comparison: {error}")))?;
        if staged_without_xmp != original_without_xmp {
            return Err(LumioError::Metadata(
                "ExifTool would modify non-XMP JPEG data. The update was cancelled.".into(),
            ));
        }

        let before_document = read_jpeg_xmp(target)
            .map_err(|error| {
                LumioError::Metadata(format!(
                    "read original {} (exists {}): {error}",
                    target.display(),
                    target.exists()
                ))
            })?
            .map(|packet| parse_xmp_bytes(&packet))
            .transpose()
            .map_err(LumioError::Metadata)?;
        let before_document = before_document.unwrap_or_default();
        let after_packet = read_jpeg_xmp(&staged)
            .map_err(|error| LumioError::Metadata(format!("read staged: {error}")))?
            .ok_or_else(|| LumioError::Metadata("ExifTool did not create XMP metadata.".into()))?;
        let after_document = parse_xmp_bytes(&after_packet).map_err(LumioError::Metadata)?;
        verify_preservation(&before_document, &after_document, update)
            .map_err(LumioError::Metadata)?;
        verify_target_fields(
            &WriteTarget::EmbeddedJpeg {
                path: staged.clone(),
            },
            update,
        )
        .map_err(LumioError::Metadata)?;

        copy_permissions(target, &staged).map_err(|error| LumioError::io(&staged, error))?;
        apply_file_times(&staged, times).map_err(|error| LumioError::io(&staged, error))?;
        replace_file(&staged, target).map_err(|error| LumioError::io(target, error))?;
        Ok(())
    })();
    let _ = fs::remove_file(&staged);
    result
}

/// Write metadata to the effective target. The supplied `expected_revision`
/// must match the packet currently on disk; this rejects stale edits made while
/// the photo was open in another application.
pub fn write_photo_metadata(
    primary: &Path,
    sidecar: Option<&Path>,
    update: &MetadataUpdate,
    expected_revision: Option<&str>,
    exiftool: &Path,
    config: Option<&Path>,
) -> Result<MetadataWriteResult> {
    let update = update.clone().normalized();
    if update.is_empty() {
        return Err(LumioError::Metadata(
            "No metadata changes were requested.".into(),
        ));
    }
    if !primary.is_file() {
        return Err(LumioError::NotFound(primary.to_path_buf()));
    }

    let target = resolve_write_target(primary, sidecar);
    let current = metadata_after_read(&target);
    if has_fatal_metadata_issues(&current) {
        return Err(LumioError::Metadata(format!(
            "The existing XMP cannot be edited safely: {}",
            current
                .issues
                .iter()
                .map(|issue| issue.message.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        )));
    }
    if let Some(expected) = expected_revision {
        if expected != current.revision {
            return Err(LumioError::MetadataConflict(
                "The photo metadata changed on disk since it was loaded. Refresh the folder and try again.".into(),
            ));
        }
    }

    let created_sidecar = matches!(&target, WriteTarget::Sidecar { existed: false, .. });
    match &target {
        WriteTarget::Sidecar { path, existed } => {
            write_sidecar(path, *existed, primary, &update, exiftool, config)?;
        }
        WriteTarget::EmbeddedJpeg { path } => {
            write_embedded_jpeg(path, &update, exiftool, config)?;
        }
    }

    let metadata = match &target {
        WriteTarget::Sidecar { path, .. } => {
            let bytes = fs::read(path).map_err(|error| LumioError::io(path, error))?;
            metadata_from_bytes(&bytes, MetadataSource::Sidecar, xmp_revision(&bytes))
        }
        WriteTarget::EmbeddedJpeg { path } => match read_jpeg_xmp(path) {
            Ok(Some(packet)) => {
                metadata_from_bytes(&packet, MetadataSource::Embedded, xmp_revision(&packet))
            }
            Ok(None) => PhotoMetadata::default(),
            Err(error) => {
                let mut metadata = PhotoMetadata::default();
                metadata.issues.push(MetadataIssue::new(
                    "embeddedUnreadable",
                    format!("Embedded XMP could not be read: {error}"),
                ));
                metadata
            }
        },
    };

    let file_metadata = fs::metadata(primary).map_err(|error| LumioError::io(primary, error))?;
    Ok(MetadataWriteResult {
        metadata,
        target: target.path().to_path_buf(),
        created_sidecar,
        file_size: file_metadata.len(),
        modified_ms: modified_ms(&file_metadata),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exiftool() -> Option<PathBuf> {
        crate::ingest::metadata::discover_exiftool(None).map(|info| info.path)
    }

    fn config(dir: &Path) -> PathBuf {
        let path = dir.join("exiftool.config");
        write_exiftool_config(&path).unwrap();
        path
    }

    const DARKTABLE_XMP: &str = r#"<?xpacket begin="﻿" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:lr="http://ns.adobe.com/lightroom/1.0/"
    xmlns:darktable="http://darktable.sf.net/"
    xmlns:private="http://example.com/private/1.0/"
    xmlns:photoshop="http://ns.adobe.com/photoshop/1.0/">
   <xmp:Rating>2</xmp:Rating>
   <xmp:Label>Yellow</xmp:Label>
   <dc:subject>
    <rdf:Bag>
     <rdf:li>travel</rdf:li>
     <rdf:li>beach</rdf:li>
    </rdf:Bag>
   </dc:subject>
   <lr:hierarchicalSubject>
    <rdf:Bag>
     <rdf:li>Travel|Beach</rdf:li>
    </rdf:Bag>
   </lr:hierarchicalSubject>
   <darktable:history>
    <rdf:Seq>
     <rdf:li>exposure 0.5</rdf:li>
     <rdf:li>tone curve</rdf:li>
    </rdf:Seq>
   </darktable:history>
   <darktable:mask>
    <rdf:Bag>
     <rdf:li>brush mask one</rdf:li>
     <rdf:li>brush mask two</rdf:li>
    </rdf:Bag>
   </darktable:mask>
   <darktable:colorlabels>4</darktable:colorlabels>
   <private:Secret>keep this exactly</private:Secret>
   <photoshop:City>Palawan</photoshop:City>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

    fn write_fixture(dir: &Path) -> PathBuf {
        fs::write(dir.join("IMG_0001.RAF"), b"raw").unwrap();
        let path = dir.join("IMG_0001.RAF.xmp");
        fs::write(&path, DARKTABLE_XMP).unwrap();
        path
    }

    #[test]
    fn parses_ratings_labels_culling_keywords_and_hierarchy() {
        let bytes = br#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:xmp="http://ns.adobe.com/xap/1.0/"
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:lr="http://ns.adobe.com/lightroom/1.0/"
    xmlns:lumio="https://lumio.app/ns/1.0/">
   <xmp:Rating>4</xmp:Rating>
   <xmp:Label>Green</xmp:Label>
   <lumio:Cull>pick</lumio:Cull>
   <lumio:Favorite>True</lumio:Favorite>
   <dc:subject><rdf:Bag><rdf:li>one</rdf:li><rdf:li>two</rdf:li></rdf:Bag></dc:subject>
   <lr:hierarchicalSubject><rdf:Bag><rdf:li>A|B</rdf:li><rdf:li>A|C</rdf:li></rdf:Bag></lr:hierarchicalSubject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;
        let metadata = metadata_from_bytes(bytes, MetadataSource::Sidecar, "rev".into());
        assert_eq!(metadata.rating, 4);
        assert_eq!(metadata.label, Some(ColorLabel::Green));
        assert_eq!(metadata.cull, Some(CullFlag::Pick));
        assert!(metadata.favorite);
        assert_eq!(metadata.keywords, vec!["one", "two"]);
        assert_eq!(metadata.hierarchical_keywords, vec!["A|B", "A|C"]);
        assert!(metadata.issues.is_empty());
    }

    #[test]
    fn decodes_xml_entities_in_text_and_attribute_values() {
        let bytes = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:dc="http://purl.org/dc/elements/1.1/"
    xmlns:private="http://example.com/private/1.0/"
    private:Note="Tom &amp; Jerry &quot;quoted&quot;">
   <dc:subject><rdf:Bag>
    <rdf:li>Rock &amp; Roll</rdf:li>
    <rdf:li>Caf&#233;</rdf:li>
    <rdf:li>Less &lt; More</rdf:li>
   </rdf:Bag></dc:subject>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;
        let document = parse_xmp_bytes(bytes).unwrap();
        assert_eq!(
            document.simple("{http://example.com/private/1.0/}Note"),
            Some("Tom & Jerry \"quoted\"")
        );
        let metadata = metadata_from_bytes(bytes, MetadataSource::Sidecar, "rev".into());
        assert_eq!(
            metadata.keywords,
            vec!["Rock & Roll", "Café", "Less < More"]
        );
        assert!(metadata.issues.is_empty());
    }

    #[test]
    fn reads_minus_one_as_rejected() {
        let bytes = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
   <xmp:Rating>-1</xmp:Rating>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>"#;
        let metadata = metadata_from_bytes(bytes, MetadataSource::Embedded, "rev".into());
        assert_eq!(metadata.rating, 0);
        assert_eq!(metadata.cull, Some(CullFlag::Reject));
    }

    #[test]
    fn preservation_snapshot_ignores_order_but_tracks_darktable_sequences() {
        let document = parse_xmp_bytes(DARKTABLE_XMP.as_bytes()).unwrap();
        let snapshot = document.preservation_snapshot();
        let history = snapshot
            .get("{http://darktable.sf.net/}history")
            .and_then(|values| values.first())
            .unwrap();
        assert_eq!(history, "seq[exposure 0.5|tone curve]");
        assert!(snapshot.contains_key("{http://example.com/private/1.0/}Secret"));
        assert!(snapshot.contains_key("{http://ns.adobe.com/photoshop/1.0/}City"));
        assert!(!snapshot.contains_key(KEY_RATING));
    }

    #[test]
    fn writes_metadata_and_preserves_darktable_fields() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let sidecar = write_fixture(temp.path());
        let config = config(temp.path());
        let before = parse_xmp_bytes(&fs::read(&sidecar).unwrap())
            .unwrap()
            .preservation_snapshot();

        let update = MetadataUpdate {
            rating: Some(5),
            label: Some(ColorLabel::Blue),
            cull: Some(CullFlag::Pick),
            favorite: Some(true),
            ..MetadataUpdate::default()
        };
        let result = write_photo_metadata(
            &temp.path().join("IMG_0001.RAF"),
            Some(&sidecar),
            &update,
            None,
            &exiftool,
            Some(&config),
        )
        .unwrap();

        assert_eq!(result.metadata.rating, 5);
        assert_eq!(result.metadata.label, Some(ColorLabel::Blue));
        assert_eq!(result.metadata.cull, Some(CullFlag::Pick));
        assert!(result.metadata.favorite);
        let after = parse_xmp_bytes(&fs::read(&sidecar).unwrap())
            .unwrap()
            .preservation_snapshot();
        assert_eq!(before, after);

        let content = fs::read_to_string(&sidecar).unwrap();
        assert!(content.contains("exposure 0.5"));
        assert!(content.contains("brush mask two"));
        assert!(content.contains("keep this exactly"));
        assert!(content.contains("Palawan"));
    }

    #[test]
    fn writes_flat_and_hierarchical_keywords() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let sidecar = write_fixture(temp.path());
        let config = config(temp.path());
        let update = MetadataUpdate {
            keywords: Some(vec!["new".into(), "keywords".into()]),
            hierarchical_keywords: Some(vec!["Places|Palawan".into(), "People|Radek".into()]),
            ..MetadataUpdate::default()
        };
        let result = write_photo_metadata(
            &temp.path().join("IMG_0001.RAF"),
            Some(&sidecar),
            &update,
            None,
            &exiftool,
            Some(&config),
        )
        .unwrap();
        assert_eq!(result.metadata.keywords, vec!["new", "keywords"]);
        assert_eq!(
            result.metadata.hierarchical_keywords,
            vec!["Places|Palawan", "People|Radek"]
        );
    }

    #[test]
    fn clears_fields_without_touching_other_properties() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let sidecar = write_fixture(temp.path());
        let config = config(temp.path());
        let update = MetadataUpdate {
            rating: Some(0),
            clear_label: true,
            clear_cull: true,
            favorite: Some(false),
            ..MetadataUpdate::default()
        };
        let result = write_photo_metadata(
            &temp.path().join("IMG_0001.RAF"),
            Some(&sidecar),
            &update,
            None,
            &exiftool,
            Some(&config),
        )
        .unwrap();
        assert_eq!(result.metadata.rating, 0);
        assert_eq!(result.metadata.label, None);
        assert_eq!(result.metadata.cull, None);
        assert!(!result.metadata.favorite);
        assert_eq!(result.metadata.keywords, vec!["travel", "beach"]);
    }

    #[test]
    fn rejects_stale_revisions() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let sidecar = write_fixture(temp.path());
        let config = config(temp.path());
        let update = MetadataUpdate {
            rating: Some(5),
            ..MetadataUpdate::default()
        };
        let error = write_photo_metadata(
            &temp.path().join("IMG_0001.RAF"),
            Some(&sidecar),
            &update,
            Some("wrong-revision"),
            &exiftool,
            Some(&config),
        )
        .unwrap_err();
        assert_eq!(error.code(), "metadataConflict");
        assert!(
            fs::read_to_string(&sidecar)
                .unwrap()
                .contains("<xmp:Rating>2</xmp:Rating>")
        );
    }

    #[test]
    fn creates_a_sidecar_from_scratch() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let raw = temp.path().join("new.NEF");
        fs::write(&raw, b"raw").unwrap();
        let config = config(temp.path());
        let update = MetadataUpdate {
            rating: Some(3),
            ..MetadataUpdate::default()
        };
        let result =
            write_photo_metadata(&raw, None, &update, Some(""), &exiftool, Some(&config)).unwrap();
        assert_eq!(result.metadata.rating, 3);
        assert!(result.created_sidecar);
        assert!(result.target.exists());
        assert_eq!(result.target, temp.path().join("new.NEF.xmp"));
        // The revision returned after a write must match a fresh read.
        let read = read_photo_metadata(&raw, Some(&result.target));
        assert_eq!(read.revision, result.metadata.revision);
        assert_eq!(read.rating, 3);
    }

    #[test]
    fn writes_embedded_xmp_into_jpeg_and_preserves_other_segments() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let jpeg = temp.path().join("photo.jpg");
        let image = image::RgbImage::new(16, 12);
        image.save(&jpeg).unwrap();
        // Add an EXIF segment through ExifTool so the test covers a second
        // metadata segment that must survive byte-identically.
        let status = Command::new(&exiftool)
            .args([
                "-overwrite_original",
                "-EXIF:Artist=Lumio Test",
                &jpeg.to_string_lossy(),
            ])
            .status()
            .unwrap();
        assert!(status.success());

        let original = fs::read(&jpeg).unwrap();
        let original_other = jpeg_bytes_without_xmp(&original).unwrap();
        let modified_before = fs::metadata(&jpeg).unwrap().modified().unwrap();
        let config = config(temp.path());
        let update = MetadataUpdate {
            rating: Some(4),
            label: Some(ColorLabel::Red),
            cull: Some(CullFlag::Reject),
            favorite: Some(true),
            keywords: Some(vec!["embedded".into()]),
            ..MetadataUpdate::default()
        };
        let result =
            write_photo_metadata(&jpeg, None, &update, Some(""), &exiftool, Some(&config)).unwrap();
        assert_eq!(result.metadata.source, MetadataSource::Embedded);
        assert_eq!(result.metadata.rating, 4);
        assert_eq!(result.metadata.label, Some(ColorLabel::Red));
        assert_eq!(result.metadata.cull, Some(CullFlag::Reject));
        assert!(result.metadata.favorite);
        assert_eq!(result.metadata.keywords, vec!["embedded"]);

        let written = fs::read(&jpeg).unwrap();
        assert_eq!(jpeg_bytes_without_xmp(&written).unwrap(), original_other);
        assert_eq!(
            fs::metadata(&jpeg).unwrap().modified().unwrap(),
            modified_before
        );
    }

    #[test]
    fn jpeg_with_a_sidecar_writes_only_the_sidecar() {
        let Some(exiftool) = exiftool() else {
            return;
        };
        let temp = tempfile::tempdir().unwrap();
        let jpeg = temp.path().join("photo.jpg");
        image::RgbImage::new(16, 12).save(&jpeg).unwrap();
        let original = fs::read(&jpeg).unwrap();
        let original_modified = fs::metadata(&jpeg).unwrap().modified().unwrap();
        let sidecar = temp.path().join("photo.jpg.xmp");
        fs::write(&sidecar, DARKTABLE_XMP).unwrap();
        let config = config(temp.path());

        let update = MetadataUpdate {
            rating: Some(5),
            ..MetadataUpdate::default()
        };
        let result = write_photo_metadata(
            &jpeg,
            Some(&sidecar),
            &update,
            None,
            &exiftool,
            Some(&config),
        )
        .unwrap();

        assert_eq!(result.target, sidecar);
        assert_eq!(result.metadata.source, MetadataSource::Sidecar);
        assert_eq!(result.metadata.rating, 5);
        // The JPEG remains byte-identical: the sidecar is the sole target.
        assert_eq!(fs::read(&jpeg).unwrap(), original);
        assert_eq!(
            fs::metadata(&jpeg).unwrap().modified().unwrap(),
            original_modified
        );
        let content = fs::read_to_string(&sidecar).unwrap();
        assert!(content.contains("exposure 0.5"));
        assert!(content.contains("keep this exactly"));
    }

    #[test]
    fn jpeg_without_xmp_reports_none() {
        let temp = tempfile::tempdir().unwrap();
        let jpeg = temp.path().join("plain.jpg");
        image::RgbImage::new(8, 8).save(&jpeg).unwrap();
        let metadata = read_photo_metadata(&jpeg, None);
        assert_eq!(metadata.source, MetadataSource::None);
        assert_eq!(metadata.revision, "");
        assert!(!metadata.has_edit());
    }
}
