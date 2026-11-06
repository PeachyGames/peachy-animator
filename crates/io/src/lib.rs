//! Shared imperative shell for project files, image copying and packed exports.
use peachy_document::{Curve, Document, Keyframe, Property, Track};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn load(path: &Path) -> Result<Document, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let doc = if path.extension().and_then(|s| s.to_str()) == Some("peachy") {
        decode_project(&text)?
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

pub fn decode_project(text: &str) -> Result<Document, String> {
    let mut value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    // Version 1 had only regions and no attachment discriminator. Add that tag,
    // then take the same migration path as tagged legacy attachments.
    if value["format"].as_u64() == Some(1) {
        if let Some(skins) = value["skins"].as_object_mut() {
            for skin in skins.values_mut() {
                if let Some(slots) = skin.as_object_mut() {
                    for attachments in slots.values_mut() {
                        if let Some(attachments) = attachments.as_object_mut() {
                            for attachment in attachments.values_mut() {
                                let object =
                                    attachment.as_object_mut().ok_or("Invalid v1 region")?;
                                if object.contains_key("type") {
                                    return Err(
                                        "Version 1 attachments cannot have a type field".into()
                                    );
                                }
                                object.insert("type".into(), "region".into());
                            }
                        }
                    }
                }
            }
        }
        value["format"] = 2.into();
    }
    let durations = if value["format"].as_u64() == Some(2) {
        let durations = take_v2_durations(&mut value)?;
        value["format"] = 3.into();
        Some(durations)
    } else {
        None
    };
    if value["format"].as_u64() == Some(3) {
        wrap_v3_mesh_vertices(&mut value)?;
        value["format"] = peachy_document::FORMAT_VERSION.into();
    }
    let mut doc: Document = serde_json::from_value(value).map_err(|e| e.to_string())?;
    // Wrapping vertices preserves their values and all surrounding fields. Decode
    // the strict schema and validate every legacy field before adding duration holds.
    doc.validate()?;
    if let Some(durations) = durations {
        materialize_v2_holds(&mut doc, durations)?;
    }
    Ok(doc)
}

fn take_v2_durations(value: &mut serde_json::Value) -> Result<Vec<f32>, String> {
    let animations = value["animations"]
        .as_array_mut()
        .ok_or("Legacy animations must be an array")?;
    let mut durations = Vec::with_capacity(animations.len());
    for animation in animations {
        let duration = animation
            .as_object_mut()
            .ok_or("Invalid legacy animation")?
            .remove("duration")
            .ok_or("Legacy animation is missing duration")?;
        let duration: f32 = serde_json::from_value(duration).map_err(|e| e.to_string())?;
        if !duration.is_finite() || duration <= 0.0 {
            return Err("Legacy animation duration must be finite and positive".into());
        }
        durations.push(duration);
    }
    Ok(durations)
}

fn wrap_v3_mesh_vertices(value: &mut serde_json::Value) -> Result<(), String> {
    if let Some(skins) = value["skins"].as_object_mut() {
        for skin in skins.values_mut() {
            if let Some(slots) = skin.as_object_mut() {
                for attachments in slots.values_mut() {
                    if let Some(attachments) = attachments.as_object_mut() {
                        for attachment in attachments.values_mut() {
                            if attachment["type"].as_str() == Some("mesh") {
                                let vertices = attachment
                                    .get_mut("vertices")
                                    .ok_or("Legacy mesh is missing vertices")?;
                                if !vertices.is_array() {
                                    return Err("Legacy mesh vertices must be an array".into());
                                }
                                *vertices = serde_json::json!({
                                    "mode":"slotLocal", "positions":vertices.take()
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn materialize_v2_holds(doc: &mut Document, durations: Vec<f32>) -> Result<(), String> {
    for (animation, &duration) in doc.animations.iter().zip(&durations) {
        if animation
            .tracks
            .iter()
            .flat_map(|track| &track.keys)
            .any(|key| key.time > duration)
        {
            return Err(format!(
                "Key exceeds legacy duration in animation {}",
                animation.name
            ));
        }
    }
    for (animation, duration) in doc.animations.iter_mut().zip(durations) {
        if animation.tracks.iter().all(|track| track.keys.is_empty()) {
            let root_rotation = animation
                .tracks
                .iter()
                .position(|track| track.bone == 0 && track.property == Property::Rotation);
            let index = root_rotation.unwrap_or_else(|| {
                animation.tracks.push(Track {
                    bone: 0,
                    property: Property::Rotation,
                    keys: vec![],
                });
                animation.tracks.len() - 1
            });
            animation.tracks[index].keys.push(Keyframe {
                time: duration,
                value: 0.0,
                curve: Curve::Linear,
            });
        } else {
            for track in &mut animation.tracks {
                if let Some(last) = track.keys.last_mut()
                    && last.time < duration
                {
                    // The old exporter omitted a final Bezier without a next key.
                    // Do not activate that formerly unused curve by appending a hold.
                    if matches!(last.curve, Curve::Bezier(_)) {
                        last.curve = Curve::Linear;
                    }
                    let value = last.value;
                    track.keys.push(Keyframe {
                        time: duration,
                        value,
                        curve: Curve::Linear,
                    });
                }
            }
        }
    }
    doc.validate()?;
    Ok(())
}

pub fn create_project(path: &Path, doc: &Document) -> Result<(), String> {
    doc.validate()?;
    require_extension(path, "peachy")?;
    write_new(
        path,
        &serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?,
    )
}

pub fn export_json(path: &Path, doc: &Document) -> Result<(), String> {
    require_extension(path, "json")?;
    let value = peachy_spine::export_json(doc)?;
    write_new(
        path,
        &serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
    )
}

fn require_extension(path: &Path, extension: &str) -> Result<(), String> {
    if path.extension().and_then(|s| s.to_str()) != Some(extension) {
        return Err(format!("Output must have .{extension} extension"));
    }
    Ok(())
}

/// Publish a complete new file without replacing an existing destination.
/// The temporary file and hard link are on the same filesystem.
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("tmp-{}-{}", std::process::id(), stamp()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|e| e.to_string())?;
        fs::hard_link(&temporary, path)
            .map_err(|e| format!("Cannot publish new file {}: {e}", path.display()))
    })();
    let _ = fs::remove_file(temporary);
    result
}

fn stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

pub fn save(path: &Path, doc: &Document, asset_root: &Path) -> Result<(), String> {
    doc.validate()?;
    require_extension(path, "peachy")?;
    let bytes = serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?;
    let old = if path.exists() {
        Some(fs::read(path).map_err(|e| e.to_string())?)
    } else {
        None
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let source = asset_root.canonicalize().map_err(|e| e.to_string())?;
    let destination = parent.canonicalize().map_err(|e| e.to_string())?;
    if source != destination {
        // Validate all paths before copying or committing the project.
        let assets: Vec<_> = doc
            .image_paths()
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
    if let Some(old) = old {
        let backup = path.with_extension(format!("peachy.backup-{}", stamp()));
        write_new(&backup, &old)?;
        if fs::read(&backup).map_err(|e| e.to_string())? != old {
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
    for path in doc.image_paths() {
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
        x += regions.last().expect("Region just appended").1.width() + 4;
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
    fn migrates_v1_regions_without_weakening_the_strict_native_schema() {
        let doc = load(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/peach-bot/peach-bot.peachy"),
        )
        .unwrap();
        let mut v = serde_json::to_value(&doc).unwrap();
        v["format"] = 1.into();
        for (animation, source) in v["animations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .zip(&doc.animations)
        {
            animation["duration"] = serde_json::json!(source.duration());
        }
        for skin in v["skins"].as_object_mut().unwrap().values_mut() {
            for slot in skin.as_object_mut().unwrap().values_mut() {
                for attachment in slot.as_object_mut().unwrap().values_mut() {
                    attachment.as_object_mut().unwrap().remove("type");
                }
            }
        }
        assert_eq!(decode_project(&v.to_string()).unwrap(), doc);
        let mut bad = v.clone();
        bad["skins"]["default"]["hand.R"]["hand.R"]["vertices"] = serde_json::json!([]);
        assert!(decode_project(&bad.to_string()).is_err());
        v["format"] = (peachy_document::FORMAT_VERSION + 1).into();
        assert!(decode_project(&v.to_string()).is_err());
    }
    #[test]
    fn native_projects_reject_unknown_fields_instead_of_losing_future_features() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../../examples/peach-bot/peach-bot.peachy"))
                .unwrap();
        value["meshes"] = serde_json::json!([]);
        assert!(decode_project(&value.to_string()).is_err());
        value.as_object_mut().unwrap().remove("meshes");
        value["bones"][0]["futureConstraint"] = serde_json::json!(1);
        assert!(decode_project(&value.to_string()).is_err());
    }
    #[test]
    fn failed_publish_keeps_existing_file_intact_and_removes_temporary_file() {
        let folder = std::env::temp_dir().join(format!("peachy-atomic-{}", stamp()));
        fs::create_dir(&folder).unwrap();
        let output = folder.join("original");
        fs::write(&output, b"original").unwrap();
        assert!(write_new(&output, b"replacement").is_err());
        assert_eq!(fs::read(&output).unwrap(), b"original");
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn saves_reopens_backs_up_and_exports_original_fixture() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/peach-bot");
        let doc = load(&root.join("mesh-bot.peachy")).unwrap();
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
