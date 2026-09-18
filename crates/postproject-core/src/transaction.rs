//! Backend-neutral transaction state semantics.

use crate::{Error, ErrorKind, Result, TransactionId};

/// The state of an explicit domain transaction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TransactionState {
    /// Mutations may still be staged.
    Open,
    /// All staged mutations were atomically persisted.
    Committed,
    /// Staged mutations were discarded.
    RolledBack,
}

/// A small state machine shared by storage-backed domain transactions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransactionLifecycle {
    id: TransactionId,
    state: TransactionState,
}

impl TransactionLifecycle {
    /// Creates a new open transaction with a stable identity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: TransactionId::new(),
            state: TransactionState::Open,
        }
    }

    /// Creates an open transaction with an existing identity.
    #[must_use]
    pub const fn with_id(id: TransactionId) -> Self {
        Self {
            id,
            state: TransactionState::Open,
        }
    }

    /// Returns the transaction identity.
    #[must_use]
    pub const fn id(self) -> TransactionId {
        self.id
    }

    /// Returns the current state.
    #[must_use]
    pub const fn state(self) -> TransactionState {
        self.state
    }

    /// Ensures that a mutation may be staged.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if the transaction is already closed.
    pub fn ensure_open(self) -> Result<()> {
        if self.state == TransactionState::Open {
            Ok(())
        } else {
            Err(self.closed_error())
        }
    }

    /// Marks the transaction committed after the backend commit succeeds.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if the transaction is already closed.
    pub fn mark_committed(&mut self) -> Result<()> {
        self.ensure_open()?;
        self.state = TransactionState::Committed;
        Ok(())
    }

    /// Marks the transaction rolled back after staged work is discarded.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Conflict`] if the transaction is already closed.
    pub fn mark_rolled_back(&mut self) -> Result<()> {
        self.ensure_open()?;
        self.state = TransactionState::RolledBack;
        Ok(())
    }

    fn closed_error(self) -> Error {
        Error::new(
            ErrorKind::Conflict,
            format!("transaction {} is already {:?}", self.id, self.state),
        )
    }
}

impl Default for TransactionLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_closes_transaction() {
        let mut lifecycle = TransactionLifecycle::new();
        lifecycle
            .mark_committed()
            .expect("open transaction commits");

        assert_eq!(lifecycle.state(), TransactionState::Committed);
        assert_eq!(
            lifecycle
                .mark_rolled_back()
                .expect_err("committed transaction stays closed")
                .kind(),
            ErrorKind::Conflict
        );
    }

    #[test]
    fn rollback_closes_transaction() {
        let mut lifecycle = TransactionLifecycle::new();
        lifecycle
            .mark_rolled_back()
            .expect("open transaction rolls back");

        assert_eq!(lifecycle.state(), TransactionState::RolledBack);
        assert_eq!(
            lifecycle
                .mark_committed()
                .expect_err("rolled-back transaction stays closed")
                .kind(),
            ErrorKind::Conflict
        );
    }
}
