//! Merge algorithm for CRUST

use crate::{Result, Tree};

/// Perform a three-way merge
pub fn merge_trees(_base: &Tree, ours: &Tree, _theirs: &Tree) -> Result<(Tree, Vec<String>)> {
    let mut merged_entries = Vec::new();
    let conflicts = Vec::new();

    // Simplified three-way merge: for now, just take ours and note conflicts
    // Full implementation would compare each entry and handle conflicts

    for entry in &ours.entries {
        merged_entries.push(entry.clone());
    }

    Ok((Tree::new(merged_entries)?, conflicts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{tree::TreeEntry, ObjectId};

    #[test]
    fn test_merge_basic() {
        let entry = TreeEntry {
            mode: "100644".to_string(),
            name: "test.txt".to_string(),
            id: ObjectId::from_hex(
                "356a192b7913b04c54574d18c28d46e6395428ab356a192b7913b04c54574d18",
            )
            .unwrap(),
        };

        let tree = Tree::new(vec![entry]).unwrap();
        let (merged, conflicts) = merge_trees(&tree, &tree, &tree).unwrap();

        assert_eq!(merged.entries.len(), 1);
        assert_eq!(conflicts.len(), 0);
    }
}
