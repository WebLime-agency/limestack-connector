// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod printer;
mod protocol;
mod server;

#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_updater::UpdaterExt;

fn main() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Hide from dock on macOS - we're a tray-only app
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            // Start WebSocket server
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(server::start_server(app_handle));
            });

            // Check for updates in background
            let update_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = check_for_updates(update_handle).await {
                    log::warn!("Update check failed: {}", e);
                }
            });

            // Create system tray menu
            let status = MenuItem::with_id(app, "status", "● Running", false, None::<&str>)?;
            let separator1 = PredefinedMenuItem::separator(app)?;
            let open_limestack = MenuItem::with_id(app, "open_limestack", "Open LimeStack", true, None::<&str>)?;
            let separator2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(app, &[
                &status,
                &separator1,
                &open_limestack,
                &separator2,
                &quit,
            ])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open_limestack" => {
                        let _ = open::that("https://app.limestack.io/settings#devices");
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .tooltip("LimeStack Connector")
                .build(app)?;

            log::info!("LimeStack Connector started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn check_for_updates(app: tauri::AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Checking for updates...");

    let updater = app.updater()?;

    match updater.check().await {
        Ok(Some(update)) => {
            log::info!("Update available: {} -> {}", update.current_version, update.version);

            // Download and install the update
            let mut downloaded = 0;
            let bytes = update.download(
                |chunk, total| {
                    downloaded += chunk;
                    log::debug!("Downloaded {} of {:?} bytes", downloaded, total);
                },
                || {
                    log::info!("Download completed, preparing to install...");
                }
            ).await?;

            log::info!("Installing update...");
            update.install(bytes)?;

            log::info!("Update installed. Restarting...");
            app.restart();
        }
        Ok(None) => {
            log::info!("No updates available");
        }
        Err(e) => {
            log::warn!("Update check error: {}", e);
        }
    }

    Ok(())
}
