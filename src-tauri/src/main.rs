// Prevent a console window on Windows in future ports.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    bettermacro_lib::run();
}
