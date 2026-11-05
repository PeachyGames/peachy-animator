use peachy_document::Document;
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn load(path: &Path) -> Result<Document, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc = if path.extension().and_then(|s| s.to_str()) == Some("peachy") {
        serde_json::from_str(&text).map_err(|e| e.to_string())?
    } else {
        let value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        peachy_spine::import_json(
            &value,
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Imported rig"),
        )?
    };
    doc.validate()?;
    Ok(doc)
}

fn asset_paths(doc: &Document) -> BTreeSet<String> {
    doc.skins
        .values()
        .flat_map(|s| s.values())
        .flat_map(|a| a.values())
        .map(|r| r.path.clone())
        .collect()
}
fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

pub fn save(path: &Path, doc: &Document, asset_root: &Path) -> Result<(), String> {
    doc.validate()?;
    let parent = path.parent().ok_or("Invalid save path")?;
    let source = asset_root.canonicalize().map_err(|e| e.to_string())?;
    let destination = parent.canonicalize().map_err(|e| e.to_string())?;
    if source != destination {
        // Validate all paths before copying or committing the project.
        let assets: Vec<_> = asset_paths(doc)
            .into_iter()
            .map(|p| {
                let from = source.join("images").join(format!("{p}.png"));
                let to = destination.join("images").join(format!("{p}.png"));
                (from, to)
            })
            .collect();
        for (from, to) in &assets {
            if !from.is_file() {
                return Err(format!("Missing image {}", from.display()));
            }
            if to.exists()
                && fs::read(from).map_err(|e| e.to_string())?
                    != fs::read(to).map_err(|e| e.to_string())?
            {
                return Err(format!(
                    "Image collision at {}; choose a new folder",
                    to.display()
                ));
            }
        }
        for (from, to) in &assets {
            fs::create_dir_all(to.parent().unwrap()).map_err(|e| e.to_string())?;
            if !to.exists() {
                fs::copy(from, to).map_err(|e| e.to_string())?;
            }
        }
    }
    let bytes = serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?;
    if path.exists() {
        let backup = path.with_extension(format!("peachy.backup-{}", stamp()));
        fs::copy(path, &backup).map_err(|e| e.to_string())?;
        if fs::read(&backup).map_err(|e| e.to_string())?
            != fs::read(path).map_err(|e| e.to_string())?
        {
            return Err("Backup verification failed".into());
        }
    }
    let temporary = path.with_extension(format!("peachy.tmp-{}", stamp()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn export_bundle(parent: &Path, doc: &Document, asset_root: &Path) -> Result<PathBuf, String> {
    let value = peachy_spine::export_json(doc)?;
    let mut regions = vec![];
    let mut x = 2;
    let mut y = 2;
    let mut row = 0;
    let mut used_width = 1;
    for path in asset_paths(doc) {
        let image = image::open(asset_root.join("images").join(format!("{path}.png")))
            .map_err(|e| format!("Image {path}: {e}"))?
            .into_rgba8();
        if image.width() > 2044 || image.height() > 2044 {
            return Err("Slice 1 atlas pages support images up to 2044 pixels".into());
        }
        if x + image.width() + 2 > 2048 {
            x = 2;
            y += row + 4;
            row = 0;
        }
        if y + image.height() + 2 > 2048 {
            return Err("Slice 1 supports one 2048×2048 atlas page".into());
        }
        used_width = used_width.max(x + image.width() + 2);
        row = row.max(image.height());
        regions.push((path, image, x, y));
        x += regions.last().unwrap().1.width() + 4;
    }
    let width = used_width.next_power_of_two();
    let height = (y + row + 2).next_power_of_two();
    let mut atlas_image = image::RgbaImage::new(width, height);
    let mut atlas =
        format!("character.png\nsize: {width},{height}\nfilter: Linear,Linear\npma: false\n");
    for (path, image, x, y) in regions {
        image::imageops::overlay(&mut atlas_image, &image, x as i64, y as i64);
        atlas.push_str(&format!(
            "{path}\n  bounds: {x},{y},{},{}\n",
            image.width(),
            image.height()
        ));
    }
    let folder = parent.join(format!("peachy-export-{}", stamp()));
    fs::create_dir(&folder).map_err(|e| e.to_string())?;
    let result = (|| {
        atlas_image
            .save(folder.join("character.png"))
            .map_err(|e| e.to_string())?;
        fs::write(folder.join("character.atlas"), atlas).map_err(|e| e.to_string())?;
        fs::write(
            folder.join("character.json"),
            serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        Ok(folder.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&folder);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saves_reopens_backs_up_and_exports_original_fixture() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
        let doc = load(&root.join("peach-bot.peachy")).unwrap();
        let folder = std::env::temp_dir().join(format!("peachy-files-{}", stamp()));
        fs::create_dir(&folder).unwrap();
        let path = folder.join("test.peachy");
        save(&path, &doc, &root).unwrap();
        assert_eq!(load(&path).unwrap(), doc);
        let mut changed = doc.clone();
        changed.bones[0].setup.rotation = 10.0;
        save(&path, &changed, &folder).unwrap();
        let backups: Vec<_> = fs::read_dir(&folder)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|p| p.file_name().to_string_lossy().contains("backup-"))
            .collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            serde_json::from_slice::<Document>(&fs::read(backups[0].path()).unwrap()).unwrap(),
            doc
        );
        let export = export_bundle(&folder, &doc, &root).unwrap();
        assert!(export.join("character.png").exists());
        assert!(export.join("character.atlas").exists());
        assert!(export.join("character.json").exists());
        fs::remove_dir_all(folder).unwrap();
    }
}
