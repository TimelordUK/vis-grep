use eframe::egui::{Context, Visuals};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Dark
    }
}

impl Theme {
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
    }

    pub fn apply(&self, ctx: &Context) {
        ctx.set_visuals(match self {
            Theme::Light => Visuals::light(),
            Theme::Dark => Visuals::dark(),
        });
    }

    pub fn cycle(&mut self) {
        *self = match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
    }
}