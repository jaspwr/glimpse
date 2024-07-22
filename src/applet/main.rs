// Glimpse - GNU/Linux Launcher and File search utility.
// Copyright (C) 2024 https://github.com/jaspwr

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

#[cfg(feature = "app")]
mod app;
#[cfg(feature = "app")]
mod exec;
#[cfg(feature = "app")]
mod icon;
#[cfg(feature = "app")]
mod preview_window;
#[cfg(feature = "app")]
mod result_templates;
#[cfg(feature = "app")]
mod search;
#[cfg(feature = "app")]
mod search_modules;
#[cfg(feature = "app")]
mod utils;

fn main() {
    #[cfg(feature = "app")]
    app::run_app();

    #[cfg(not(feature = "app"))]
    println!("Incorrect build configuration. Please use `--features app`.")
}
