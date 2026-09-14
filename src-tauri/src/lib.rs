mod commands;
mod config;
mod fetch;
mod slurm;
mod ssh_config;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            app.manage(commands::AppState::new(dir.join("clusters.json")));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_clusters,
            commands::save_cluster,
            commands::delete_cluster,
            commands::set_selection,
            commands::list_ssh_hosts,
            commands::fetch_jobs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
