pub mod commands;
pub mod core;
pub mod domain;
pub mod infra;
pub mod state;

use commands::export::{get_export_task_status, retry_export, start_export};
use commands::project::{
    delete_project, evaluate_camera_motion, list_projects, load_project, recover_projects,
    update_camera_motion, update_project_title, update_timeline, validate_quality_gate,
};
use commands::recording::{pause_recording, resume_recording, start_recording, stop_recording};
use commands::settings::{
    get_platform_capability, list_audio_input_devices, load_hotkeys, save_hotkeys,
};
use infra::logging::init_tracing;
use state::RuntimeState;
use tauri::Manager;

pub fn run() {
    init_tracing();

    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            std::fs::create_dir_all(app_data_dir.join("projects"))
                .map_err(|error| error.to_string())?;
            app.manage(RuntimeState::new(app_data_dir.join("projects")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            pause_recording,
            resume_recording,
            stop_recording,
            list_projects,
            load_project,
            update_project_title,
            delete_project,
            update_timeline,
            update_camera_motion,
            evaluate_camera_motion,
            validate_quality_gate,
            start_export,
            retry_export,
            get_export_task_status,
            recover_projects,
            get_platform_capability,
            list_audio_input_devices,
            load_hotkeys,
            save_hotkeys
        ])
        .run(tauri::generate_context!())
        .expect("failed to run FocusLens");
}
