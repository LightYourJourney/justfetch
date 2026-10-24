//! Pure derivations for the Windows adapter, kept free of native calls so they
//! can be tested on any host. Windows reports a commit limit rather than a
//! page-file usage figure, so the number has to be derived and is reported as
//! estimated.

use crate::model::SwapInfo;

/// What a set of `GetPerformanceInfo` counters implies about the page file.
#[derive(Debug, PartialEq, Eq)]
pub enum DerivedSwap {
    /// The commit limit does not exceed physical memory, which is what a
    /// machine without a page file reports. Zero is then the true answer, not
    /// a missing one.
    NoPageFile,
    Swap(SwapInfo),
    /// The counters do not describe a page file this code can report.
    Inconsistent,
}

/// Derive swap from `GetPerformanceInfo`, in bytes.
///
/// Committing beyond physical memory is what a page file is for, so the commit
/// limit above physical memory is its size, and committed memory above what is
/// resident is what it is holding.
pub fn swap(limit: u64, physical: u64, committed: u64, available: u64) -> DerivedSwap {
    if limit <= physical {
        return DerivedSwap::NoPageFile;
    }
    let total_bytes = limit - physical;
    let Some(resident) = physical.checked_sub(available) else {
        return DerivedSwap::Inconsistent;
    };
    let Some(used_bytes) = committed.checked_sub(resident) else {
        return DerivedSwap::Inconsistent;
    };
    if used_bytes > total_bytes {
        return DerivedSwap::Inconsistent;
    }
    DerivedSwap::Swap(SwapInfo {
        total_bytes,
        used_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_commit_limit_at_or_below_physical_memory_means_no_page_file() {
        // This is the shape the GitHub Windows runner reports.
        assert_eq!(swap(16, 16, 8, 4), DerivedSwap::NoPageFile);
        assert_eq!(swap(15, 16, 8, 4), DerivedSwap::NoPageFile);
    }

    #[test]
    fn the_page_file_is_the_commit_limit_above_physical_memory() {
        assert_eq!(
            swap(20, 16, 10, 8),
            DerivedSwap::Swap(SwapInfo {
                total_bytes: 4,
                used_bytes: 2,
            })
        );
    }

    #[test]
    fn counters_that_do_not_describe_a_page_file_are_rejected() {
        // Committed below what is resident.
        assert_eq!(swap(20, 16, 2, 8), DerivedSwap::Inconsistent);
        // More committed than the page file could hold.
        assert_eq!(swap(20, 16, 30, 8), DerivedSwap::Inconsistent);
        // Available memory above physical memory.
        assert_eq!(swap(20, 16, 10, 17), DerivedSwap::Inconsistent);
    }
}
