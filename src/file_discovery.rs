use crate::{pdf_utils, Result};
use lopdf::{Document, ObjectId};

/// Handles discovery of embedded file specifications from PDF documents.
///
/// This module contains logic to find file specifications from two sources:
/// 1. The /Names/EmbeddedFiles name tree in the document catalog
/// 2. /FileAttachment annotations on pages
pub struct FileSpecDiscovery<'a> {
    document: &'a Document,
}

impl<'a> FileSpecDiscovery<'a> {
    pub fn new(document: &'a Document) -> Self {
        Self { document }
    }

    /// Helper to resolve a value that might be inline or a reference to a dictionary.
    fn resolve_dict(&self, value: &lopdf::Object) -> Option<lopdf::Dictionary> {
        if let Ok(id) = value.as_reference() {
            self.document
                .get_object(id)
                .ok()
                .and_then(|o| o.as_dict().ok().cloned())
        } else {
            value.as_dict().ok().cloned()
        }
    }

    /// Helper to resolve a value that might be inline or a reference to an array.
    fn resolve_array(&self, value: &lopdf::Object) -> Option<Vec<lopdf::Object>> {
        if let Ok(id) = value.as_reference() {
            self.document
                .get_object(id)
                .ok()
                .and_then(|o| o.as_array().ok().cloned())
        } else {
            value.as_array().ok().cloned()
        }
    }

    /// Process a names array, extracting (name, ObjectId) pairs.
    fn process_names_array(&self, names_array: &[lopdf::Object]) -> Vec<(String, ObjectId)> {
        let mut pairs = Vec::new();
        let mut i = 0;
        while i + 1 < names_array.len() {
            if let Ok(name_bytes) = names_array[i].as_str() {
                let name = String::from_utf8_lossy(name_bytes).into_owned();
                if let Ok(spec_id) = names_array[i + 1].as_reference() {
                    pairs.push((name, spec_id));
                }
            }
            i += 2;
        }
        pairs
    }

    /// Collect `(name, ObjectId)` pairs for every embedded-file specification
    /// in the document.
    ///
    /// Two sources are searched:
    /// 1. The `/Names/EmbeddedFiles` name tree in the document catalog.
    /// 2. `/FileAttachment` annotations on every page.
    pub fn collect_file_specs(&self) -> Result<Vec<(String, ObjectId)>> {
        let mut specs = Vec::new();
        
        specs.extend(self.collect_from_names_tree());
        specs.extend(self.collect_from_annotations());
        
        Ok(specs)
    }

    /// Collect file specifications from the document's names tree.
    fn collect_from_names_tree(&self) -> Vec<(String, ObjectId)> {
        let catalog = match self.document.catalog() {
            Ok(cat) => cat,
            Err(_) => return Vec::new(),
        };

        let names_val = match catalog.get(b"Names") {
            Ok(val) => val,
            Err(_) => return Vec::new(),
        };

        let names_dict = match self.resolve_dict(names_val) {
            Some(dict) => dict,
            None => return Vec::new(),
        };

        let ef_val = match names_dict.get(b"EmbeddedFiles") {
            Ok(val) => val,
            Err(_) => return Vec::new(),
        };

        if let Ok(ef_id) = ef_val.as_reference() {
            self.walk_name_tree(ef_id)
        } else if let Ok(ef_dict) = ef_val.as_dict() {
            // Handle inline /EmbeddedFiles dictionary
            self.extract_from_inline_ef_dict(ef_dict)
        } else {
            Vec::new()
        }
    }

    /// Extract file specifications from an inline EmbeddedFiles dictionary.
    fn extract_from_inline_ef_dict(&self, ef_dict: &lopdf::Dictionary) -> Vec<(String, ObjectId)> {
        if let Ok(names_val) = ef_dict.get(b"Names") {
            if let Ok(names_array) = names_val.as_array() {
                return self.process_names_array(names_array);
            }
        }
        Vec::new()
    }

    /// Collect file specifications from page FileAttachment annotations.
    fn collect_from_annotations(&self) -> Vec<(String, ObjectId)> {
        let mut specs = Vec::new();
        let pages = self.document.get_pages();
        
        for page_id in pages.values() {
            specs.extend(self.process_page_annotations(*page_id));
        }
        
        specs
    }

    /// Process annotations on a single page.
    fn process_page_annotations(&self, page_id: ObjectId) -> Vec<(String, ObjectId)> {
        let page_obj = match self.document.get_object(page_id) {
            Ok(obj) => obj,
            Err(_) => return Vec::new(),
        };

        let page_dict = match page_obj.as_dict() {
            Ok(dict) => dict,
            Err(_) => return Vec::new(),
        };

        let annots_val = match page_dict.get(b"Annots") {
            Ok(val) => val,
            Err(_) => return Vec::new(),
        };

        let annots_array = match self.resolve_array(annots_val) {
            Some(array) => array,
            None => return Vec::new(),
        };

        self.extract_file_attachments(&annots_array)
    }

    /// Extract file attachments from an annotations array.
    fn extract_file_attachments(&self, annots: &[lopdf::Object]) -> Vec<(String, ObjectId)> {
        let mut specs = Vec::new();
        
        for item in annots {
            if let Ok(annot_id) = item.as_reference() {
                if let Some((name, fs_id)) = self.process_file_attachment_annotation(annot_id) {
                    specs.push((name, fs_id));
                }
            }
        }
        
        specs
    }

    /// Process a single FileAttachment annotation.
    fn process_file_attachment_annotation(&self, annot_id: ObjectId) -> Option<(String, ObjectId)> {
        let annot_obj = self.document.get_object(annot_id).ok()?;
        let dict = annot_obj.as_dict().ok()?;
        
        // Check if this is a FileAttachment
        let subtype_name = dict.get(b"Subtype").ok()?.as_name().ok()?;
        if subtype_name != b"FileAttachment" {
            return None;
        }
        
        // Get the file specification reference
        let fs_val = dict.get(b"FS").ok()?;
        let fs_id = fs_val.as_reference().ok()?;
        
        let name = Self::annotation_name(dict);
        Some((name, fs_id))
    }

    /// Recursively walk a PDF name tree, collecting
    /// `(name_string, file_spec_object_id)` pairs from leaf nodes.
    fn walk_name_tree(&self, node_id: ObjectId) -> Vec<(String, ObjectId)> {
        let mut out = Vec::new();

        let node_obj = match self.document.get_object(node_id) {
            Ok(o) => o,
            Err(_) => return out,
        };
        
        let node_dict = match node_obj.as_dict() {
            Ok(d) => d,
            Err(_) => return out,
        };

        // Leaf node: has a /Names array of [key, value, key, value, …]
        if let Ok(names_val) = node_dict.get(b"Names") {
            if let Ok(arr) = names_val.as_array() {
                out.extend(self.process_names_array(arr));
            }
        }

        // Intermediate node: has a /Kids array of references
        if let Ok(kids_val) = node_dict.get(b"Kids") {
            if let Ok(kids) = kids_val.as_array() {
                for kid in kids {
                    if let Ok(kid_id) = kid.as_reference() {
                        out.extend(self.walk_name_tree(kid_id));
                    }
                }
            }
        }

        out
    }

    /// Extract a display name from a FileAttachment annotation dictionary.
    /// Falls back to `"attachment"` if neither `/Contents` nor `/T` is set.
    fn annotation_name(dict: &lopdf::Dictionary) -> String {
        for key in [b"Contents" as &[u8], b"T"] {
            if let Some(name) = pdf_utils::extract_string_from_dict(dict, key) {
                return name;
            }
        }
        "attachment".into()
    }
}