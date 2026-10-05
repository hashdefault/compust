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
    /// Whether the window is the one the window manager reports as active.
    focused: Option<bool>,
    opacity: Option<u8>,
    blur: Option<bool>,
    fade_ms: Option<u16>,
    shadow: Option<bool>,
    corner_radius: Option<u8>,
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

    /// Whether Compust decorates windows of this type unless a rule says otherwise, with a
    /// shadow and rounded corners: those that stand on the desktop as windows do. Desktops
    /// and docks are part of it, and menus, tooltips, and the like are drawn by toolkits that
    /// often shade them themselves.
    pub(crate) fn decorated(self) -> bool {
        matches!(
            self,
            Self::Normal | Self::Dialog | Self::Utility | Self::Splash | Self::Toolbar
        )
    }
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
    /// Whether Compust decorates the window unless a rule says otherwise, with a shadow and
    /// rounded corners: its type is decorated, and the client draws neither itself.
    pub(crate) decorated: bool,
    /// Whether the window manager shows the window fullscreen: `_NET_WM_STATE` lists
    /// `_NET_WM_STATE_FULLSCREEN`.
    pub(crate) fullscreen: bool,
    /// Whether the window is the active one, or the window manager reports none.
    pub(crate) focused: bool,
}

/// The settings rules give one window; `None` keeps the global value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Overrides {
    pub(crate) opacity: Option<u8>,
    pub(crate) blur: Option<bool>,
    pub(crate) fade_ms: Option<u16>,
    pub(crate) shadow: Option<bool>,
    pub(crate) corner_radius: Option<u8>,
}

impl Rule {
    /// Reject a rule that could never match, would change nothing, or is out of range;
    /// `number` counts rules from 1 for the message.
    pub(crate) fn validate(&self, number: usize) -> Result<()> {
        ensure!(
            self.wm_class.is_some()
                || self.window_type.is_some()
                || self.name.is_some()
                || self.focused.is_some(),
            "rule {number} needs wm_class, window_type, name, or focused"
        );
        ensure!(
            self.opacity.is_some()
                || self.blur.is_some()
                || self.fade_ms.is_some()
                || self.shadow.is_some()
                || self.corner_radius.is_some(),
            "rule {number} needs opacity, blur, fade_ms, shadow, or corner_radius"
        );
        ensure!(
            self.opacity.is_none_or(|opacity| opacity <= 100),
            "rule {number}: opacity must be between 0 and 100"
        );
        ensure!(
            self.corner_radius.is_none_or(|radius| radius <= 64),
            "rule {number}: corner_radius must be between 0 and 64"
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
            && self.focused.is_none_or(|wanted| identity.focused == wanted)
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
        overrides.shadow = overrides.shadow.or(rule.shadow);
        overrides.corner_radius = overrides.corner_radius.or(rule.corner_radius);
    }
    overrides
}

/// The radius of a window's rounded corners before its size limits it: its rule's, or else
/// `global` when Compust decorates the window. A fullscreen window keeps square corners
/// whatever its rules say, so that it still covers its monitor whole.
pub(crate) fn corner_radius(global: u8, overrides: Overrides, identity: &Identity) -> u8 {
    if identity.fullscreen {
        return 0;
    }
    overrides
        .corner_radius
        .unwrap_or(if identity.decorated { global } else { 0 })
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
            decorated: window_type.decorated(),
            fullscreen: false,
            focused: true,
        }
    }

    #[test]
    fn rules_need_a_selector_a_setting_and_valid_values() {
        // Given rules missing a part or out of range, validation names the rule.
        for (text, message) in [
            ("opacity = 50", "rule 1 needs wm_class"),
            ("wm_class = \"a\"", "rule 1 needs opacity"),
            ("wm_class = \"a\"\nopacity = 101", "opacity must be"),
            ("shadow = true", "rule 1 needs wm_class"),
            ("corner_radius = 8", "rule 1 needs wm_class"),
            (
                "wm_class = \"a\"\ncorner_radius = 65",
                "corner_radius must be",
            ),
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
            rule("window_type = \"dock\"\nshadow = true")
                .and_then(|rule| rule.validate(1))
                .is_ok()
        );
        // A radius alone is a setting, zero included.
        assert!(
            rule("window_type = \"notification\"\ncorner_radius = 0")
                .and_then(|rule| rule.validate(1))
                .is_ok()
        );
        // Focus alone chooses windows.
        assert!(
            rule("focused = false\nopacity = 80")
                .and_then(|rule| rule.validate(1))
                .is_ok()
        );
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
            focused: None,
            opacity: Some(80),
            blur: None,
            fade_ms: None,
            shadow: None,
            corner_radius: None,
        };
        assert!(rule.matches(&identity(Some("Brave"), WindowType::Menu, None)));
        assert!(!rule.matches(&identity(Some("brave"), WindowType::Menu, None)));
        assert!(!rule.matches(&identity(Some("Brave"), WindowType::Normal, None)));
        assert!(!rule.matches(&identity(None, WindowType::Menu, None)));
        // A rule for unfocused windows passes over the active one, whatever else matches.
        let inactive = Rule {
            focused: Some(false),
            ..rule
        };
        let mut window = identity(Some("Brave"), WindowType::Menu, None);
        assert!(!inactive.matches(&window));
        window.focused = false;
        assert!(inactive.matches(&window));
    }

    #[test]
    fn each_setting_comes_from_the_first_matching_rule_that_sets_it() {
        // Given a specific rule before a broad one, the specific one wins where it speaks and
        // the broad one fills in the rest; a rule that does not match gives nothing.
        let rules = [
            rule("name = \"Other\"\nopacity = 10").ok(),
            rule("wm_class = \"Term\"\nname = \"Notes\"\nblur = true").ok(),
            rule("wm_class = \"Term\"\nblur = false\nopacity = 90\nfade_ms = 0").ok(),
            rule("window_type = \"normal\"\nshadow = false\ncorner_radius = 12").ok(),
            rule("wm_class = \"Term\"\ncorner_radius = 0").ok(),
        ];
        let rules: Vec<_> = rules.into_iter().flatten().collect();
        assert_eq!(rules.len(), 5);
        assert_eq!(
            overrides(
                &rules,
                &identity(Some("Term"), WindowType::Normal, Some("Notes"))
            ),
            Overrides {
                opacity: Some(90),
                blur: Some(true),
                fade_ms: Some(0),
                shadow: Some(false),
                corner_radius: Some(12),
            }
        );
        assert_eq!(
            overrides(
                &rules,
                &identity(Some("Editor"), WindowType::Dialog, Some("Notes"))
            ),
            Overrides::default()
        );
    }

    #[test]
    fn corners_follow_rules_then_decoration_and_stay_square_fullscreen() {
        // Given a window type Compust decorates and one it does not, the global radius rounds
        // only the first; a rule's radius rounds or squares either, and fullscreen squares all.
        let normal = identity(None, WindowType::Normal, None);
        let dock = identity(None, WindowType::Dock, None);
        let unset = Overrides::default();
        let set = |radius| Overrides {
            corner_radius: Some(radius),
            ..Overrides::default()
        };
        assert_eq!(corner_radius(8, unset, &normal), 8);
        assert_eq!(corner_radius(8, unset, &dock), 0);
        assert_eq!(corner_radius(8, set(12), &dock), 12);
        assert_eq!(corner_radius(8, set(0), &normal), 0);
        assert_eq!(corner_radius(0, set(4), &normal), 4);
        assert_eq!(corner_radius(0, unset, &normal), 0);
        let fullscreen = Identity {
            fullscreen: true,
            ..normal
        };
        assert_eq!(corner_radius(8, unset, &fullscreen), 0);
        assert_eq!(corner_radius(8, set(12), &fullscreen), 0);
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
