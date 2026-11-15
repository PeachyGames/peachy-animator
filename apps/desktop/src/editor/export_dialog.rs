//! Export targets are an explicit per-export choice, never a project setting.
use super::*;
use files::ExportTarget;

pub(super) struct ExportDialog {
    target: ExportTarget,
    first_frame: bool,
}

fn target_label(target: ExportTarget) -> &'static str {
    match target {
        ExportTarget::SpineEditor43 => "Spine Editor 4.3",
        ExportTarget::Runtime43 => "Spine Runtime 4.3",
    }
}

impl Editor {
    pub(super) fn begin_export(&mut self) {
        self.cancel_gesture();
        self.playing = false;
        self.pause_audio_for_modal();
        self.export_dialog = Some(ExportDialog {
            target: ExportTarget::SpineEditor43,
            first_frame: true,
        });
    }

    pub(super) fn export_dialog_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if self.export_dialog.is_none() {
            return false;
        }
        if !ctx.input(|input| input.focused)
            || ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.export_dialog = None;
            return true;
        }
        if ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F9)) {
            self.toggle_views();
            return true;
        }
        // As with a held authoring preview, history first dismisses the modal;
        // it must not unexpectedly undo the document behind the choice.
        if ctx.input_mut(|input| {
            input.consume_key(egui::Modifiers::COMMAND, egui::Key::Z)
                || input.consume_key(
                    egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
                    egui::Key::Z,
                )
                || input.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
        }) {
            self.export_dialog = None;
        }
        true
    }

    /// Presentation returns the confirmed target. The caller owns the native
    /// directory chooser and I/O, so canceling this modal cannot write anything.
    pub(super) fn export_target_dialog(&mut self, ctx: &egui::Context) -> Option<ExportTarget> {
        let mut state = self.export_dialog.take()?;
        if !ctx.input(|input| input.focused)
            || self.error.is_some()
            || self.rename.is_some()
            || self.duplicate_animation.is_some()
        {
            return None;
        }
        let mut confirm = false;
        let mut cancel = false;
        let response = egui::Modal::new(egui::Id::new("export-target")).show(ctx, |ui| {
            ui.set_min_width(400.);
            ui.heading("Export Spine 4.3");
            ui.label("Choose how this export will be used.");
            ui.separator();
            for (target, name, explanation) in [
                (ExportTarget::SpineEditor43, "exportTarget:spineEditor43",
                    "Checks attachment naming to avoid lost references when importing into the editor."),
                (ExportTarget::Runtime43, "exportTarget:runtime43",
                    "Retains native attachment names and placeholders for runtime consumers."),
            ] {
                let choice = ui.radio_value(&mut state.target, target, target_label(target));
                self.record_target(name, choice.rect);
                if state.first_frame && target == ExportTarget::SpineEditor43 { choice.request_focus(); }
                ui.add(egui::Label::new(RichText::new(explanation).small().color(MUTED)).wrap());
                ui.add_space(5.);
            }
            if !self.session.pose_edits().is_empty() {
                ui.label(RichText::new("Unkeyed poses are excluded. Press K before exporting to include them.").small().color(ACCENT));
            }
            ui.separator();
            ui.horizontal(|ui| {
                let next = ui.button("Continue…");
                self.record_target("exportTargetContinue", next.rect);
                let close = ui.button("Cancel");
                self.record_target("exportTargetCancel", close.rect);
                confirm = next.clicked();
                cancel = close.clicked();
            });
        });
        self.record_target("exportTargetDialog", response.response.rect);
        if cancel || response.should_close() {
            return None;
        }
        if confirm {
            return Some(state.target);
        }
        state.first_frame = false;
        self.export_dialog = Some(state);
        None
    }

    pub(super) fn export_to_target(&mut self, target: ExportTarget) {
        let label = target_label(target);
        if let Some(folder) = rfd::FileDialog::new()
            .set_title(format!("Choose parent folder for a new {label} export"))
            .pick_folder()
        {
            match files::export_bundle_for(
                &folder,
                self.session.document(),
                &self.asset_root,
                target,
            ) {
                Ok(path) => {
                    self.status = format!(
                        "{label} export saved to {}{}",
                        path.display(),
                        if self.session.pose_edits().is_empty() {
                            ""
                        } else {
                            "; unkeyed pose excluded — press K to key it"
                        }
                    )
                }
                Err(error) => self.error = Some(format!("{label} export failed:\n{error}")),
            }
        }
    }

    pub(super) fn export_dialog_diagnostic(&self) -> serde_json::Value {
        self.export_dialog.as_ref().map_or(serde_json::Value::Null, |state| serde_json::json!({
            "target":match state.target { ExportTarget::SpineEditor43 => "spineEditor43", ExportTarget::Runtime43 => "runtime43" },
            "label":target_label(state.target), "unkeyedPoseExcluded":!self.session.pose_edits().is_empty(),
        }))
    }
}

#[cfg(test)]
#[path = "export_dialog_tests.rs"]
mod tests;
