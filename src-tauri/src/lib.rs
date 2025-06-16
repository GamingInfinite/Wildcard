// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use dotenvy::dotenv;
use git2::{build::CheckoutBuilder, Oid, Repository};
use std::env;
use std::env::temp_dir;
use std::fs;
use std::path::PathBuf;
use tokio::task;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_secret_key,
            clone_repo,
            nuke_directory,
            extract_folder_from_repo
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_secret_key(env: String) -> String {
    dotenv().ok(); // loads .env file
    env::var(env).unwrap_or_else(|_| "default".into())
}

#[tauri::command]
async fn clone_repo(
    repo_url: String,
    destination: String,
    commit_sha: Option<String>,
) -> Result<String, String> {
    let dest_path = PathBuf::from(destination.clone());

    let result = task::spawn_blocking(move || {
        let repo = Repository::clone(&repo_url, &dest_path)
            .map_err(|e| format!("Failed to clone repo: {}", e))?;

        if let Some(sha) = commit_sha {
            let oid = Oid::from_str(&sha).map_err(|e| format!("Invalid commit SHA: {}", e))?;
            let commit = repo
                .find_commit(oid)
                .map_err(|e| format!("Commit not found: {}", e))?;

            // Detach HEAD to point to this commit
            repo.set_head_detached(commit.id())
                .map_err(|e| format!("Failed to detach HEAD: {}", e))?;

            let tree = commit
                .tree()
                .map_err(|e| format!("Failed to get tree: {}", e))?;
            let mut checkout = CheckoutBuilder::new();
            checkout.force(); // force to overwrite uncommitted changes
            repo.checkout_tree(tree.as_object(), Some(&mut checkout))
                .map_err(|e| format!("Failed to checkout commit tree: {}", e))?;

            Ok(format!(
                "Cloned {} and checked out commit {}",
                repo_url, sha
            ))
        } else {
            Ok(format!("Cloned {} into {:?}", repo_url, dest_path))
        }
    })
    .await;

    match result {
        Ok(res) => res,
        Err(e) => Err(format!("Background task panicked: {}", e)),
    }
}

#[tauri::command]
async fn extract_folder_from_repo(
    repo_url: String,
    folder_in_repo: String,
    destination_path: String,
) -> Result<String, String> {
    task::spawn_blocking(move || {
        // Create a temporary directory for the cloned repository
        let temp_path = temp_dir().join(&folder_in_repo);

        // Clone the repository
        Repository::clone(&repo_url, &temp_path)
            .map_err(|e| format!("Failed to clone repo: {}", e))?;

        // Locate the desired folder in the cloned repo
        let folder_path = temp_path.join(&folder_in_repo);
        if !folder_path.exists() {
            return Err(format!(
                "Folder '{}' not found in repository.",
                folder_in_repo
            ));
        }

        // Prepare destination path
        let dest_path = PathBuf::from(&destination_path);
        if dest_path.exists() {
            return Err("Destination path already exists.".into());
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directories: {}", e))?;
        }

        // Move the folder
        fs::rename(&folder_path, &dest_path)
            .map_err(|e| format!("Failed to move folder: {}", e))?;

        fs::remove_dir_all(&temp_path).map_err(|e| format!("Failed to delete temp repo: {}", e))?;

        Ok(format!(
            "Folder '{}' extracted to '{}'",
            folder_in_repo, destination_path
        ))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
fn nuke_directory(path: String) -> Result<(), String> {
    let dir = PathBuf::from(path);

    if !dir.exists() {
        return Err("Directory does not exist.".into());
    }

    if !dir.is_dir() {
        return Err("Provided path is not a directory.".into());
    }

    // Read and delete all contents of the directory
    match fs::read_dir(&dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Err(e) = fs::remove_dir_all(&path) {
                            return Err(format!("Failed to remove dir: {:?}", e));
                        }
                    } else {
                        if let Err(e) = fs::remove_file(&path) {
                            return Err(format!("Failed to remove file: {:?}", e));
                        }
                    }
                }
            }
            Ok(())
        }
        Err(e) => Err(format!("Failed to read directory: {:?}", e)),
    }
}
