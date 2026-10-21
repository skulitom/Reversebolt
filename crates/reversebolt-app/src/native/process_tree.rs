//! A bounded ancestry inventory, not proof that a descendant owns gameplay state.
use reversebolt_core::{Result, live::ProcessInfo};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn descendants(
    root: &ProcessInfo,
    parents: &BTreeMap<u32, u32>,
    mut resolve: impl FnMut(u32) -> Result<ProcessInfo>,
) -> Result<Vec<ProcessInfo>> {
    let mut result = vec![root.clone()];
    let mut pending = VecDeque::from([root.clone()]);
    let mut visited = BTreeSet::from([root.pid]);
    let mut attempts = 1;
    while let Some(parent) = pending.pop_front() {
        let parent_time = u64::from_str_radix(&parent.created, 16)
            .map_err(|_| "Invalid parent creation identity.")?;
        for (&pid, _) in parents.iter().filter(|(_, p)| **p == parent.pid) {
            if !visited.insert(pid) {
                continue;
            }
            attempts += 1;
            if attempts > 64 {
                return Err(
                    "The application family exceeded the 64-process inventory budget.".into(),
                );
            }
            // An inaccessible/exited ancestor leaves its whole branch unknown.
            let Ok(child) = resolve(pid) else { continue };
            let child_time = u64::from_str_radix(&child.created, 16)
                .map_err(|_| "Invalid child creation identity.")?;
            if child.pid != pid || child.session != root.session || child_time < parent_time {
                continue;
            }
            pending.push_back(child.clone());
            result.push(child);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn info(pid: u32, created: u64, session: u32) -> ProcessInfo {
        ProcessInfo {
            pid,
            created: format!("{created:016X}"),
            session,
            ..Default::default()
        }
    }
    #[test]
    fn ancestry_excludes_reused_parent_ids_other_sessions_and_unrelated_processes() {
        let parents = BTreeMap::from([
            (2, 1),
            (3, 2),
            (4, 1),
            (5, 1),
            (6, 99),
            (7, 4),
            (8, 1),
            (9, 8),
        ]);
        let entries = BTreeMap::from([
            (2, info(2, 11, 1)),
            (3, info(3, 12, 1)),
            (4, info(4, 9, 1)), // Predates the current root: parent PID was reused.
            (5, info(5, 11, 2)),
            (6, info(6, 13, 1)),
            (7, info(7, 14, 1)),
            (9, info(9, 15, 1)), // Its direct parent cannot be inspected.
        ]);
        let found = descendants(&info(1, 10, 1), &parents, |pid| {
            entries.get(&pid).cloned().ok_or("unavailable".into())
        })
        .unwrap();
        assert_eq!(found.iter().map(|p| p.pid).collect::<Vec<_>>(), [1, 2, 3]);
    }
    #[test]
    fn cycles_are_bounded_and_oversized_families_are_explicit_errors() {
        let root = info(1, 10, 1);
        let cycle = BTreeMap::from([(2, 1), (1, 2)]);
        assert_eq!(
            descendants(&root, &cycle, |pid| Ok(info(pid, 10, 1)))
                .unwrap()
                .len(),
            2
        );
        let large = (2..=66).map(|pid| (pid, 1)).collect();
        assert!(descendants(&root, &large, |pid| Ok(info(pid, 11, 1))).is_err());
    }
}
