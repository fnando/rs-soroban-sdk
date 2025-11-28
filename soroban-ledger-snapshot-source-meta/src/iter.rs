use soroban_sdk::xdr::{
    LedgerCloseMeta, LedgerEntry, LedgerEntryChange, LedgerEntryChanges, LedgerKey,
    TransactionMeta, TransactionResultMeta, TransactionResultMetaV1,
};

/// Iterator over ledger entry changes in reverse order from a LedgerCloseMeta
pub struct LedgerEntryChangesIterator<'a> {
    components: TransactionProcessingComponents<'a>,
    state: IteratorState,
}

enum IteratorState {
    Processing {
        phase: ProcessingPhase,
        change_idx: usize,
    },
    Done,
}

#[derive(Clone, Copy, Debug)]
pub enum ProcessingPhase {
    PostTxApplyFeeProcessing { tx_idx: usize },
    TxChangesAfter { tx_idx: usize },
    OperationsChanges { tx_idx: usize, op_idx: usize },
    TxChangesBefore { tx_idx: usize },
    FeeProcessing { tx_idx: usize },
}

impl ProcessingPhase {
    fn tx_idx(&self) -> usize {
        match self {
            ProcessingPhase::PostTxApplyFeeProcessing { tx_idx } => *tx_idx,
            ProcessingPhase::TxChangesAfter { tx_idx } => *tx_idx,
            ProcessingPhase::OperationsChanges { tx_idx, .. } => *tx_idx,
            ProcessingPhase::TxChangesBefore { tx_idx } => *tx_idx,
            ProcessingPhase::FeeProcessing { tx_idx } => *tx_idx,
        }
    }

    fn get_changes<'a>(
        &self,
        components: &'a TransactionProcessingComponents<'a>,
    ) -> Option<&'a LedgerEntryChanges> {
        match self {
            ProcessingPhase::PostTxApplyFeeProcessing { tx_idx } => {
                components.post_tx_apply_fee_processing(*tx_idx)
            }
            ProcessingPhase::TxChangesAfter { tx_idx } => components.tx_changes_after(*tx_idx),
            ProcessingPhase::OperationsChanges { tx_idx, op_idx } => {
                Some(components.operation_changes(*tx_idx, *op_idx))
            }
            ProcessingPhase::TxChangesBefore { tx_idx } => components.tx_changes_before(*tx_idx),
            ProcessingPhase::FeeProcessing { tx_idx } => Some(components.fee_processing(*tx_idx)),
        }
    }

    fn advance(&self, components: &TransactionProcessingComponents) -> Option<ProcessingPhase> {
        match self {
            ProcessingPhase::PostTxApplyFeeProcessing { tx_idx } => {
                if *tx_idx > 0 {
                    let prev_tx_idx = tx_idx - 1;
                    Some(ProcessingPhase::PostTxApplyFeeProcessing {
                        tx_idx: prev_tx_idx,
                    })
                } else {
                    Some(ProcessingPhase::TxChangesAfter {
                        tx_idx: components.len() - 1,
                    })
                }
            }
            ProcessingPhase::TxChangesAfter { tx_idx } => {
                if components.operation_count(*tx_idx) > 0 {
                    Some(ProcessingPhase::OperationsChanges {
                        tx_idx: *tx_idx,
                        op_idx: components.operation_count(*tx_idx).saturating_sub(1),
                    })
                } else {
                    Some(ProcessingPhase::TxChangesBefore { tx_idx: *tx_idx })
                }
            }
            ProcessingPhase::OperationsChanges { tx_idx, op_idx } => {
                if *op_idx > 0 {
                    let prev_op_idx = op_idx - 1;
                    Some(ProcessingPhase::OperationsChanges {
                        tx_idx: *tx_idx,
                        op_idx: prev_op_idx,
                    })
                } else {
                    Some(ProcessingPhase::TxChangesBefore { tx_idx: *tx_idx })
                }
            }
            ProcessingPhase::TxChangesBefore { tx_idx } => {
                if *tx_idx > 0 {
                    let prev_tx_idx = tx_idx - 1;
                    Some(ProcessingPhase::TxChangesAfter {
                        tx_idx: prev_tx_idx,
                    })
                } else {
                    Some(ProcessingPhase::FeeProcessing {
                        tx_idx: components.len() - 1,
                    })
                }
            }
            ProcessingPhase::FeeProcessing { tx_idx } => {
                if *tx_idx > 0 {
                    let prev_tx_idx = tx_idx - 1;
                    Some(ProcessingPhase::FeeProcessing {
                        tx_idx: prev_tx_idx,
                    })
                } else {
                    None
                }
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
    pub fn new(meta: &'a LedgerCloseMeta, skip_post_tx_fee: bool) -> Self {
        let components = TransactionProcessingComponents::from(meta);
        let len = components.len();
        if len > 0 {
            let initial_phase = if skip_post_tx_fee {
                ProcessingPhase::TxChangesAfter { tx_idx: len - 1 }
            } else {
                ProcessingPhase::PostTxApplyFeeProcessing { tx_idx: len - 1 }
            };
            Self {
                components,
                state: IteratorState::Processing {
                    phase: initial_phase,
                    change_idx: 0,
                },
            }
        } else {
            Self {
                components,
                state: IteratorState::Done,
            }
        }
    }
}

impl<'a> Iterator for LedgerEntryChangesIterator<'a> {
    type Item = ([u8; 32], LedgerKey, Option<LedgerEntry>, ProcessingPhase);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match &mut self.state {
                IteratorState::Processing { phase, change_idx } => {
                    // Try to get changes from current phase
                    if let Some(changes) = phase.get_changes(&self.components) {
                        // If we've processed all changes in this phase
                        if *change_idx >= changes.len() {
                            // Move to next phase
                            if let Some(next_phase) = phase.advance(&self.components) {
                                *phase = next_phase;
                                *change_idx = 0;
                                continue;
                            } else {
                                // No more phases
                                self.state = IteratorState::Done;
                                continue;
                            }
                        }

                        // Get the change in reverse order
                        let change = &changes[changes.len() - 1 - *change_idx];
                        let (key, entry) = extract_key_entry(change);

                        *change_idx += 1;
                        let tx_idx = phase.tx_idx();
                        let hash = *self.components.tx_hash(tx_idx);
                        return Some((hash, key, entry, *phase));
                    } else {
                        // This phase has no changes, move to next phase
                        if let Some(next_phase) = phase.advance(&self.components) {
                            *phase = next_phase;
                            *change_idx = 0;
                            continue;
                        } else {
                            // No more phases
                            self.state = IteratorState::Done;
                            continue;
                        }
                    }
                }
                IteratorState::Done => return None,
            }
        }
    }
}

/// Extracted transaction processing components from LedgerCloseMeta
pub enum TransactionProcessingComponents<'a> {
    V0(&'a [TransactionResultMeta]),
    V1(&'a [TransactionResultMetaV1]),
}

impl<'a> From<&'a LedgerCloseMeta> for TransactionProcessingComponents<'a> {
    fn from(meta: &'a LedgerCloseMeta) -> Self {
        match meta {
            LedgerCloseMeta::V0(meta_v0) => Self::V0(&meta_v0.tx_processing),
            LedgerCloseMeta::V1(meta_v1) => Self::V0(&meta_v1.tx_processing),
            LedgerCloseMeta::V2(meta_v2) => Self::V1(&meta_v2.tx_processing),
        }
    }
}

impl<'a> TransactionProcessingComponents<'a> {
    pub fn len(&self) -> usize {
        match self {
            TransactionProcessingComponents::V0(slice) => slice.len(),
            TransactionProcessingComponents::V1(slice) => slice.len(),
        }
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
            TransactionMeta::V1(tx_meta) => Some(&tx_meta.tx_changes),
            TransactionMeta::V2(tx_meta) => Some(&tx_meta.tx_changes_before),
            TransactionMeta::V3(tx_meta) => Some(&tx_meta.tx_changes_before),
            TransactionMeta::V4(tx_meta) => Some(&tx_meta.tx_changes_before),
        }
    }

    /// Get the number of operations for a transaction from any TransactionMeta version
    pub fn operation_count(&self, tx_index: usize) -> usize {
        match self.tx_apply_processing(tx_index) {
            TransactionMeta::V0(operations) => operations.len(),
            TransactionMeta::V1(tx_meta) => tx_meta.operations.len(),
            TransactionMeta::V2(tx_meta) => tx_meta.operations.len(),
            TransactionMeta::V3(tx_meta) => tx_meta.operations.len(),
            TransactionMeta::V4(tx_meta) => tx_meta.operations.len(),
        }
    }

    /// Extract changes for a specific operation from any TransactionMeta version
    pub fn operation_changes(&self, tx_index: usize, op_index: usize) -> &'a LedgerEntryChanges {
        match self.tx_apply_processing(tx_index) {
            TransactionMeta::V0(operations) => &operations[op_index].changes,
            TransactionMeta::V1(tx_meta) => &tx_meta.operations[op_index].changes,
            TransactionMeta::V2(tx_meta) => &tx_meta.operations[op_index].changes,
            TransactionMeta::V3(tx_meta) => &tx_meta.operations[op_index].changes,
            TransactionMeta::V4(tx_meta) => &tx_meta.operations[op_index].changes,
        }
    }

    fn tx_apply_processing(&self, index: usize) -> &'a TransactionMeta {
        match self {
            Self::V0(slice) => &slice[index].tx_apply_processing,
            Self::V1(slice) => &slice[index].tx_apply_processing,
        }
    }

    /// Extract tx_changes_after from any TransactionMeta version
    pub fn tx_changes_after(&self, index: usize) -> Option<&'a LedgerEntryChanges> {
        match self.tx_apply_processing(index) {
            TransactionMeta::V0(_) => None,
            TransactionMeta::V1(_tx_meta) => None,
            TransactionMeta::V2(tx_meta) => Some(&tx_meta.tx_changes_after),
            TransactionMeta::V3(tx_meta) => Some(&tx_meta.tx_changes_after),
            TransactionMeta::V4(tx_meta) => Some(&tx_meta.tx_changes_after),
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
