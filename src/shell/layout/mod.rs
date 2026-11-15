// SPDX-License-Identifier: GPL-3.0-only

use cosmic_settings_config::{shortcuts::action::Orientation, window_rules::ApplicationException};
use regex::{Regex, RegexSet};
use smithay::{
    desktop::WindowSurface,
    wayland::{
        compositor::with_states,
        shell::xdg::{XdgToplevelSurfaceData, dialog::ToplevelDialogHint},
    },
    xwayland::xwm::WmWindowType,
};
use tracing::warn;

use super::CosmicSurface;

pub mod floating;
pub mod tiling;

pub fn is_dialog(window: &CosmicSurface) -> bool {
    // Check "window type"
    match window.0.underlying_surface() {
        WindowSurface::Wayland(toplevel) => {
            if with_states(toplevel.wl_surface(), |states| {
                let attrs = states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap();
                attrs.parent.is_some() || attrs.dialog_hint == ToplevelDialogHint::Modal
            }) {
                return true;
            }
        }
        WindowSurface::X11(surface) => {
            if surface.is_override_redirect()
                || surface.is_modal()
                || !matches!(
                    surface.window_type(),
                    None | Some(WmWindowType::Normal) | Some(WmWindowType::Utility)
                )
            {
                return true;
            }
        }
    };

    // Check if sizing suggest dialog
    let max_size = window.max_size_without_ssd();
    let min_size = window.min_size_without_ssd();

    min_size.is_some() && min_size == max_size
}

#[derive(Debug, Clone, Default)]
pub struct TilingExceptions {
    /// Default exceptions: a window floats when an app_id and a title pattern match.
    app_ids: RegexSet,
    titles: RegexSet,
    /// User rules with `enabled: false`: a window tiles when an app_id and a title pattern match,
    /// even if a default exception matches it too.
    override_app_ids: RegexSet,
    override_titles: RegexSet,
}

impl TilingExceptions {
    pub fn new<'a, 'b, I, J>(exceptions_config: I, overrides_config: J) -> Self
    where
        I: Iterator<Item = &'a ApplicationException>,
        J: Iterator<Item = &'b ApplicationException>,
    {
        let (app_ids, titles) = pattern_sets(exceptions_config);
        let (override_app_ids, override_titles) = pattern_sets(overrides_config);

        Self {
            app_ids,
            titles,
            override_app_ids,
            override_titles,
        }
    }
}

/// Builds the app_id and title pattern sets. Entries are index-aligned: index `i` in both sets
/// comes from the same rule. Patterns that do not compile are skipped.
fn pattern_sets<'a, I>(exceptions_config: I) -> (RegexSet, RegexSet)
where
    I: Iterator<Item = &'a ApplicationException>,
{
    let mut app_ids = Vec::new();
    let mut titles = Vec::new();

    for exception in exceptions_config {
        if let Err(e) = Regex::new(&exception.appid) {
            warn!("Invalid regex for appid: {}, {}", exception.appid, e);
            continue;
        }
        if let Err(e) = Regex::new(&exception.title) {
            warn!("Invalid regex for title: {}, {}", exception.title, e);
            continue;
        }

        app_ids.push(exception.appid.clone());
        titles.push(exception.title.clone());
    }

    (
        RegexSet::new(app_ids).unwrap(),
        RegexSet::new(titles).unwrap(),
    )
}

/// Returns true if one rule matches the window: its app_id pattern and its title pattern both match.
fn any_rule_matches(app_ids: &RegexSet, titles: &RegexSet, window: &CosmicSurface) -> bool {
    let appid_matches = app_ids.matches(&window.app_id());
    let title_matches = titles.matches(&window.title());
    appid_matches
        .into_iter()
        .any(|idx| title_matches.matched(idx))
}

/// Returns true if a default exception floats the window.
///
/// A user rule with `enabled: false` that matches the window wins over the defaults, so the
/// window tiles.
pub fn has_floating_exception(exceptions: &TilingExceptions, window: &CosmicSurface) -> bool {
    if any_rule_matches(
        &exceptions.override_app_ids,
        &exceptions.override_titles,
        window,
    ) {
        return false;
    }

    any_rule_matches(&exceptions.app_ids, &exceptions.titles, window)
}
