//! Generic modal dialog stack. A dialog is a [`Dialog`] view (what the tests
//! read) plus a handler function that reacts to field edits and button
//! presses. A feature adds a dialog by opening an [`OpenDialog`] with its own
//! handler; see `src/README.md`.

use crate::state::State;
use crate::types::*;

/// What the user did to the dialog.
#[derive(Debug, Clone, Copy)]
pub enum DialogAction<'a> {
    /// Typed into a text field / picked a choice by its label.
    Set { field: &'a str, value: &'a str },
    /// Pressed a button (already checked to exist in `Dialog::buttons`).
    Click(&'a str),
}

/// What happens to the dialog after its handler ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Keep,
    Close,
}

/// Handler signature. It owns the whole reaction: validate input, change the
/// document, open follow-up dialogs (they appear above this one), return
/// whether this dialog stays open.
pub type DialogFn = fn(&mut State, &mut OpenDialog, DialogAction<'_>) -> HResult<Outcome>;

/// Continuation of a message box: receives the label of the pressed button
/// after the box has been closed.
pub type MessageFn = fn(&mut State, &str) -> HResult<()>;

#[derive(Debug, Clone)]
pub struct OpenDialog {
    pub dialog: Dialog,
    pub handler: DialogFn,
    /// Only used by [`message_box_handler`].
    pub on_button: Option<MessageFn>,
}

impl OpenDialog {
    pub fn new(dialog: Dialog, handler: DialogFn) -> OpenDialog {
        OpenDialog { dialog, handler, on_button: None }
    }

    /// Convenience constructor for a dialog with text fields.
    pub fn field(name: &str, value: &str) -> DialogField {
        DialogField { name: name.to_owned(), value: value.to_owned(), enabled: true }
    }

    pub fn field_mut(&mut self, name: &str) -> Option<&mut DialogField> {
        self.dialog.fields.iter_mut().find(|f| f.name == name)
    }
}

/// Default reaction to `Set`: store the value in an existing, enabled field.
/// Handlers call this first and then validate/derive other fields.
pub fn store_field(od: &mut OpenDialog, field: &str, value: &str) -> HResult<()> {
    match od.field_mut(field) {
        Some(f) if f.enabled => {
            f.value = value.to_owned();
            Ok(())
        }
        Some(_) => Err(HarnessError::Invalid(format!("field {field:?} is disabled"))),
        None => Err(HarnessError::Invalid(format!("dialog has no field {field:?}"))),
    }
}

/// Handler of message boxes: every button closes the box, then runs the
/// continuation (if any) with the button label.
pub fn message_box_handler(st: &mut State, od: &mut OpenDialog, action: DialogAction<'_>) -> HResult<Outcome> {
    match action {
        DialogAction::Set { field, .. } => {
            Err(HarnessError::Invalid(format!("message box has no field {field:?}")))
        }
        DialogAction::Click(button) => {
            if let Some(cb) = od.on_button {
                cb(st, button)?;
            }
            Ok(Outcome::Close)
        }
    }
}

impl State {
    /// Open a modal dialog on top of the stack.
    pub fn open_dialog(&mut self, od: OpenDialog) {
        self.dialogs.push(od);
    }

    /// Open a message box. `on_button` runs after the box closed.
    pub fn message_box(&mut self, title: &str, text: &str, buttons: &[&str], on_button: Option<MessageFn>) {
        let dialog = Dialog {
            kind: DialogKind::MessageBox,
            title: title.to_owned(),
            text: text.to_owned(),
            buttons: buttons.iter().map(|b| (*b).to_owned()).collect(),
            fields: Vec::new(),
        };
        self.open_dialog(OpenDialog { dialog, handler: message_box_handler, on_button });
    }

    /// An error box titled "Brushling" with a single OK button.
    pub fn error_box(&mut self, text: &str) {
        self.message_box("Brushling", text, &["OK"], None);
    }

    pub fn top_dialog(&self) -> Option<&Dialog> {
        self.dialogs.last().map(|d| &d.dialog)
    }

    pub fn dialog_set(&mut self, field: &str, value: &str) -> HResult<()> {
        self.run_dialog(DialogAction::Set { field, value })
    }

    pub fn dialog_click(&mut self, button: &str) -> HResult<()> {
        let Some(top) = self.dialogs.last() else {
            return Err(HarnessError::Invalid("no dialog is open".into()));
        };
        if !top.dialog.buttons.iter().any(|b| b == button) {
            return Err(HarnessError::Invalid(format!(
                "dialog {:?} has no button {button:?}",
                top.dialog.title
            )));
        }
        self.run_dialog(DialogAction::Click(button))
    }

    /// Esc: press Cancel, or the lone OK of an information box.
    pub fn dialog_escape(&mut self) -> HResult<()> {
        let Some(top) = self.dialogs.last() else { return Ok(()) };
        let buttons = &top.dialog.buttons;
        let target = if buttons.iter().any(|b| b == "Cancel") {
            Some("Cancel")
        } else if buttons.len() == 1 && buttons[0] == "OK" {
            Some("OK")
        } else {
            None
        };
        match target {
            Some(b) => self.dialog_click(b),
            None => Ok(()),
        }
    }

    /// Enter: press OK if the dialog has one.
    pub fn dialog_enter(&mut self) -> HResult<()> {
        let has_ok = self.dialogs.last().is_some_and(|d| d.dialog.buttons.iter().any(|b| b == "OK"));
        if has_ok {
            self.dialog_click("OK")
        } else {
            Ok(())
        }
    }

    /// Run the topmost dialog's handler. The dialog is taken off the stack
    /// while it runs so follow-up dialogs it opens land above it.
    fn run_dialog(&mut self, action: DialogAction<'_>) -> HResult<()> {
        let Some(mut od) = self.dialogs.pop() else {
            return Err(HarnessError::Invalid("no dialog is open".into()));
        };
        let at = self.dialogs.len();
        let handler = od.handler;
        match handler(self, &mut od, action) {
            Ok(Outcome::Keep) => {
                self.dialogs.insert(at.min(self.dialogs.len()), od);
                Ok(())
            }
            Ok(Outcome::Close) => Ok(()),
            Err(e) => {
                // A rejected action leaves the dialog as it was.
                self.dialogs.insert(at.min(self.dialogs.len()), od);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn state() -> State {
        let dir = std::env::temp_dir().join(format!("brushling-dlg-{}", std::process::id()));
        State::new(Settings::load(&dir), Services::default(), (1920, 1080)).unwrap()
    }

    fn reopen(st: &mut State, _button: &str) -> HResult<()> {
        st.error_box("second");
        Ok(())
    }

    #[test]
    fn message_box_stack_and_continuation() {
        let mut st = state();
        st.message_box("Brushling", "first", &["OK", "Cancel"], Some(reopen));
        assert!(st.dialog_click("Nope").is_err());
        assert!(st.dialog_set("x", "y").is_err());
        st.dialog_click("OK").unwrap();
        assert_eq!(st.top_dialog().map(|d| d.text.as_str()), Some("second"));
        st.dialog_escape().unwrap();
        assert!(st.top_dialog().is_none());
        assert!(st.dialog_click("OK").is_err());
    }
}
