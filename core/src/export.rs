use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use log::{debug,warn};
use crate::git_commands;
use std::io::Write;

use crate::error::ReqvireError;
use crate::html_export;
use crate::graph_registry::GraphRegistry;


fn prepare_output_folder(output_folder: &Path) -> std::io::Result<()> {
    // Clean output folder
    if output_folder.exists() {
        fs::remove_dir_all(output_folder)?;
    }
    fs::create_dir_all(output_folder)?;

    // Create a .gitignore file that ignores everything except itself
    let gitignore_path = output_folder.join(".gitignore");
    let mut file = fs::File::create(gitignore_path)?;
    writeln!(
        file,
        "*\n!.gitignore"
    )?;

    Ok(())
}

/// Converts Markdown → HTML *and* copies all registry-internal files into `output_folder`.
pub fn export_model(
    registry: &GraphRegistry,
    output_folder: &Path,
) -> Result<usize, ReqvireError> {

    // Try to get repository root as base directory
    let base_dir = match git_commands::get_git_root_dir() {
        Ok(git_root) => git_root,
        Err(_) => {
            // If Git repository root can't be found, use the current working directory
            std::env::current_dir()
                .map_err(|e| ReqvireError::PathError(format!("Failed to get current directory: {}", e)))?
        }
    };
    
    // prepare output folder
    prepare_output_folder(&output_folder)?;

    let count = html_export::export_markdown_to_html(&base_dir, output_folder)?;
    
    debug!("{} markdown files converted to HTML", count);

    let internal_paths: HashSet<PathBuf> = registry.get_internal_path_targets();

    for src in internal_paths {
        // src is e.g. "core/src/linting/newlines.rs"
        if !src.is_file() {
            warn!("Skipping missing/non-file path: {:?}", src);
            continue;
        }

        // Build the destination: output_folder/core/src/linting/newlines.rs
        let dst = output_folder.join(&src);

        // Ensure parent dirs exist: output_folder/core/src/linting
        if let Some(parent) = dst.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                warn!("Failed to create directory {:?}: {}", parent, e);
                continue;
            }
        }

        // Copy the file
        match fs::copy(&src, &dst) {
            Ok(_)  => println!("✅ Exported: {:?} -> {}", src.display(), dst.display()),
            Err(e) => warn!("Failed to copy {:?}: {}", src, e),
        }
    }

    Ok(count)
}

