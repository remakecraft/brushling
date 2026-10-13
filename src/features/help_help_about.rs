//! Feature help.help_about: Help > Help Topics and Help > About Brushling.
//!
//! Help Topics opens the separate, non-modal help window (`Panel::Help`); the
//! window is a single instance, so opening it twice changes nothing. Opening
//! it never touches the document, selection, text frame or an unfinished
//! curve/polygon. The window is modelled by [`HelpModel`]: Contents, Index and
//! Search tabs, a topic pane and a Hide/Back/Forward/Options toolbar, with the
//! text in `help_content`. About is a modal box with the name, version and licence.

use crate::dialogs::{message_box_handler, OpenDialog};
use crate::features::help_content::{Topic, TOPICS};
use crate::registry::Registry;
use crate::state::State;
use crate::types::*;

pub fn register(r: &mut Registry) {
    r.command("Help > Help Topics", help_topics);
    r.command("Help > About Brushling", about);
}

/// The licence notice shown in the About box (matches `Cargo.toml`).
const LICENCE: &str = "Released under the MIT licence.";

/// Whether the help window is open.
pub fn help_visible(st: &State) -> bool {
    st.help_open
}

/// Press the help window's close button.
pub fn close_help(st: &mut State) {
    st.help_open = false;
}

fn help_topics(st: &mut State) -> HResult<()> {
    // One window only: a second request leaves the open window as it is.
    if !st.help_open {
        st.help = HelpModel::default();
        st.help_open = true;
    }
    Ok(())
}

// ----- the help window model

/// The state of the Help Topics window. Topics are indices into `TOPICS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpModel {
    tab: HelpTab,
    tabs_visible: bool,
    /// Topics shown so far and the position in that list (Back / Forward).
    history: Vec<usize>,
    pos: usize,
    keyword_field: String,
    selected_keyword: Option<String>,
    search_field: String,
    results: Vec<usize>,
    selected_result: Option<usize>,
}

impl Default for HelpModel {
    fn default() -> Self {
        HelpModel {
            tab: HelpTab::Contents,
            tabs_visible: true,
            history: vec![0],
            pos: 0,
            keyword_field: String::new(),
            selected_keyword: None,
            search_field: String::new(),
            results: Vec::new(),
            selected_result: None,
        }
    }
}

const OPTIONS: [&str; 3] = ["Back", "Forward", "Print"];

/// Every Index keyword once, sorted case-insensitively.
fn all_keywords() -> Vec<&'static str> {
    let mut words: Vec<&'static str> = TOPICS.iter().flat_map(|t| t.keywords.iter().copied()).collect();
    words.sort_by_key(|w| w.to_lowercase());
    words.dedup_by_key(|w| w.to_lowercase());
    words
}

fn topic_by_title(title: &str) -> HResult<usize> {
    TOPICS
        .iter()
        .position(|t| t.title == title)
        .ok_or_else(|| HarnessError::Invalid(format!("no help topic titled {title:?}")))
}

/// The first topic that lists the keyword.
fn topic_for_keyword(keyword: &str) -> Option<usize> {
    TOPICS.iter().position(|t| t.keywords.iter().any(|k| k.eq_ignore_ascii_case(keyword)))
}

/// Topics whose title or text contain every word of the query.
fn search(query: &str) -> Vec<usize> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return Vec::new();
    }
    let matches = |t: &Topic| {
        let haystack = format!("{} {}", t.title, t.body).to_lowercase();
        words.iter().all(|w| haystack.contains(w.as_str()))
    };
    (0..TOPICS.len()).filter(|&i| matches(&TOPICS[i])).collect()
}

impl HelpModel {
    /// Show a topic; the forward history is dropped.
    fn show(&mut self, topic: usize) {
        if self.history[self.pos] != topic {
            self.history.truncate(self.pos + 1);
            self.history.push(topic);
            self.pos += 1;
        }
    }

    fn can_back(&self) -> bool {
        self.pos > 0
    }

    fn can_forward(&self) -> bool {
        self.pos + 1 < self.history.len()
    }

    fn step(&mut self, forward: bool) {
        if forward && self.can_forward() {
            self.pos += 1;
        } else if !forward && self.can_back() {
            self.pos -= 1;
        }
    }

    fn display(&mut self) {
        let topic = match self.tab {
            HelpTab::Contents => None,
            HelpTab::Index => self.selected_keyword.as_deref().and_then(topic_for_keyword),
            HelpTab::Search => self.selected_result,
        };
        if let Some(t) = topic {
            self.show(t);
        }
    }

    fn view(&self) -> HelpView {
        let topic = &TOPICS[self.history[self.pos]];
        let hide_label = if self.tabs_visible { "Hide" } else { "Show" };
        let titles = |ids: &[usize]| ids.iter().map(|&i| TOPICS[i].title.to_owned()).collect();
        HelpView {
            tab: self.tab,
            tabs_visible: self.tabs_visible,
            toolbar: vec![
                (hide_label.to_owned(), true),
                ("Back".to_owned(), self.can_back()),
                ("Forward".to_owned(), self.can_forward()),
                ("Options".to_owned(), true),
            ],
            options: OPTIONS.iter().map(|o| (*o).to_owned()).collect(),
            contents: titles(&(0..TOPICS.len()).collect::<Vec<_>>()),
            keyword_field: self.keyword_field.clone(),
            keywords: all_keywords().into_iter().map(str::to_owned).collect(),
            selected_keyword: self.selected_keyword.clone(),
            search_field: self.search_field.clone(),
            results: titles(&self.results),
            selected_result: self.selected_result.map(|i| TOPICS[i].title.to_owned()),
            topic_title: topic.title.to_owned(),
            topic_text: topic.body.to_owned(),
        }
    }
}

/// `App::help_window`.
pub fn view(st: &State) -> Option<HelpView> {
    st.help_open.then(|| st.help.view())
}

/// `App::help_action`.
pub fn act(st: &mut State, action: HelpAction) -> HResult<()> {
    if !st.help_open {
        return Err(HarnessError::Invalid("the Help window is not open".into()));
    }
    let tab = st.help.tab;
    let needs_tabs = !matches!(action, HelpAction::Hide | HelpAction::Back | HelpAction::Forward | HelpAction::Options(_));
    if needs_tabs && !st.help.tabs_visible {
        return Err(HarnessError::Invalid("the Help tabs are hidden".into()));
    }
    let wrong_tab = |wanted: HelpTab| {
        if tab == wanted {
            Ok(())
        } else {
            Err(HarnessError::Invalid(format!("that action belongs to the {wanted:?} tab, which is not selected")))
        }
    };
    let h = &mut st.help;
    match action {
        HelpAction::SelectTab(t) => h.tab = t,
        HelpAction::OpenTopic(title) => {
            wrong_tab(HelpTab::Contents)?;
            h.show(topic_by_title(&title)?);
        }
        HelpAction::TypeKeyword(text) => {
            wrong_tab(HelpTab::Index)?;
            let lower = text.to_lowercase();
            h.selected_keyword = (!lower.is_empty())
                .then(|| all_keywords().into_iter().find(|k| k.to_lowercase().starts_with(&lower)))
                .flatten()
                .map(str::to_owned);
            h.keyword_field = text;
        }
        HelpAction::SelectKeyword(word) => {
            wrong_tab(HelpTab::Index)?;
            let found = all_keywords().into_iter().find(|k| k.eq_ignore_ascii_case(&word));
            let Some(found) = found else {
                return Err(HarnessError::Invalid(format!("no Index keyword {word:?}")));
            };
            h.keyword_field = found.to_owned();
            h.selected_keyword = Some(found.to_owned());
        }
        HelpAction::TypeSearch(text) => {
            wrong_tab(HelpTab::Search)?;
            h.search_field = text;
        }
        HelpAction::ListTopics => {
            wrong_tab(HelpTab::Search)?;
            h.results = search(&h.search_field);
            h.selected_result = None;
        }
        HelpAction::SelectResult(title) => {
            wrong_tab(HelpTab::Search)?;
            let topic = topic_by_title(&title)?;
            if !h.results.contains(&topic) {
                return Err(HarnessError::Invalid(format!("{title:?} is not in the search results")));
            }
            h.selected_result = Some(topic);
        }
        HelpAction::Display => h.display(),
        HelpAction::Hide => h.tabs_visible = !h.tabs_visible,
        HelpAction::Back => h.step(false),
        HelpAction::Forward => h.step(true),
        HelpAction::Options(item) => match item.as_str() {
            "Back" => h.step(false),
            "Forward" => h.step(true),
            "Print" => return print_topic(st),
            _ => return Err(HarnessError::Invalid(format!("the Options menu has no item {item:?}"))),
        },
    }
    Ok(())
}

/// Options > Print: the shown topic goes to the printer as one page.
fn print_topic(st: &mut State) -> HResult<()> {
    if st.services.printer {
        st.record_call(PlatformCall::Print { pages: 1 });
    } else {
        st.error_box("No printer is installed.");
    }
    Ok(())
}

/// The text of the About box.
pub fn about_text() -> String {
    format!("Brushling\nVersion {}\n{}", crate::VERSION, LICENCE)
}

fn about(st: &mut State) -> HResult<()> {
    let dialog = Dialog {
        kind: DialogKind::About,
        title: "About Brushling".to_owned(),
        text: about_text(),
        buttons: vec!["OK".to_owned()],
        fields: Vec::new(),
    };
    st.open_dialog(OpenDialog::new(dialog, message_box_handler));
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{App, DialogKind, HelpAction, HelpTab, PlatformCall, StartOptions};

    fn app(tag: &str) -> App {
        App::start(StartOptions::new(std::env::temp_dir().join(format!("brushling-help-{}-{tag}", std::process::id())))).unwrap()
    }

    #[test]
    fn help_opens_once_and_closes() {
        let mut a = app("once");
        a.menu("Help > Help Topics").unwrap();
        a.menu("Help > Help Topics").unwrap();
        assert!(super::help_visible(&a.st));
        super::close_help(&mut a.st);
        assert!(!super::help_visible(&a.st));
    }

    #[test]
    fn about_box_has_name_version_licence_and_ok() {
        let mut a = app("about");
        a.menu("Help > About Brushling").unwrap();
        let d = a.dialog().unwrap().unwrap();
        assert_eq!(d.kind, DialogKind::About);
        assert!(d.text.contains("Brushling") && d.text.contains(crate::VERSION) && d.text.contains("licence"));
        a.dialog_click("OK").unwrap();
        assert!(a.dialog().unwrap().is_none());
    }

    fn open(tag: &str) -> App {
        let mut a = app(tag);
        a.menu("Help > Help Topics").unwrap();
        a
    }

    #[test]
    fn window_opens_on_contents_with_the_welcome_topic() {
        let a = open("view");
        let v = a.help_window().unwrap().unwrap();
        assert_eq!(v.tab, HelpTab::Contents);
        assert!(v.tabs_visible && v.contents.len() > 5);
        assert_eq!(v.topic_title, "Welcome to Brushling");
        let labels: Vec<_> = v.toolbar.iter().map(|b| b.0.as_str()).collect();
        assert_eq!(labels, ["Hide", "Back", "Forward", "Options"]);
        assert!(!v.toolbar[1].1 && !v.toolbar[2].1);
    }

    #[test]
    fn back_and_forward_follow_the_topics_shown() {
        let mut a = open("history");
        a.help_action(HelpAction::OpenTopic("Edit menu".into())).unwrap();
        a.help_action(HelpAction::OpenTopic("Tool Box".into())).unwrap();
        a.help_action(HelpAction::Back).unwrap();
        let v = a.help_window().unwrap().unwrap();
        assert_eq!(v.topic_title, "Edit menu");
        assert!(v.toolbar[1].1 && v.toolbar[2].1);
        a.help_action(HelpAction::OpenTopic("Image menu".into())).unwrap();
        let v = a.help_window().unwrap().unwrap();
        assert!(!v.toolbar[2].1, "a new topic drops the forward history");
    }

    #[test]
    fn index_keyword_display_opens_its_topic() {
        let mut a = open("index");
        a.help_action(HelpAction::SelectTab(HelpTab::Index)).unwrap();
        a.help_action(HelpAction::TypeKeyword("marg".into())).unwrap();
        let v = a.help_window().unwrap().unwrap();
        assert_eq!(v.selected_keyword.as_deref(), Some("margins"));
        a.help_action(HelpAction::Display).unwrap();
        assert_eq!(a.help_window().unwrap().unwrap().topic_title, "Printing and Print Preview");
    }

    #[test]
    fn search_lists_topics_containing_every_word() {
        let mut a = open("search");
        a.help_action(HelpAction::SelectTab(HelpTab::Search)).unwrap();
        a.help_action(HelpAction::TypeSearch("wallpaper tiled".into())).unwrap();
        a.help_action(HelpAction::ListTopics).unwrap();
        let v = a.help_window().unwrap().unwrap();
        assert_eq!(v.results, ["File menu"]);
        a.help_action(HelpAction::SelectResult("File menu".into())).unwrap();
        a.help_action(HelpAction::Display).unwrap();
        assert_eq!(a.help_window().unwrap().unwrap().topic_title, "File menu");
    }

    #[test]
    fn hide_toggles_the_tabs_and_reopening_resets_the_window() {
        let mut a = open("hide");
        a.help_action(HelpAction::Hide).unwrap();
        let v = a.help_window().unwrap().unwrap();
        assert!(!v.tabs_visible && v.toolbar[0].0 == "Show");
        assert!(a.help_action(HelpAction::SelectTab(HelpTab::Search)).is_err());
        super::close_help(&mut a.st);
        assert!(a.help_window().unwrap().is_none());
        a.menu("Help > Help Topics").unwrap();
        assert!(a.help_window().unwrap().unwrap().tabs_visible);
    }

    #[test]
    fn options_print_records_one_page_job() {
        let mut a = open("print");
        a.help_action(HelpAction::Options("Print".into())).unwrap();
        assert_eq!(a.platform_calls().unwrap(), [PlatformCall::Print { pages: 1 }]);
    }

    #[test]
    fn actions_fail_while_the_window_is_closed() {
        let mut a = app("closed");
        assert!(a.help_action(HelpAction::Back).is_err());
    }
}
