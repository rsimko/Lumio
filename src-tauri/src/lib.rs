mod commands;
mod ingest;
mod state;

use ingest::IngestRuntime;
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
            app.manage(IngestRuntime::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::add_root,
            commands::create_folder,
            commands::remove_root,
            commands::remove_subfolder,
            commands::update_ui_settings,
            commands::list_system_fonts,
            commands::list_subfolders,
            commands::count_folder_images,
            commands::scan_folder,
            commands::update_metadata,
            commands::get_hidden_assets,
            commands::move_assets,
            commands::remove_assets,
            commands::restore_assets,
            commands::get_thumbnail,
            commands::get_histogram,
            commands::get_raw_histogram,
            commands::get_camera_info,
            commands::clear_thumbnail_cache,
            commands::edit_assets,
            commands::open_asset,
            commands::reveal_asset,
            commands::open_maps,
            commands::update_ingest_settings,
            commands::detect_ingest_tools,
            commands::create_ingest_plan,
            commands::set_ingest_resolution,
            commands::run_ingest,
            commands::cancel_ingest,
            commands::write_text_file,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Lumio");
}
