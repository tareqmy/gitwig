#![allow(dead_code, unused_imports)]
pub mod cmd_bar;
use crossterm::event::Event;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::io::Result;

pub enum EventState {
    Consumed,
    NotConsumed,
}

impl EventState {
    pub fn is_consumed(&self) -> bool {
        matches!(self, Self::Consumed)
    }
}

pub enum CommandBlocking {
    Blocking,
    PassingOn,
}

pub trait DrawableComponent {
    fn draw(&self, f: &mut Frame, rect: Rect) -> Result<()>;
}

pub trait Component: DrawableComponent {
    fn event(&mut self, ev: &Event) -> Result<EventState>;
    // fn commands(&self, out: &mut Vec<CommandInfo>, force_all: bool) -> CommandBlocking;
    fn focused(&self) -> bool {
        false
    }
    fn focus(&mut self, _focus: bool) {}
    fn is_visible(&self) -> bool {
        true
    }
    fn hide(&mut self) {}
    fn show(&mut self) -> Result<()> {
        Ok(())
    }
}

pub mod branch_list;
pub mod commit_list;
pub mod diff;
pub mod file_tree;
pub mod stash_list;
pub mod status_list;
pub mod tag_list;
pub mod worktree_list;
