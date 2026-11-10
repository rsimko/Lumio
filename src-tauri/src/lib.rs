mod commands;
mod state;

use state::{AppPaths, AppState};
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {
            let paths = AppPaths::resolve(app.handle())?;
            let state = AppState::new(paths).map_err(std::io::Error::other)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::add_root,
            commands::create_folder,
            commands::remove_root,
            commands::remove_subfolder,
            commands::update_ui_settings,
            commands::list_subfolders,
            commands::count_folder_images,
            commands::scan_folder,
            commands::get_thumbnail,
            commands::clear_thumbnail_cache,
            commands::open_asset,
            commands::reveal_asset,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Lumio");
}
