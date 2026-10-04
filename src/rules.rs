use anyhow::{Result, ensure};
use serde::Deserialize;

/// A per-window rule. When every selector it gives matches a window's client, the settings it
/// gives replace the global ones, unless an earlier matching rule already set them.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Rule {
    /// The resource class, the second string of `WM_CLASS`.
    wm_class: Option<String>,
    /// The window's EWMH type.
    window_type: Option<WindowType>,
    /// The title, `_NET_WM_NAME` or else `WM_NAME`.
    name: Option<String>,
    opacity: Option<u8>,
    blur: Option<bool>,
    fade_ms: Option<u16>,
}

/// The EWMH window types, named after their `_NET_WM_WINDOW_TYPE_` suffixes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WindowType {
    Desktop,
    Dock,
    Toolbar,
    Menu,
    Utility,
    Splash,
    Dialog,
    DropdownMenu,
    PopupMenu,
    Tooltip,
    Notification,
    Combo,
    Dnd,
    #[default]
    Normal,
}

impl WindowType {
    pub(crate) const ALL: [(Self, &'static str); 14] = [
        (Self::Desktop, "_NET_WM_WINDOW_TYPE_DESKTOP"),
        (Self::Dock, "_NET_WM_WINDOW_TYPE_DOCK"),
        (Self::Toolbar, "_NET_WM_WINDOW_TYPE_TOOLBAR"),
        (Self::Menu, "_NET_WM_WINDOW_TYPE_MENU"),
        (Self::Utility, "_NET_WM_WINDOW_TYPE_UTILITY"),
        (Self::Splash, "_NET_WM_WINDOW_TYPE_SPLASH"),
        (Self::Dialog, "_NET_WM_WINDOW_TYPE_DIALOG"),
        (Self::DropdownMenu, "_NET_WM_WINDOW_TYPE_DROPDOWN_MENU"),
        (Self::PopupMenu, "_NET_WM_WINDOW_TYPE_POPUP_MENU"),
        (Self::Tooltip, "_NET_WM_WINDOW_TYPE_TOOLTIP"),
        (Self::Notification, "_NET_WM_WINDOW_TYPE_NOTIFICATION"),
        (Self::Combo, "_NET_WM_WINDOW_TYPE_COMBO"),
        (Self::Dnd, "_NET_WM_WINDOW_TYPE_DND"),
        (Self::Normal, "_NET_WM_WINDOW_TYPE_NORMAL"),
    ];
}

/// What rules match on, read from a window's client. A missing or malformed class or title
/// is `None` and satisfies no selector.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Identity {
    pub(crate) class: Option<String>,
    /// The first type in `_NET_WM_WINDOW_TYPE` that Compust knows. Without one, EWMH makes a
    /// window `dialog` when the window manager handles it and it is transient for another
    /// window, and `normal` otherwise.
    pub(crate) window_type: WindowType,
    pub(crate) name: Option<String>,
}

/// The settings rules give one window; `None` keeps the global value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Overrides {
    pub(crate) opacity: Option<u8>,
    pub(crate) blur: Option<bool>,
    pub(crate) fade_ms: Option<u16>,
}

impl Rule {
    /// Reject a rule that could never match, would change nothing, or is out of range;
    /// `number` counts rules from 1 for the message.
    pub(crate) fn validate(&self, number: usize) -> Result<()> {
        ensure!(
            self.wm_class.is_some() || self.window_type.is_some() || self.name.is_some(),
            "rule {number} needs wm_class, window_type, or name"
        );
        ensure!(
            self.opacity.is_some() || self.blur.is_some() || self.fade_ms.is_some(),
            "rule {number} needs opacity, blur, or fade_ms"
        );
        ensure!(
            self.opacity.is_none_or(|opacity| opacity <= 100),
            "rule {number}: opacity must be between 0 and 100"
        );
        Ok(())
    }

    fn matches(&self, identity: &Identity) -> bool {
        let text = |wanted: &Option<String>, found: &Option<String>| {
            wanted
                .as_ref()
                .is_none_or(|wanted| found.as_ref() == Some(wanted))
        };
        text(&self.wm_class, &identity.class)
            && self
                .window_type
                .is_none_or(|wanted| identity.window_type == wanted)
            && text(&self.name, &identity.name)
    }
}

/// The settings `rules` give a window with `identity`: each from the first matching rule that
/// sets it.
pub(crate) fn overrides(rules: &[Rule], identity: &Identity) -> Overrides {
    let mut overrides = Overrides::default();
    for rule in rules.iter().filter(|rule| rule.matches(identity)) {
        overrides.opacity = overrides.opacity.or(rule.opacity);
        overrides.blur = overrides.blur.or(rule.blur);
        overrides.fade_ms = overrides.fade_ms.or(rule.fade_ms);
    }
    overrides
}

/// The resource class in a `WM_CLASS` value: the second of two NUL-terminated Latin-1
/// strings.
pub(crate) fn class(value: &[u8]) -> Option<String> {
    let mut parts = value.split(|byte| *byte == 0);
    let (Some(_instance), Some(class)) = (parts.next(), parts.next()) else {
        return None;
    };
    (!class.is_empty()).then(|| latin1(class))
}

/// A `STRING` property's text, which ICCCM defines as Latin-1.
pub(crate) fn latin1(value: &[u8]) -> String {
    value.iter().map(|byte| char::from(*byte)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(text: &str) -> Result<Rule> {
        Ok(toml::from_str(text)?)
    }

    fn identity(class: Option<&str>, window_type: WindowType, name: Option<&str>) -> Identity {
        Identity {
            class: class.map(str::to_owned),
            window_type,
            name: name.map(str::to_owned),
        }
    }

    #[test]
    fn rules_need_a_selector_a_setting_and_valid_values() {
        // Given rules missing a part or out of range, validation names the rule.
        for (text, message) in [
            ("opacity = 50", "rule 1 needs wm_class"),
            ("wm_class = \"a\"", "rule 1 needs opacity"),
            ("wm_class = \"a\"\nopacity = 101", "opacity must be"),
        ] {
            let error = rule(text).and_then(|rule| rule.validate(1));
            assert!(
                error.is_err_and(|error| error.to_string().contains(message)),
                "{text}"
            );
        }
        assert!(rule("window_type = \"menuu\"\nblur = false").is_err());
        assert!(rule("class = \"a\"\nblur = false").is_err());
        assert!(
            rule("window_type = \"dropdown_menu\"\nblur = false")
                .and_then(|rule| rule.validate(1))
                .is_ok()
        );
    }

    #[test]
    fn every_given_selector_must_match_exactly() {
        let rule = Rule {
            wm_class: Some("Brave".into()),
            window_type: Some(WindowType::Menu),
            name: None,
            opacity: Some(80),
            blur: None,
            fade_ms: None,
        };
        assert!(rule.matches(&identity(Some("Brave"), WindowType::Menu, None)));
        assert!(!rule.matches(&identity(Some("brave"), WindowType::Menu, None)));
        assert!(!rule.matches(&identity(Some("Brave"), WindowType::Normal, None)));
        assert!(!rule.matches(&identity(None, WindowType::Menu, None)));
    }

    #[test]
    fn each_setting_comes_from_the_first_matching_rule_that_sets_it() {
        // Given a specific rule before a broad one, the specific one wins where it speaks and
        // the broad one fills in the rest; a rule that does not match gives nothing.
        let rules = [
            rule("name = \"Other\"\nopacity = 10").ok(),
            rule("wm_class = \"Term\"\nname = \"Notes\"\nblur = true").ok(),
            rule("wm_class = \"Term\"\nblur = false\nopacity = 90\nfade_ms = 0").ok(),
        ];
        let rules: Vec<_> = rules.into_iter().flatten().collect();
        assert_eq!(rules.len(), 3);
        assert_eq!(
            overrides(
                &rules,
                &identity(Some("Term"), WindowType::Normal, Some("Notes"))
            ),
            Overrides {
                opacity: Some(90),
                blur: Some(true),
                fade_ms: Some(0),
            }
        );
        assert_eq!(
            overrides(
                &rules,
                &identity(Some("Editor"), WindowType::Normal, Some("Notes"))
            ),
            Overrides::default()
        );
    }

    #[test]
    fn classes_come_from_the_second_wm_class_string() {
        assert_eq!(
            class(b"brave-browser\0Brave-browser\0"),
            Some("Brave-browser".into())
        );
        assert_eq!(class(b"instance\0Caf\xe9\0"), Some("Café".into()));
        assert_eq!(class(b"only-instance\0"), None);
        assert_eq!(class(b"instance"), None);
    }
}
