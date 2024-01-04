#[cfg(feature = "app")]
mod biases;
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
#[cfg(feature = "app")]
mod app;

fn main() {
    #[cfg(feature = "app")]
    app::run_app();

    #[cfg(not(feature = "app"))]
    println!("Incorrect build configuration. Please use `--features app`.")
}