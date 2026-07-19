use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A conservative resource claim calculated before a tool may execute.
///
/// The prepared-call seam only records these claims. The product scheduler
/// continues to use its established Boolean policy until resource-aware
/// batching lands.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceClaim {
    ReadPath(PathBuf),
    WritePath(PathBuf),
    ReadTree(PathBuf),
    WriteTree(PathBuf),
    Terminal(String),
    GlobalExclusive,
}

impl ResourceClaim {
    #[must_use]
    pub fn conflicts_with(&self, other: &Self) -> bool {
        use ResourceClaim::{GlobalExclusive, ReadPath, ReadTree, Terminal, WritePath, WriteTree};

        match (self, other) {
            (GlobalExclusive, _) | (_, GlobalExclusive) => true,
            (Terminal(left), Terminal(right)) => left == right,
            (ReadPath(_), ReadPath(_)) => false,
            (ReadPath(left), WritePath(right)) | (WritePath(right), ReadPath(left)) => {
                left == right
            }
            (WritePath(left), WritePath(right)) => left == right,
            (ReadTree(_), ReadTree(_)) => false,
            (ReadTree(_), ReadPath(_)) | (ReadPath(_), ReadTree(_)) => false,
            (ReadTree(tree), WritePath(path)) | (WritePath(path), ReadTree(tree)) => {
                path.starts_with(tree)
            }
            (WriteTree(tree), ReadPath(path)) | (ReadPath(path), WriteTree(tree)) => {
                path.starts_with(tree)
            }
            (WriteTree(tree), WritePath(path)) | (WritePath(path), WriteTree(tree)) => {
                path.starts_with(tree)
            }
            (ReadTree(left), WriteTree(right)) | (WriteTree(right), ReadTree(left)) => {
                trees_overlap(left, right)
            }
            (WriteTree(left), WriteTree(right)) => trees_overlap(left, right),
            _ => false,
        }
    }
}

fn trees_overlap(left: &Path, right: &Path) -> bool {
    left.starts_with(right) || right.starts_with(left)
}

/// Build deterministic parallel batches. Items with no conflicting resource
/// claims share a batch; conflicting items retain their original order.
#[must_use]
pub fn schedule_non_conflicting<T>(items: Vec<(T, Vec<ResourceClaim>)>) -> Vec<Vec<T>> {
    let mut batches: Vec<(Vec<T>, Vec<ResourceClaim>)> = Vec::new();
    for (item, claims) in items {
        if let Some((batch, batch_claims)) = batches.iter_mut().find(|(_, batch_claims)| {
            !claims.iter().any(|claim| {
                batch_claims
                    .iter()
                    .any(|existing| claim.conflicts_with(existing))
            })
        }) {
            batch.push(item);
            batch_claims.extend(claims);
        } else {
            batches.push((vec![item], claims));
        }
    }
    batches.into_iter().map(|(items, _)| items).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_reads_share_a_batch_but_write_is_ordered() {
        let path = PathBuf::from("src/lib.rs");
        let batches = schedule_non_conflicting(vec![
            ("read-a", vec![ResourceClaim::ReadPath(path.clone())]),
            ("read-b", vec![ResourceClaim::ReadPath(path.clone())]),
            ("write", vec![ResourceClaim::WritePath(path)]),
        ]);
        assert_eq!(batches, vec![vec!["read-a", "read-b"], vec!["write"]]);
    }

    #[test]
    fn unrelated_writes_can_run_together() {
        let batches = schedule_non_conflicting(vec![
            ("a", vec![ResourceClaim::WritePath(PathBuf::from("a.rs"))]),
            ("b", vec![ResourceClaim::WritePath(PathBuf::from("b.rs"))]),
        ]);
        assert_eq!(batches, vec![vec!["a", "b"]]);
    }

    #[test]
    fn global_exclusive_conflicts_with_every_claim() {
        for claim in [
            ResourceClaim::ReadPath(PathBuf::from("src/lib.rs")),
            ResourceClaim::Terminal("shell-1".to_string()),
            ResourceClaim::GlobalExclusive,
        ] {
            assert!(ResourceClaim::GlobalExclusive.conflicts_with(&claim));
            assert!(claim.conflicts_with(&ResourceClaim::GlobalExclusive));
        }
    }
}
