//! Thread-safe, lock-free internal event queue for components communication with the App engine.

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
    ConfirmYes,
    ConfirmNo,
    InputChar(char),
    InputBackspace,
    InputEnter,
    InputEsc,
    ShowError(String),
    ShowStatus(String),
    Update(NeedsUpdate),
    OpenCommitPopup,
    OpenCreateBranch,
    ClosePopup,
    Commit,

    SearchColumnPicker,
    StartCommit,
    StartCommitAmend,
    StartTagCreate,
    RunInteractiveRebase,
    RequestCherryPick,
    YankSelectedCommitHash,
    RequestRevert,
    InspectCommit,
    CommitSelectionUp,
    CommitSelectionDown,
    CommitSelectionPageUp,
    CommitSelectionPageDown,

    CommitSelectionTop,
    CommitSelectionBottom,
    OpenCommitHistoryPicker,
    CommitHistoryPickerUp,
    CommitHistoryPickerDown,
    CommitHistoryPickerSelect,
    CommitHistoryPickerCancel,
    LoadMoreCommits,
    CommitDetailsUp,
    CommitDetailsDown,
    StagingFileUp,
    StagingFileDown,
    ConflictFileUp,
    ConflictFileDown,
    StageSelectedFile,
    UnstageSelectedFile,
    ResolveConflictOurs,
    ResolveConflictTheirs,
    MarkConflictResolved,
    MergeAbortConfirm,
    MergeContinueConfirm,
    StageSelectedHunk,
    UnstageSelectedHunk,
    StageAllChanges,
    UnstageAllChanges,
    RequestDiscardChanges,
    RequestDiscardAllChanges,
    StartStashCreate,
    DiffScrollUp,
    DiffScrollDown,
    DiffScrollPageUp,
    DiffScrollPageDown,
    DiffScrollTop,
    DiffScrollBottom,

    // FileTree
    FileTreeUp,
    FileTreeDown,
    FileTreePageUp,
    FileTreePageDown,
    FileTreeTop,
    FileTreeBottom,
    FileContentUp,
    FileContentDown,
    FileContentPageUp,
    FileContentPageDown,
    FileContentTop,
    FileContentBottom,
    ToggleFolderExpanded,
    CollapseAllFolders,
    RequestDiscardFile,

    // BranchList
    LocalBranchUp,
    LocalBranchDown,
    LocalBranchPageUp,
    LocalBranchPageDown,
    LocalBranchTop,
    LocalBranchBottom,
    RemoteBranchUp,
    RemoteBranchDown,
    RemoteBranchPageUp,
    RemoteBranchPageDown,
    RemoteBranchTop,
    RemoteBranchBottom,
    CheckoutBranch,
    RequestDeleteBranch,
    StartBranchCreate,
    StartBranchMerge,
    StartBranchRebase,
    RequestBranchPush,
    FetchRemote,
    StartRemoteAdd,
    RequestDeleteRemote,

    // TagList
    TagUp,
    TagDown,
    TagPageUp,
    TagPageDown,
    TagTop,
    TagBottom,
    CheckoutTag,
    RequestDeleteTag,
    RequestPushTag,
    RequestPushAllTags,
    FetchRemoteTags,

    // StashList
    StashUp,
    StashDown,
    StashPageUp,
    StashPageDown,
    StashTop,
    StashBottom,
    StashFileUp,
    StashFileDown,
    StashFilePageUp,
    StashFilePageDown,
    StashFileTop,
    StashFileBottom,
    RequestDeleteStash,
    RequestApplyStash,
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
        if let Ok(mut data) = self.data.try_borrow_mut() { data.pop_front() } else { None }
    }
}
