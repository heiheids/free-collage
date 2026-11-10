// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 黑山大叔 (Heishan Dashu / heiheids)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod codec;
mod config;
mod i18n;
mod model;
mod platform;
mod render;
mod state;
mod tools;
mod ui;

fn main() {
    platform::window::run();
}
