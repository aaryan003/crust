// init command - initialize a new CRUST repository

use anyhow::Result;
use std::fs;

pub fn cmd_init(directory: Option<&str>) -> Result<()> {
    // Determine base path
    let base = directory.unwrap_or(".");

    // Create the target directory if specified
    if let Some(d) = directory {
        fs::create_dir_all(d)?;
    }

    let crust_dir = if base == "." {
        ".crust".to_string()
    } else {
        format!("{}/.crust", base)
    };

    // Check if repository already exists
    if std::path::Path::new(&crust_dir).exists() {
        println!("Reinitialized existing CRUST repository in {}/", crust_dir);
        return Ok(());
    }

    // Create directory structure
    fs::create_dir_all(format!("{}/objects", crust_dir))?;
    fs::create_dir_all(format!("{}/refs/heads", crust_dir))?;
    fs::create_dir_all(format!("{}/refs/tags", crust_dir))?;

    // Create HEAD file pointing to main branch
    fs::write(format!("{}/HEAD", crust_dir), "ref: refs/heads/main\n")?;

    // Create empty index file
    fs::write(format!("{}/index", crust_dir), "")?;

    // Create empty config file
    fs::write(format!("{}/config", crust_dir), "")?;

    println!("Initialized empty CRUST repository in {}/", crust_dir);
    Ok(())
}
