use crate::{Command, apply_command};
use peachy_document::Document;

pub struct Session {
    document: Document,
    undo: Vec<Document>,
    redo: Vec<Document>,
    saved: Document,
    transaction: Option<Document>,
}

impl Session {
    pub fn new(document: Document) -> Result<Self, String> {
        document.validate()?;
        Ok(Self {
            saved: document.clone(),
            document,
            undo: vec![],
            redo: vec![],
            transaction: None,
        })
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn is_dirty(&self) -> bool {
        self.document != self.saved
    }
    pub fn mark_saved(&mut self) {
        self.commit_transaction();
        self.saved = self.document.clone();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
            || self
                .transaction
                .as_ref()
                .is_some_and(|base| base != &self.document)
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
            && self
                .transaction
                .as_ref()
                .is_none_or(|base| base == &self.document)
    }
    /// Apply to a candidate first. Failed commands change neither data nor history.
    pub fn dispatch(&mut self, command: Command) -> Result<(), String> {
        let candidate = apply_command(&self.document, command)?;
        if candidate != self.document {
            let previous = std::mem::replace(&mut self.document, candidate);
            if self.transaction.is_none() {
                self.record(previous);
            }
        }
        Ok(())
    }
    pub fn undo(&mut self) {
        if self.transaction.is_some() {
            self.cancel_transaction();
            return;
        }
        if let Some(previous) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.document, previous));
        }
    }
    pub fn redo(&mut self) {
        self.commit_transaction();
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.document, next));
        }
    }

    /// UI gestures may dispatch many updates but create one undo entry.
    pub fn begin_transaction(&mut self) {
        if self.transaction.is_none() {
            self.transaction = Some(self.document.clone());
        }
    }
    pub fn commit_transaction(&mut self) {
        if let Some(previous) = self.transaction.take()
            && previous != self.document
        {
            self.record(previous);
        }
    }
    pub fn cancel_transaction(&mut self) {
        if let Some(previous) = self.transaction.take() {
            self.document = previous;
        }
    }
    fn record(&mut self, previous: Document) {
        self.undo.push(previous);
        if self.undo.len() > 128 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
}
