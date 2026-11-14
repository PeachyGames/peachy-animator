//! Explicit key requests settle after text parsing, inside the owner's current transaction.
use super::*;

#[derive(Clone)]
pub(super) enum PoseKeyAction {
    Transform {
        bone: usize,
        properties: Vec<Property>,
    },
    Edited {
        bone: Option<usize>,
    },
    Ik {
        constraint: usize,
    },
    Deform {
        attachment: AttachmentRef,
    },
}

pub(super) struct PoseKeyRequest {
    action: PoseKeyAction,
    animation: usize,
    time: f32,
}

impl Editor {
    pub(super) fn request_pose_key(&mut self, action: PoseKeyAction) {
        if self.setup_mode {
            return;
        }
        // Mesh weights are global setup edits, not authored animation pose values.
        self.cancel_weight_edit();
        self.cancel_dopesheet_drag();
        if self.mesh_drag.take().is_some() {
            self.session.cancel_transaction();
        }
        self.playing = false;
        let request = PoseKeyRequest {
            action,
            animation: self.animation,
            time: self.time,
        };
        if self.drag.is_some() || self.ik_edit.is_some() || self.deform_drag.is_some() {
            self.pose_key_request = Some(request);
        } else {
            self.apply_pose_key(request);
        }
    }

    fn apply_pose_key(&mut self, request: PoseKeyRequest) {
        let result = match request.action {
            PoseKeyAction::Transform { bone, properties } => {
                self.session
                    .key_pose(request.animation, request.time, bone, &properties)
            }
            PoseKeyAction::Edited { bone } => {
                self.session
                    .key_edited(request.animation, request.time, bone)
            }
            PoseKeyAction::Ik { constraint } => {
                self.session
                    .key_ik_pose(request.animation, request.time, constraint)
            }
            PoseKeyAction::Deform { attachment } => {
                self.session
                    .key_deform_pose(request.animation, request.time, attachment)
            }
        };
        match result {
            Ok(()) => {
                self.seal_transform_for_key();
                self.seal_ik_for_key();
                self.seal_deform_for_key();
                self.session.commit_transaction();
                self.status = "Keyed the current authored pose".into();
            }
            Err(error) => {
                self.hold_transform_key_error(&error);
                self.hold_ik_key_error(&error);
                self.status = format!("Cannot key the current pose: {error}");
            }
        }
    }

    pub(super) fn settle_pose_key(&mut self) {
        let Some(request) = self.pose_key_request.take() else {
            return;
        };
        if self.setup_mode || self.animation != request.animation || self.time != request.time {
            return;
        }
        if let Err(error) = self
            .transform_key_ready()
            .and_then(|()| self.ik_key_ready())
        {
            self.status = format!("Cannot key the current pose: {error}");
            return;
        }
        self.apply_pose_key(request);
    }
}
