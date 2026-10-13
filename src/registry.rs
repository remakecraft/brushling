//! The feature registry: every feature contributes its tool handler, menu
//! commands, shortcuts, pointer hooks, file decoders and render layers from
//! its own `features/<name>.rs::register`. The registry is built once, on
//! first use, and is read-only afterwards.

use crate::formats::DecodeFn;
use crate::state::State;
use crate::tools::{PointerEvent, ToolHandler};
use crate::types::*;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Menu command body.
pub type CommandFn = fn(&mut State) -> HResult<()>;
/// Whether a menu command is currently enabled (default: always).
pub type EnabledFn = fn(&State) -> bool;
/// Sees every pointer event before the active tool; return `true` to consume
/// it (canvas resize handles, selection handles).
pub type PointerHookFn = fn(&mut State, &PointerEvent) -> HResult<bool>;
/// Draws extra content (floating selection, tool previews, grid, marquee)
/// onto an image in the layer's coordinate space.
pub type LayerFn = fn(&State, &mut Image);

#[derive(Clone, Copy)]
pub struct Command {
    pub run: CommandFn,
    pub enabled: Option<EnabledFn>,
}

#[derive(Default)]
pub struct Registry {
    tools: HashMap<Tool, &'static dyn ToolHandler>,
    commands: HashMap<&'static str, Command>,
    shortcuts: Vec<(Key, Mods, &'static str)>,
    pointer_hooks: Vec<PointerHookFn>,
    decoders: Vec<DecodeFn>,
    composite_layers: Vec<LayerFn>,
    workspace_layers: Vec<LayerFn>,
}

impl Registry {
    /// Register the handler of a tool. Registering twice is a bug.
    pub fn tool(&mut self, tool: Tool, handler: &'static dyn ToolHandler) {
        let prev = self.tools.insert(tool, handler);
        assert!(prev.is_none(), "tool {tool:?} registered twice");
    }

    /// Register a menu command under its full path, e.g. `"Image > Invert Colors"`.
    pub fn command(&mut self, path: &'static str, run: CommandFn) {
        self.command_if(path, run, None);
    }

    /// Like [`Registry::command`] with an enabled-state predicate.
    pub fn command_if(&mut self, path: &'static str, run: CommandFn, enabled: Option<EnabledFn>) {
        let prev = self.commands.insert(path, Command { run, enabled });
        assert!(prev.is_none(), "menu command {path:?} registered twice");
    }

    /// Bind a key (letters match case-insensitively) to a menu path.
    pub fn shortcut(&mut self, key: Key, mods: Mods, path: &'static str) {
        self.shortcuts.push((key, mods, path));
    }

    pub fn pointer_hook(&mut self, hook: PointerHookFn) {
        self.pointer_hooks.push(hook);
    }

    pub fn decoder(&mut self, decode: DecodeFn) {
        self.decoders.push(decode);
    }

    /// Layer drawn into `render(Composite)` (document coordinates, 1:1).
    pub fn composite_layer(&mut self, layer: LayerFn) {
        self.composite_layers.push(layer);
    }

    /// Layer drawn into `render(Workspace)` after the zoomed canvas, in
    /// workspace pixels (grid, marquee, handles).
    pub fn workspace_layer(&mut self, layer: LayerFn) {
        self.workspace_layers.push(layer);
    }

    // ----- lookups

    pub fn tool_handler(&self, tool: Tool) -> Option<&'static dyn ToolHandler> {
        self.tools.get(&tool).copied()
    }

    pub fn command_for(&self, path: &str) -> Option<Command> {
        self.commands.get(path).copied()
    }

    pub fn shortcut_for(&self, key: Key, mods: Mods) -> Option<&'static str> {
        self.shortcuts.iter().find(|(k, m, _)| key_eq(*k, key) && *m == mods).map(|(_, _, p)| *p)
    }

    pub fn decoders(&self) -> &[DecodeFn] {
        &self.decoders
    }

    pub fn composite_layers(&self) -> &[LayerFn] {
        &self.composite_layers
    }

    pub fn workspace_layers(&self) -> &[LayerFn] {
        &self.workspace_layers
    }

    pub fn pointer_hooks(&self) -> &[PointerHookFn] {
        &self.pointer_hooks
    }
}

fn key_eq(a: Key, b: Key) -> bool {
    match (a, b) {
        (Key::Char(x), Key::Char(y)) => x.to_lowercase().eq(y.to_lowercase()),
        _ => a == b,
    }
}

/// The process-wide registry.
pub fn registry() -> &'static Registry {
    static REGISTRY: OnceLock<Registry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut r = Registry::default();
        crate::commands::register_core(&mut r);
        crate::features::register_all(&mut r);
        r
    })
}
