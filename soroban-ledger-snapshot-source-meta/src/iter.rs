use soroban_sdk::xdr::{
    LedgerCloseMeta, LedgerEntry, LedgerEntryChange, LedgerEntryChanges, LedgerKey,
    TransactionMeta, TransactionResultMeta, TransactionResultMetaV1,
};

/// Iterator over ledger entry changes in reverse order from a LedgerCloseMeta.
/// Within each phase, non-State changes are yielded first, then State changes.
pub struct LedgerEntryChangesIterator<'a> {
    tx_result_meta: TransactionResultMetaNormalized<'a>,
    /// Current iteration position, or None if done
    position: Option<IteratorPosition>,
}

/// Current position within the iteration
#[derive(Clone, Copy, Debug)]
struct IteratorPosition {
    phase: ProcessingPhase,
    change_idx: usize,
}

/// Groups LedgerEntryChange variants by whether they represent state before or after changes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LedgerEntryChangeGroup {
    /// State before changes: State.
    Before,
    /// Changes after: Created, Updated, Restored, Removed.
    After,
}

#[derive(Clone, Copy, Debug)]
pub enum ProcessingPhase {
    PostTxApplyFeeProcessing { tx_idx: usize, group: LedgerEntryChangeGroup },
    TxChangesAfter { tx_idx: usize, group: LedgerEntryChangeGroup },
    OperationsChanges { tx_idx: usize, op_idx: usize, group: LedgerEntryChangeGroup },
    TxChangesBefore { tx_idx: usize, group: LedgerEntryChangeGroup },
    FeeProcessing { tx_idx: usize, group: LedgerEntryChangeGroup },
}

impl ProcessingPhase {
    fn first(tx_idx: usize) -> Self {
        Self::PostTxApplyFeeProcessing { tx_idx, group: LedgerEntryChangeGroup::After }
    }

    fn tx_idx(&self) -> usize {
        match self {
            Self::PostTxApplyFeeProcessing { tx_idx, .. }
            | Self::TxChangesAfter { tx_idx, .. }
            | Self::OperationsChanges { tx_idx, .. }
            | Self::TxChangesBefore { tx_idx, .. }
            | Self::FeeProcessing { tx_idx, .. } => *tx_idx,
        }
    }

    fn group(&self) -> LedgerEntryChangeGroup {
        match self {
            Self::PostTxApplyFeeProcessing { group, .. }
            | Self::TxChangesAfter { group, .. }
            | Self::OperationsChanges { group, .. }
            | Self::TxChangesBefore { group, .. }
            | Self::FeeProcessing { group, .. } => *group,
        }
    }

    fn get_changes<'a>(
        &self,
        components: &'a TransactionResultMetaNormalized<'a>,
    ) -> Option<&'a LedgerEntryChanges> {
        match self {
            Self::PostTxApplyFeeProcessing { tx_idx, .. } => {
                components.post_tx_apply_fee_processing(*tx_idx)
            }
            Self::TxChangesAfter { tx_idx, .. } => components.tx_changes_after(*tx_idx),
            Self::OperationsChanges { tx_idx, op_idx, .. } => {
                Some(components.operation_changes(*tx_idx, *op_idx))
            }
            Self::TxChangesBefore { tx_idx, .. } => components.tx_changes_before(*tx_idx),
            Self::FeeProcessing { tx_idx, .. } => Some(components.fee_processing(*tx_idx)),
        }
    }

    fn advance(&self, components: &TransactionResultMetaNormalized) -> Option<ProcessingPhase> {
        let tx_count = components.len();
        match self {
            Self::PostTxApplyFeeProcessing { tx_idx, group: LedgerEntryChangeGroup::After } => {
                Some(Self::PostTxApplyFeeProcessing { tx_idx: *tx_idx, group: LedgerEntryChangeGroup::Before })
            }
            Self::PostTxApplyFeeProcessing { tx_idx, group: LedgerEntryChangeGroup::Before } => {
                Some(tx_idx
                    .checked_sub(1)
                    .map_or(Self::TxChangesAfter { tx_idx: tx_count - 1, group: LedgerEntryChangeGroup::After }, |i| {
                        Self::PostTxApplyFeeProcessing { tx_idx: i, group: LedgerEntryChangeGroup::After }
                    }))
            }
            Self::TxChangesAfter { tx_idx, group: LedgerEntryChangeGroup::After } => {
                Some(Self::TxChangesAfter { tx_idx: *tx_idx, group: LedgerEntryChangeGroup::Before })
            }
            Self::TxChangesAfter { tx_idx, group: LedgerEntryChangeGroup::Before } => {
                let op_count = components.operation_count(*tx_idx);
                Some(if op_count > 0 {
                    Self::OperationsChanges {
                        tx_idx: *tx_idx,
                        op_idx: op_count - 1,
                        group: LedgerEntryChangeGroup::After,
                    }
                } else {
                    Self::TxChangesBefore { tx_idx: *tx_idx, group: LedgerEntryChangeGroup::After }
                })
            }
            Self::OperationsChanges { tx_idx, op_idx, group: LedgerEntryChangeGroup::After } => {
                Some(Self::OperationsChanges { tx_idx: *tx_idx, op_idx: *op_idx, group: LedgerEntryChangeGroup::Before })
            }
            Self::OperationsChanges { tx_idx, op_idx, group: LedgerEntryChangeGroup::Before } => {
                Some(op_idx
                    .checked_sub(1)
                    .map_or(Self::TxChangesBefore { tx_idx: *tx_idx, group: LedgerEntryChangeGroup::After }, |i| {
                        Self::OperationsChanges {
                            tx_idx: *tx_idx,
                            op_idx: i,
                            group: LedgerEntryChangeGroup::After,
                        }
                    }))
            }
            Self::TxChangesBefore { tx_idx, group: LedgerEntryChangeGroup::After } => {
                Some(Self::TxChangesBefore { tx_idx: *tx_idx, group: LedgerEntryChangeGroup::Before })
            }
            Self::TxChangesBefore { tx_idx, group: LedgerEntryChangeGroup::Before } => {
                Some(tx_idx
                    .checked_sub(1)
                    .map_or(Self::FeeProcessing { tx_idx: tx_count - 1, group: LedgerEntryChangeGroup::After }, |i| {
                        Self::TxChangesAfter { tx_idx: i, group: LedgerEntryChangeGroup::After }
                    }))
            }
            Self::FeeProcessing { tx_idx, group: LedgerEntryChangeGroup::After } => {
                Some(Self::FeeProcessing { tx_idx: *tx_idx, group: LedgerEntryChangeGroup::Before })
            }
            Self::FeeProcessing { tx_idx, group: LedgerEntryChangeGroup::Before } => {
                tx_idx
                    .checked_sub(1)
                    .map(|i| Self::FeeProcessing { tx_idx: i, group: LedgerEntryChangeGroup::After })
            }
        }
    }
}

/// Extract the key and entry from a ledger entry change
fn extract_key_entry(change: &LedgerEntryChange) -> (LedgerKey, Option<LedgerEntry>) {
    match change {
        LedgerEntryChange::Created(ledger_entry)
        | LedgerEntryChange::Updated(ledger_entry)
        | LedgerEntryChange::State(ledger_entry)
        | LedgerEntryChange::Restored(ledger_entry) => {
            (ledger_entry.to_key(), Some(ledger_entry.clone()))
        }
        LedgerEntryChange::Removed(ledger_key) => (ledger_key.clone(), None),
    }
}

impl<'a> LedgerEntryChangesIterator<'a> {
    /// Create a new iterator over ledger entry changes
    pub fn new(meta: &'a LedgerCloseMeta) -> Self {
        let tx_result_meta = TransactionResultMetaNormalized::from(meta);
        let len = tx_result_meta.len();
        let position = (len > 0).then(|| IteratorPosition {
            phase: ProcessingPhase::first(len - 1),
            change_idx: 0,
        });
        Self { tx_result_meta, position }
    }
}

impl<'a> Iterator for LedgerEntryChangesIterator<'a> {
    type Item = (ProcessingPhase, [u8; 32], LedgerKey, Option<LedgerEntry>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let pos = self.position.as_mut()?;

            let Some(changes) = pos.phase.get_changes(&self.tx_result_meta) else {
                // No changes in this phase, advance to next phase
                self.position = pos.phase.advance(&self.tx_result_meta).map(|phase| {
                    IteratorPosition {
                        phase,
                        change_idx: 0,
                    }
                });
                continue;
            };

            // If we've processed all changes in this phase for current group
            if pos.change_idx >= changes.len() {
                // Advance to next phase (which handles After -> Before transition)
                self.position = pos.phase.advance(&self.tx_result_meta).map(|phase| {
                    IteratorPosition {
                        phase,
                        change_idx: 0,
                    }
                });
                continue;
            }

            // Get the change in reverse order
            let change = &changes[changes.len() - 1 - pos.change_idx];

            // Check if this change matches the current group
            let is_before = matches!(change, LedgerEntryChange::State(_));
            let should_yield = match pos.phase.group() {
                LedgerEntryChangeGroup::After => !is_before,
                LedgerEntryChangeGroup::Before => is_before,
            };

            if !should_yield {
                // Skip this change, it belongs to the other group
                pos.change_idx += 1;
                continue;
            }

            let phase = pos.phase;
            pos.change_idx += 1;
            let hash = *self.tx_result_meta.tx_hash(phase.tx_idx());
            let (key, entry) = extract_key_entry(change);
            return Some((phase, hash, key, entry));
        }
    }
}

/// Extracted transaction processing components from LedgerCloseMeta
enum TransactionResultMetaNormalized<'a> {
    V0(&'a [TransactionResultMeta]),
    V1(&'a [TransactionResultMetaV1]),
}

impl<'a> From<&'a LedgerCloseMeta> for TransactionResultMetaNormalized<'a> {
    fn from(meta: &'a LedgerCloseMeta) -> Self {
        match meta {
            LedgerCloseMeta::V0(meta_v0) => Self::V0(&meta_v0.tx_processing),
            LedgerCloseMeta::V1(meta_v1) => Self::V0(&meta_v1.tx_processing),
            LedgerCloseMeta::V2(meta_v2) => Self::V1(&meta_v2.tx_processing),
        }
    }
}

impl<'a> TransactionResultMetaNormalized<'a> {
    pub fn len(&self) -> usize {
        match self {
            TransactionResultMetaNormalized::V0(slice) => slice.len(),
            TransactionResultMetaNormalized::V1(slice) => slice.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn fee_processing(&self, index: usize) -> &'a LedgerEntryChanges {
        match self {
            Self::V0(slice) => &slice[index].fee_processing,
            Self::V1(slice) => &slice[index].fee_processing,
        }
    }

    /// Extract tx_changes_before from any TransactionMeta version
    pub fn tx_changes_before(&self, index: usize) -> Option<&'a LedgerEntryChanges> {
        match self.tx_apply_processing(index) {
            TransactionMeta::V0(_) => None,
            TransactionMeta::V1(m) => Some(&m.tx_changes),
            TransactionMeta::V2(m) => Some(&m.tx_changes_before),
            TransactionMeta::V3(m) => Some(&m.tx_changes_before),
            TransactionMeta::V4(m) => Some(&m.tx_changes_before),
        }
    }

    /// Get the number of operations for a transaction from any TransactionMeta version
    pub fn operation_count(&self, tx_index: usize) -> usize {
        match self.tx_apply_processing(tx_index) {
            TransactionMeta::V0(ops) => ops.len(),
            TransactionMeta::V1(m) => m.operations.len(),
            TransactionMeta::V2(m) => m.operations.len(),
            TransactionMeta::V3(m) => m.operations.len(),
            TransactionMeta::V4(m) => m.operations.len(),
        }
    }

    /// Extract changes for a specific operation from any TransactionMeta version
    pub fn operation_changes(&self, tx_index: usize, op_index: usize) -> &'a LedgerEntryChanges {
        match self.tx_apply_processing(tx_index) {
            TransactionMeta::V0(ops) => &ops[op_index].changes,
            TransactionMeta::V1(m) => &m.operations[op_index].changes,
            TransactionMeta::V2(m) => &m.operations[op_index].changes,
            TransactionMeta::V3(m) => &m.operations[op_index].changes,
            TransactionMeta::V4(m) => &m.operations[op_index].changes,
        }
    }

    fn tx_apply_processing(&self, index: usize) -> &'a TransactionMeta {
        match self {
            Self::V0(s) => &s[index].tx_apply_processing,
            Self::V1(s) => &s[index].tx_apply_processing,
        }
    }

    /// Extract tx_changes_after from any TransactionMeta version
    pub fn tx_changes_after(&self, index: usize) -> Option<&'a LedgerEntryChanges> {
        match self.tx_apply_processing(index) {
            TransactionMeta::V0(_) => None,
            TransactionMeta::V1(_) => None,
            TransactionMeta::V2(m) => Some(&m.tx_changes_after),
            TransactionMeta::V3(m) => Some(&m.tx_changes_after),
            TransactionMeta::V4(m) => Some(&m.tx_changes_after),
        }
    }

    pub fn post_tx_apply_fee_processing(&self, index: usize) -> Option<&'a LedgerEntryChanges> {
        match self {
            Self::V0(_) => None,
            Self::V1(slice) => Some(&slice[index].post_tx_apply_fee_processing),
        }
    }

    pub fn tx_hash(&self, index: usize) -> &[u8; 32] {
        match self {
            Self::V0(slice) => &slice[index].result.transaction_hash.0,
            Self::V1(slice) => &slice[index].result.transaction_hash.0,
        }
    }
}
