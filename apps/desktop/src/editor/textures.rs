use super::*;

/// Before the first frame, egui still has its portable 2048-side default.
/// Seed the limit from the device eframe actually created, rather than guessing
/// from adapter capabilities or changing the artwork to fit that default.
pub(super) fn configure_texture_limit(ctx: &egui::Context, device_limit: Option<usize>) {
    if let Some(limit) = device_limit {
        ctx.input_mut(|input| input.max_texture_side = limit);
    }
}

impl Editor {
    pub(super) fn load_textures(&mut self, ctx: &egui::Context) {
        self.textures.clear();
        self.ghosting.solid_textures.clear();
        let paths: Vec<_> = self.session.document().image_paths().into_iter().collect();
        let mut errors = vec![];
        for path in paths {
            let file = self.asset_root.join("images").join(format!("{path}.png"));
            match load_texture_pair(ctx, &path, &file) {
                Ok((image, solid)) => {
                    self.textures.insert(path.clone(), image);
                    self.ghosting.solid_textures.insert(path, solid);
                }
                Err(error) => errors.push(format!("{path}: {error}")),
            }
        }
        if !errors.is_empty() {
            let message = format!("Could not load PNG images:\n{}", errors.join("\n"));
            self.error = Some(match self.error.take() {
                Some(previous) => format!("{previous}\n{message}"),
                None => message,
            });
        }
    }
}

fn check_size(size: [usize; 2], limit: usize) -> Result<(), String> {
    if size[0] > limit || size[1] > limit {
        return Err(format!(
            "image is {}×{} pixels; this renderer supports at most {limit} pixels per side",
            size[0], size[1]
        ));
    }
    Ok(())
}

fn load_texture_pair(
    ctx: &egui::Context,
    path: &str,
    file: &std::path::Path,
) -> Result<(egui::TextureHandle, egui::TextureHandle), String> {
    let limit = ctx.input(|input| input.max_texture_side);
    // Inspect the header before decoding, so a genuinely oversized image is
    // refused before allocating its pixels or either of the texture handles.
    let (width, height) = image::image_dimensions(file).map_err(|error| error.to_string())?;
    check_size([width as usize, height as usize], limit)?;
    let image = image::open(file)
        .map_err(|error| error.to_string())?
        .into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    // Recheck decoded data too: the file may have changed since header reading.
    check_size(size, limit)?;
    let mut solid = image.clone();
    for pixel in solid.pixels_mut() {
        pixel.0[..3].fill(255);
    }
    let texture = ctx.load_texture(
        path,
        egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    let solid_texture = ctx.load_texture(
        format!("ghost-solid:{path}"),
        egui::ColorImage::from_rgba_unmultiplied(size, solid.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    Ok((texture, solid_texture))
}

#[cfg(test)]
mod tests {
    use super::super::test_support::editor_fixture;
    use super::*;

    struct TestImages(PathBuf);

    impl TestImages {
        fn new() -> Self {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let directory = std::env::temp_dir().join(format!(
                "peachy-texture-regression-{}-{stamp}",
                std::process::id()
            ));
            std::fs::create_dir_all(directory.join("images")).unwrap();
            Self(directory)
        }

        fn write(&self, size: [u32; 2]) -> Vec<u8> {
            let file = self.0.join("images/large.png");
            image::RgbaImage::from_pixel(size[0], size[1], image::Rgba([75, 120, 200, 128]))
                .save(&file)
                .unwrap();
            std::fs::read(file).unwrap()
        }

        fn editor(&self) -> Editor {
            let mut editor = editor_fixture();
            let mut document = editor.session.document().clone();
            for attachment in document
                .skins
                .values_mut()
                .flat_map(|skin| skin.values_mut())
                .flat_map(|attachments| attachments.values_mut())
            {
                let Attachment::Region(region) = attachment else {
                    panic!("Original Peach Bot fixture should contain regions");
                };
                region.path = "large".into();
            }
            editor.session = Session::new(document).unwrap();
            editor.asset_root = self.0.clone();
            editor
        }
    }

    impl Drop for TestImages {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn startup_uploads_original_image_larger_than_egui_default_at_device_limit() {
        let ctx = egui::Context::default();
        assert_eq!(ctx.input(|input| input.max_texture_side), 2048);
        configure_texture_limit(&ctx, Some(4096));
        let files = TestImages::new();
        let source = files.write([2059, 451]);
        let mut editor = files.editor();
        let document = editor.session.document().clone();
        editor.load_textures(&ctx);
        assert!(editor.error.is_none(), "{:?}", editor.error);
        assert_eq!(editor.textures["large"].size(), [2059, 451]);
        assert_eq!(editor.ghosting.solid_textures["large"].size(), [2059, 451]);
        assert_eq!(
            std::fs::read(files.0.join("images/large.png")).unwrap(),
            source
        );
        assert_eq!(editor.session.document(), &document);
        assert!(!editor.session.can_undo());
    }

    #[test]
    fn real_limit_rejection_on_reload_clears_both_texture_maps_without_resizing() {
        let ctx = egui::Context::default();
        configure_texture_limit(&ctx, Some(4096));
        let files = TestImages::new();
        files.write([4096, 1]);
        let mut editor = files.editor();
        editor.load_textures(&ctx);
        assert!(editor.error.is_none());
        assert_eq!(editor.textures.len(), 1);
        assert_eq!(editor.ghosting.solid_textures.len(), 1);

        let source = files.write([1, 4097]);
        editor.load_textures(&ctx);
        assert!(editor.textures.is_empty());
        assert!(editor.ghosting.solid_textures.is_empty());
        let error = editor.error.as_ref().unwrap();
        assert!(error.contains("large"));
        assert!(error.contains("1×4097"));
        assert!(error.contains("4096"));
        assert_eq!(
            std::fs::read(files.0.join("images/large.png")).unwrap(),
            source
        );
    }

    #[test]
    fn absent_device_does_not_inflate_the_reported_limit_or_panic_on_large_images() {
        let ctx = egui::Context::default();
        configure_texture_limit(&ctx, None);
        assert_eq!(ctx.input(|input| input.max_texture_side), 2048);
        let files = TestImages::new();
        files.write([2059, 1]);
        let mut editor = files.editor();
        editor.load_textures(&ctx);
        assert!(editor.textures.is_empty());
        assert!(editor.ghosting.solid_textures.is_empty());
        assert!(editor.error.unwrap().contains("2048"));
    }
}
