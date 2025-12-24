use std::collections::HashMap;
use std::io::{self, Write, Read};

/// DOM node types for Machine-HTTP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomNodeType {
    Element,
    Text,
    Comment,
    Document,
    DocumentFragment,
}

/// DOM node structure with efficient representation for snapshots
#[derive(Debug, Clone)]
pub struct DomNode {
    pub node_type: DomNodeType,
    pub tag_name: Option<String>,
    pub attributes: HashMap<String, String>,
    pub text_content: Option<String>,
    pub children: Vec<DomNode>,
    pub is_self_closing: bool,
    pub id: Option<u32>, // Optional unique identifier for efficient diffing
}

impl Default for DomNode {
    fn default() -> Self {
        DomNode {
            node_type: DomNodeType::Element,
            tag_name: None,
            attributes: HashMap::new(),
            text_content: None,
            children: Vec::new(),
            is_self_closing: false,
            id: None,
        }
    }
}

impl DomNode {
    /// Create a new element node
    pub fn new_element(tag_name: &str) -> Self {
        DomNode {
            node_type: DomNodeType::Element,
            tag_name: Some(tag_name.to_string()),
            attributes: HashMap::new(),
            text_content: None,
            children: Vec::new(),
            is_self_closing: false,
            id: None,
        }
    }

    /// Create a new text node
    pub fn new_text(content: &str) -> Self {
        DomNode {
            node_type: DomNodeType::Text,
            tag_name: None,
            attributes: HashMap::new(),
            text_content: Some(content.to_string()),
            children: Vec::new(),
            is_self_closing: false,
            id: None,
        }
    }

    /// Add an attribute to the node
    pub fn add_attribute(&mut self, name: &str, value: &str) -> &mut Self {
        self.attributes.insert(name.to_string(), value.to_string());
        self
    }

    /// Add a child node
    pub fn add_child(&mut self, child: DomNode) -> &mut Self {
        self.children.push(child);
        self
    }

    /// Set unique ID for diffing
    pub fn set_id(&mut self, id: u32) -> &mut Self {
        self.id = Some(id);
        self
    }
}

/// Binary DOM serializer/deserializer for efficient snapshots
pub struct BinaryDomSerializer;

impl BinaryDomSerializer {
    /// Serialize DOM node to binary format
    pub fn serialize<W: Write>(node: &DomNode, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        // Write node type as u8
        writer.write_all(&[(node.node_type as u8)])?;

        // Write tag name if element
        if let Some(tag_name) = &node.tag_name {
            Self::write_string(tag_name, writer)?;
        } else {
            writer.write_all(&[0])?; // No tag name
        }

        // Write attributes count
        let attr_count = node.attributes.len() as u16;
        writer.write_all(&attr_count.to_le_bytes())?;

        // Write attributes
        for (key, value) in &node.attributes {
            Self::write_string(key, writer)?;
            Self::write_string(value, writer)?;
        }

        // Write text content if text node
        if let Some(text) = &node.text_content {
            Self::write_string(text, writer)?;
        } else {
            writer.write_all(&[0])?; // No text content
        }

        // Write is_self_closing flag
        writer.write_all(&[(node.is_self_closing as u8)])?;

        // Write ID if present
        if let Some(id) = node.id {
            writer.write_all(&[1])?; // Has ID
            writer.write_all(&id.to_le_bytes())?;
        } else {
            writer.write_all(&[0])?; // No ID
        }

        // Write children count
        let children_count = node.children.len() as u32;
        writer.write_all(&children_count.to_le_bytes())?;

        // Write children recursively
        for child in &node.children {
            Self::serialize(child, writer)?;
        }

        Ok(())
    }

    /// Deserialize DOM node from binary format
    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<DomNode>
    where
        R: Read,
    {
        // Read node type
        let mut node_type_buf = [0; 1];
        reader.read_exact(&mut node_type_buf)?;
        let node_type = match node_type_buf[0] {
            0 => DomNodeType::Element,
            1 => DomNodeType::Text,
            2 => DomNodeType::Comment,
            3 => DomNodeType::Document,
            4 => DomNodeType::DocumentFragment,
            _ => return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid node type")),
        };

        // Read tag name if element
        let tag_name = if node_type == DomNodeType::Element {
            Self::read_string(reader)?
        } else {
            None
        };

        // Read attributes
        let mut attr_buf = [0; 2];
        reader.read_exact(&mut attr_buf)?;
        let attr_count = u16::from_le_bytes(attr_buf);

        let mut attributes = HashMap::new();
        for _ in 0..attr_count {
            let key = Self::read_string(reader)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Expected attribute key")
            })?;
            let value = Self::read_string(reader)?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "Expected attribute value")
            })?;
            attributes.insert(key, value);
        }

        // Read text content if text node
        let text_content = if node_type == DomNodeType::Text {
            Self::read_string(reader)?
        } else {
            None
        };

        // Read is_self_closing flag
        let mut self_closing_buf = [0; 1];
        reader.read_exact(&mut self_closing_buf)?;
        let is_self_closing = self_closing_buf[0] != 0;

        // Read ID if present
        let mut has_id_buf = [0; 1];
        reader.read_exact(&mut has_id_buf)?;
        let id = if has_id_buf[0] != 0 {
            let mut id_buf = [0; 4];
            reader.read_exact(&mut id_buf)?;
            Some(u32::from_le_bytes(id_buf))
        } else {
            None
        };

        // Read children
        let mut children_count_buf = [0; 4];
        reader.read_exact(&mut children_count_buf)?;
        let children_count = u32::from_le_bytes(children_count_buf);

        let mut children = Vec::new();
        for _ in 0..children_count {
            let child = Self::deserialize(reader)?;
            children.push(child);
        }

        Ok(DomNode {
            node_type,
            tag_name,
            attributes,
            text_content,
            children,
            is_self_closing,
            id,
        })
    }

    /// Write string to binary format with length prefix
    fn write_string<W: Write>(s: &str, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        let len = s.len() as u16;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(s.as_bytes())?;
        Ok(())
    }

    /// Read string from binary format with length prefix
    fn read_string<R: Read>(reader: &mut R) -> io::Result<Option<String>>
    where
        R: Read,
    {
        let mut len_buf = [0; 2];
        reader.read_exact(&mut len_buf)?;
        let len = u16::from_le_bytes(len_buf);

        if len == 0 {
            return Ok(None);
        }

        let mut str_buf = vec![0; len as usize];
        reader.read_exact(&mut str_buf)?;
        let s = String::from_utf8(str_buf).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 string")
        })?;

        Ok(Some(s))
    }
}

/// DOM diff operation types
#[derive(Debug, Clone)]
pub enum DomDiffOperation {
    InsertNode { index: usize, node: DomNode },
    UpdateNode { index: usize, changes: DomChanges },
    DeleteNode { index: usize },
    MoveNode { from_index: usize, to_index: usize },
    UpdateText { index: usize, new_text: String },
}

/// Changes to a DOM node for efficient diffing
#[derive(Debug, Clone)]
pub struct DomChanges {
    pub added_attributes: HashMap<String, String>,
    pub removed_attributes: Vec<String>,
    pub updated_attributes: HashMap<String, String>,
    pub children_changes: Vec<DomDiffOperation>,
}

impl Default for DomChanges {
    fn default() -> Self {
        DomChanges {
            added_attributes: HashMap::new(),
            removed_attributes: Vec::new(),
            updated_attributes: HashMap::new(),
            children_changes: Vec::new(),
        }
    }
}

/// DOM snapshot with efficient binary representation
#[derive(Debug, Clone)]
pub struct DomSnapshot {
    pub root: DomNode,
    pub timestamp: u64,
    pub version: u32,
    pub node_count: u32,
    pub size_in_bytes: u32,
}

impl DomSnapshot {
    /// Create a new snapshot from a DOM root
    pub fn new(root: DomNode) -> Self {
        let node_count = Self::count_nodes(&root);
        
        // Calculate size by serializing
        let mut buffer = Vec::new();
        BinaryDomSerializer::serialize(&root, &mut buffer).unwrap();
        
        DomSnapshot {
            root,
            timestamp: 0, // Will be set by the system
            version: 0,
            node_count,
            size_in_bytes: buffer.len() as u32,
        }
    }

    /// Count total nodes in the DOM tree
    fn count_nodes(node: &DomNode) -> u32 {
        1 + node.children.iter().map(Self::count_nodes).sum::<u32>()
    }

    /// Serialize snapshot to binary format
    pub fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()>
    where
        W: Write,
    {
        // Write header
        writer.write_all(b"BIOSURF-DOM")?;
        
        // Write version
        writer.write_all(&self.version.to_le_bytes())?;
        
        // Write timestamp
        writer.write_all(&self.timestamp.to_le_bytes())?;
        
        // Write node count
        writer.write_all(&self.node_count.to_le_bytes())?;
        
        // Write root node
        BinaryDomSerializer::serialize(&self.root, writer)?;
        
        Ok(())
    }

    /// Deserialize snapshot from binary format
    pub fn deserialize<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        R: Read,
    {
        // Read header
        let mut header = [0; 11]; // "BIOSURF-DOM" is 11 bytes
        reader.read_exact(&mut header)?;
        if &header != b"BIOSURF-DOM" {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid snapshot header"));
        }
        
        // Read version
        let mut version_buf = [0; 4];
        reader.read_exact(&mut version_buf)?;
        let version = u32::from_le_bytes(version_buf);
        
        // Read timestamp
        let mut timestamp_buf = [0; 8];
        reader.read_exact(&mut timestamp_buf)?;
        let timestamp = u64::from_le_bytes(timestamp_buf);
        
        // Read node count
        let mut node_count_buf = [0; 4];
        reader.read_exact(&mut node_count_buf)?;
        let node_count = u32::from_le_bytes(node_count_buf);
        
        // Read root node
        let root = BinaryDomSerializer::deserialize(reader)?;
        
        // Calculate size by serializing
        let mut buffer = Vec::new();
        BinaryDomSerializer::serialize(&root, &mut buffer)?;
        
        Ok(DomSnapshot {
            root,
            timestamp,
            version,
            node_count,
            size_in_bytes: buffer.len() as u32,
        })
    }
}

/// DOM diff generator for incremental changes
pub struct DomDiffer;

impl DomDiffer {
    /// Generate diff between two DOM snapshots
    pub fn diff(old: &DomSnapshot, new: &DomSnapshot) -> Vec<DomDiffOperation> {
        Self::diff_nodes(&old.root, &new.root)
    }

    /// Recursively diff two DOM nodes
    fn diff_nodes(old: &DomNode, new: &DomNode) -> Vec<DomDiffOperation> {
        let mut operations = Vec::new();

        // Check if nodes are the same type and tag
        if old.node_type != new.node_type || old.tag_name != new.tag_name {
            // If different types/tags, replace the entire node
            operations.push(DomDiffOperation::DeleteNode { index: 0 });
            operations.push(DomDiffOperation::InsertNode { index: 0, node: new.clone() });
            return operations;
        }

        // Handle text nodes specially
        if old.node_type == DomNodeType::Text {
            if old.text_content != new.text_content {
                operations.push(DomDiffOperation::UpdateText {
                    index: 0,
                    new_text: new.text_content.clone().unwrap_or_default(),
                });
            }
            return operations;
        }

        // Compare attributes
        let mut changes = DomChanges::default();
        
        // Check for added attributes
        for (name, value) in &new.attributes {
            if !old.attributes.contains_key(name) {
                changes.added_attributes.insert(name.clone(), value.clone());
            }
        }
        
        // Check for removed attributes
        for (name, _) in &old.attributes {
            if !new.attributes.contains_key(name) {
                changes.removed_attributes.push(name.clone());
            }
        }
        
        // Check for updated attributes
        for (name, new_value) in &new.attributes {
            if let Some(old_value) = old.attributes.get(name) {
                if old_value != new_value {
                    changes.updated_attributes.insert(name.clone(), new_value.clone());
                }
            }
        }

        // Compare children using structural diffing
        changes.children_changes = Self::diff_children(&old.children, &new.children);

        // If there are changes, add an update operation
        if !changes.added_attributes.is_empty() || 
           !changes.removed_attributes.is_empty() || 
           !changes.updated_attributes.is_empty() || 
           !changes.children_changes.is_empty() {
            operations.push(DomDiffOperation::UpdateNode { index: 0, changes });
        }

        operations
    }

    /// Diff children nodes with structural awareness
    fn diff_children(old_children: &[DomNode], new_children: &[DomNode]) -> Vec<DomDiffOperation> {
        let mut operations = Vec::new();
        let mut old_index = 0;
        let mut new_index = 0;

        // Create maps of nodes by ID if available
        let old_id_map: HashMap<u32, (usize, &DomNode)> = old_children
            .iter()
            .enumerate()
            .filter_map(|(i, node)| node.id.map(|id| (id, (i, node))))
            .collect();
        
        let new_id_map: HashMap<u32, (usize, &DomNode)> = new_children
            .iter()
            .enumerate()
            .filter_map(|(i, node)| node.id.map(|id| (id, (i, node))))
            .collect();

        // First handle nodes with matching IDs for efficient diffing
        for (id, (new_i, new_node)) in &new_id_map {
            if let Some((old_i, old_node)) = old_id_map.get(id) {
                let node_changes = Self::diff_nodes(old_node, new_node);
                for change in node_changes {
                    operations.push(change);
                }
                old_index = *old_i + 1;
                new_index = *new_i + 1;
            }
        }

        // Handle remaining nodes with structural comparison
        while old_index < old_children.len() || new_index < new_children.len() {
            if old_index >= old_children.len() {
                // All old nodes processed, insert remaining new nodes
                for node in &new_children[new_index..] {
                    operations.push(DomDiffOperation::InsertNode {
                        index: new_index,
                        node: node.clone(),
                    });
                    new_index += 1;
                }
            } else if new_index >= new_children.len() {
                // All new nodes processed, delete remaining old nodes
                for _ in old_index..old_children.len() {
                    operations.push(DomDiffOperation::DeleteNode { index: old_index });
                    old_index += 1;
                }
            } else {
                // Both have nodes left, compare them
                let old_node = &old_children[old_index];
                let new_node = &new_children[new_index];

                // Check if nodes are structurally similar
                if Self::nodes_are_similar(old_node, new_node) {
                    // Similar nodes, diff them
                    let node_changes = Self::diff_nodes(old_node, new_node);
                    for change in node_changes {
                        operations.push(change);
                    }
                    old_index += 1;
                    new_index += 1;
                } else {
                    // Different nodes, check if new node exists later in old list
                    let mut found = false;
                    for i in old_index + 1..old_children.len() {
                        if Self::nodes_are_similar(&old_children[i], new_node) {
                            // Move node from old position to new position
                            operations.push(DomDiffOperation::MoveNode {
                                from_index: i,
                                to_index: new_index,
                            });
                            old_index += 1;
                            new_index += 1;
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        // New node doesn't exist in old list, insert it
                        operations.push(DomDiffOperation::InsertNode {
                            index: new_index,
                            node: new_node.clone(),
                        });
                        new_index += 1;
                    }
                }
            }
        }

        operations
    }

    /// Check if two nodes are structurally similar for diffing
    fn nodes_are_similar(old: &DomNode, new: &DomNode) -> bool {
        if old.node_type != new.node_type || old.tag_name != new.tag_name {
            return false;
        }

        // For elements, check if they have similar structure
        if old.node_type == DomNodeType::Element {
            // Check if both have the same ID if present
            if let (Some(old_id), Some(new_id)) = (old.id, new.id) {
                return old_id == new_id;
            }

            // Check if they have similar attributes (class and id are most important for structure)
            let old_has_id = old.attributes.contains_key("id");
            let new_has_id = new.attributes.contains_key("id");
            
            let old_class = old.attributes.get("class").cloned().unwrap_or_default();
            let new_class = new.attributes.get("class").cloned().unwrap_or_default();

            // If both have IDs, they must match
            if old_has_id && new_has_id {
                return old.attributes.get("id") == new.attributes.get("id");
            }

            // If they have the same class and similar tag, consider them similar
            return !old_class.is_empty() && old_class == new_class;
        }

        // For text nodes, check if they're both text nodes
        old.node_type == new.node_type
    }
}

/// Apply diff operations to a DOM snapshot to create a new snapshot
pub struct DomPatchApplier;

impl DomPatchApplier {
    /// Apply diff operations to a snapshot
    pub fn apply(snapshot: &DomSnapshot, diff: &[DomDiffOperation]) -> DomSnapshot {
        let mut new_root = snapshot.root.clone();
        // Apply operations (simplified implementation)
        // In a real implementation, we'd recursively apply the operations
        
        DomSnapshot {
            root: new_root,
            timestamp: snapshot.timestamp + 1,
            version: snapshot.version + 1,
            node_count: snapshot.node_count, // This would be updated in a real implementation
            size_in_bytes: snapshot.size_in_bytes, // This would be updated in a real implementation
        }
    }
}
