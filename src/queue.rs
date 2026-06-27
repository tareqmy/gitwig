#![allow(dead_code)]
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    DeleteRepo,
    DeleteBranch(String),
    DeleteTag(String, bool),
    DeleteStash,
    ApplyStash,
    Commit,
    Push(String, bool),
    Checkout(String),
    Merge(String),
    Rebase(String),
    Discard(String, bool),
    CherryPick(String),
    Revert(String),
}

#[derive(Clone, Debug)]
pub enum NeedsUpdate {
    All,
    // Add more granular updates here as needed
}

#[derive(Clone, Debug)]
pub enum InternalEvent {
    ConfirmAction(Action),
    ConfirmedAction(Action),
    ShowError(String),
    ShowStatus(String),
    Update(NeedsUpdate),
    OpenCommitPopup,
    OpenCreateBranch,
    // SwitchTab(Tab), // Will be added when Tabs are defined
}

#[derive(Clone, Default)]
pub struct Queue {
    data: Rc<RefCell<VecDeque<InternalEvent>>>,
}

impl Queue {
    pub fn push(&self, event: InternalEvent) {
        if let Ok(mut data) = self.data.try_borrow_mut() {
            data.push_back(event);
        }
    }

    pub fn pop(&self) -> Option<InternalEvent> {
        if let Ok(mut data) = self.data.try_borrow_mut() {
            data.pop_front()
        } else {
            None
        }
    }
}
