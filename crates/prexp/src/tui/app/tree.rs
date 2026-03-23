use std::collections::HashMap;

use prexp_core::models::ProcessSnapshot;

use super::{ProcessSortField, SortDirection, TreeEntry};

/// Build a tree-ordered list, sorting only root processes by the given field.
/// Children remain grouped under their parent, sorted by PID.
pub fn build_process_tree_sorted(
    snapshots: &[ProcessSnapshot],
    sort_field: ProcessSortField,
    sort_dir: SortDirection,
) -> Vec<TreeEntry> {
    if snapshots.is_empty() {
        return Vec::new();
    }

    let pid_to_idx: HashMap<i32, usize> = snapshots
        .iter()
        .enumerate()
        .map(|(i, s)| (s.pid, i))
        .collect();

    let mut children: HashMap<i32, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();

    for (i, snap) in snapshots.iter().enumerate() {
        if snap.ppid == 0 || !pid_to_idx.contains_key(&snap.ppid) {
            roots.push(i);
        } else {
            children.entry(snap.ppid).or_default().push(i);
        }
    }

    sort_indices_by_field(&mut roots, snapshots, sort_field, sort_dir);
    for kids in children.values_mut() {
        kids.sort_by_key(|&i| snapshots[i].pid);
    }

    let mut result = Vec::with_capacity(snapshots.len());
    for &root_idx in &roots {
        walk_tree(snapshots, &children, root_idx, 0, String::new(), true, &mut result);
    }

    result
}

fn sort_indices_by_field(
    indices: &mut [usize],
    snapshots: &[ProcessSnapshot],
    field: ProcessSortField,
    dir: SortDirection,
) {
    match field {
        ProcessSortField::Unsorted => {
            indices.sort_by_key(|&i| snapshots[i].pid);
        }
        _ => {
            indices.sort_by(|&a, &b| {
                let sa = &snapshots[a];
                let sb = &snapshots[b];
                let cmp = match field {
                    ProcessSortField::Pid => sa.pid.cmp(&sb.pid),
                    ProcessSortField::Name => sa.name.to_lowercase().cmp(&sb.name.to_lowercase()),
                    ProcessSortField::Total => sa.resources.len().cmp(&sb.resources.len()),
                    ProcessSortField::Unsorted => unreachable!(),
                };
                match dir {
                    SortDirection::Asc => cmp,
                    SortDirection::Desc => cmp.reverse(),
                }
            });
        }
    }
}

fn walk_tree(
    snapshots: &[ProcessSnapshot],
    children: &HashMap<i32, Vec<usize>>,
    idx: usize,
    depth: usize,
    parent_prefix: String,
    is_last: bool,
    result: &mut Vec<TreeEntry>,
) {
    let prefix = if depth == 0 {
        String::new()
    } else {
        let connector = if is_last { "└── " } else { "├── " };
        format!("{}{}", parent_prefix, connector)
    };

    result.push(TreeEntry {
        snapshot_index: idx,
        depth,
        prefix: prefix.clone(),
    });

    let pid = snapshots[idx].pid;
    if let Some(kids) = children.get(&pid) {
        let child_prefix = if depth == 0 {
            String::new()
        } else {
            let continuation = if is_last { "    " } else { "│   " };
            format!("{}{}", parent_prefix, continuation)
        };

        for (i, &child_idx) in kids.iter().enumerate() {
            let child_is_last = i == kids.len() - 1;
            walk_tree(
                snapshots, children, child_idx, depth + 1, child_prefix.clone(), child_is_last, result,
            );
        }
    }
}
