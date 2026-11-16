//! All Graph channels and Curves share one typed handle edit and numeric draft.
use super::curve_gesture::{CurveGesture, CurveInputOwner};
use super::*;
use peachy_engine::{
    AnimationKey, CurveChannel, CurveData, CurveSelection, CurveTarget, curve_channels_for_targets,
    curve_edit_targets,
};

pub(super) struct HandleEdit {
    gesture: CurveGesture,
    primary: CurveChannel,
    selection: Vec<CurveSelection>,
    targets: Vec<CurveTarget>,
    expected: Vec<CurveChannel>,
    normalized: bool,
    match_curves: bool,
    separate: bool,
    origin: Pos2,
    moved: bool,
}

pub(super) struct HandleDraft {
    owner: CurveInputOwner,
    source: CurveChannel,
    selection: Vec<CurveSelection>,
    text: [String; 4],
    changed: bool,
}

pub(super) struct CurveSurface {
    pub owner: CurveInputOwner,
    pub rect: egui::Rect,
    pub bounds: [f64; 4],
    pub normalized: bool,
    pub color: Color32,
}

pub(super) fn value_handles(channel: &CurveChannel) -> Option<[f64; 4]> {
    let [left, right] = channel.values;
    let span = right - left;
    match channel.curve {
        CurveData::Standard(Curve::Bezier(h)) => Some([
            f64::from(h[0]),
            left + span * f64::from(h[1]),
            f64::from(h[2]),
            left + span * f64::from(h[3]),
        ]),
        CurveData::Standard(Curve::BezierOffset(h)) => Some([
            f64::from(h[0]),
            left + f64::from(h[1]),
            f64::from(h[2]),
            left + f64::from(h[3]),
        ]),
        CurveData::Ik(IkCurve::Bezier { time, mix }) => {
            Some([time[0], f64::from(mix[0]), time[1], f64::from(mix[1])])
        }
        _ => None,
    }
}

impl Editor {
    pub(super) fn shared_curve_owner(&self) -> Option<CurveInputOwner> {
        self.curves.edit.as_ref().map(|edit| edit.gesture.owner)
    }

    pub(super) fn active_curve_bounds(&self, owner: CurveInputOwner) -> Option<[f64; 4]> {
        self.curves
            .edit
            .as_ref()
            .filter(|edit| edit.gesture.owner == owner)
            .map(|edit| edit.gesture.bounds)
    }

    pub(super) fn graph_curve_channel(
        &self,
        key: AnimationKey,
        next_time: f32,
        channel: usize,
    ) -> Option<CurveChannel> {
        peachy_engine::curve_channels(
            self.session.document(),
            self.animation,
            &CurveTarget { key, next_time },
        )
        .ok()?
        .into_iter()
        .find(|value| value.channel == channel)
    }

    fn curve_action_selection(
        &self,
        owner: CurveInputOwner,
        channel: &CurveChannel,
    ) -> Vec<CurveSelection> {
        if self.key_selection.is_empty() {
            return vec![CurveSelection {
                key: channel.target.key.clone(),
                channel: channel.channel,
            }];
        }
        if owner == CurveInputOwner::Curves {
            return self.curve_preset_selection();
        }
        self.key_selection
            .keys()
            .iter()
            .map(|key| CurveSelection {
                key: key.clone(),
                channel: if matches!(key, AnimationKey::SlotColor { .. }) {
                    channel.channel
                } else {
                    0
                },
            })
            .collect()
    }

    pub(super) fn curve_handle_diagnostic(&self) -> serde_json::Value {
        serde_json::json!({
            "draft":self.curves.draft.as_ref().map(|draft|serde_json::json!({"owner":format!("{:?}",draft.owner),"text":draft.text,"changed":draft.changed,"source":draft.source.target.key,"channel":draft.source.channel})),
            "editing":self.curves.edit.as_ref().map(|edit|serde_json::json!({"owner":format!("{:?}",edit.gesture.owner),"handle":edit.gesture.handle,"source":edit.primary.target.key,"channel":edit.primary.channel,"finishPending":edit.gesture.finish_pending})),
        })
    }

    pub(super) fn shared_curve_edit_active(&self) -> bool {
        self.curves.edit.is_some()
    }

    pub(super) fn cancel_shared_curve_edit(&mut self) {
        if let Some(edit) = self.curves.edit.take() {
            self.cancel_curve_gesture(edit.gesture.owner);
        }
        self.curves.draft = None;
    }

    pub(super) fn reconcile_shared_curve_input(&mut self, ctx: &egui::Context) {
        if self.curves.edit.as_ref().is_some_and(|edit| {
            let visible = match edit.gesture.owner {
                CurveInputOwner::Curves => self.curves_visible(),
                CurveInputOwner::ScalarGraph
                | CurveInputOwner::SliderGraph
                | CurveInputOwner::DeformGraph
                | CurveInputOwner::SlotGraph => self.graph_visible(),
            };
            !self.curve_gesture_context_valid(&edit.gesture, ctx, visible)
                || self.curves.separate != edit.separate
                || (edit.gesture.owner == CurveInputOwner::SlotGraph
                    && self.slot_graph_channel() != edit.primary.channel)
                || (edit.gesture.owner == CurveInputOwner::Curves
                    && self.curves.match_handles != edit.match_curves)
                || curve_channels_for_targets(
                    self.session.document(),
                    self.animation,
                    &edit.targets,
                )
                .map_or(true, |current| current != edit.expected)
        }) {
            self.cancel_shared_curve_edit();
        }
    }

    pub(super) fn settle_shared_curve_input(&mut self, ctx: &egui::Context) {
        self.reconcile_shared_curve_input(ctx);
        let finish = self
            .curves
            .edit
            .as_ref()
            .map(|edit| (edit.gesture.owner, edit.gesture.finish_pending));
        if let Some((owner, pending)) = finish
            && self.settle_curve_gesture(owner, ctx, pending)
        {
            self.curves.edit = None;
        }
    }

    pub(super) fn curve_handles_on_surface(
        &mut self,
        ui: &mut egui::Ui,
        channel: &CurveChannel,
        surface: CurveSurface,
    ) {
        let CurveSurface {
            owner,
            rect,
            bounds,
            normalized,
            color,
        } = surface;
        if self.curves.edit.as_ref().is_some_and(|edit| {
            edit.gesture.owner == owner
                && (edit.gesture.rect != rect || edit.gesture.bounds != bounds)
        }) {
            self.cancel_shared_curve_edit();
        }
        let Some(mut handles) = value_handles(channel) else {
            return;
        };
        if normalized {
            let span = channel.values[1] - channel.values[0];
            if span == 0. {
                return;
            }
            handles[1] = (handles[1] - channel.values[0]) / span;
            handles[3] = (handles[3] - channel.values[0]) / span;
        } else {
            let start = f64::from(channel.target.key.time());
            let length = f64::from(channel.target.next_time) - start;
            handles[0] = start + handles[0] * length;
            handles[2] = start + handles[2] * length;
        }
        let project = |x: f64, y: f64| {
            pos2(
                (f64::from(rect.left())
                    + (x - bounds[0]) / (bounds[1] - bounds[0]) * f64::from(rect.width()))
                    as f32,
                (f64::from(rect.bottom())
                    - (y - bounds[2]) / (bounds[3] - bounds[2]) * f64::from(rect.height()))
                    as f32,
            )
        };
        if !rect.is_finite()
            || rect.width() <= 0.
            || rect.height() <= 0.
            || !bounds.into_iter().all(f64::is_finite)
            || bounds[1] <= bounds[0]
            || bounds[3] <= bounds[2]
        {
            return;
        }
        let painter = ui.painter_at(rect);
        let mut positions = Vec::new();
        for index in 0..2 {
            let point = project(handles[index * 2], handles[index * 2 + 1]);
            let anchor = if normalized {
                project(index as f64, index as f64)
            } else {
                project(
                    if index == 0 {
                        f64::from(channel.target.key.time())
                    } else {
                        f64::from(channel.target.next_time)
                    },
                    channel.values[index],
                )
            };
            if point.is_finite() && anchor.is_finite() {
                painter.line_segment([anchor, point], Stroke::new(1., color));
            }
            if point.is_finite() && rect.contains(point) && ui.clip_rect().contains(point) {
                painter.circle_filled(point, 4., color);
                self.record_target(
                    &format!(
                        "{}Handle:{index}",
                        match owner {
                            CurveInputOwner::Curves => "curves",
                            CurveInputOwner::ScalarGraph => "graphScalar",
                            CurveInputOwner::SliderGraph => "graphSlider",
                            CurveInputOwner::DeformGraph => "graphDeform",
                            CurveInputOwner::SlotGraph => "graphSlot",
                        }
                    ),
                    egui::Rect::from_center_size(point, vec2(12., 12.)),
                );
                positions.push((index, point));
            }
        }
        let press = ui.input(|input| super::key_drag::primary_pointer_event(&input.events, true));
        if self.curves.edit.is_none()
            && !self.curve_gesture_blocked(ui.ctx())
            && self.slot_authoring_input_available(ui.ctx())
            && ui.is_enabled()
            && let Some((pointer, _)) = press
            && rect.contains(pointer)
            && ui.clip_rect().contains(pointer)
            && ui.rect_contains_pointer(rect)
            && let Some((handle, _)) = positions
                .iter()
                .find(|(_, point)| point.distance(pointer) <= 9.)
        {
            let selection = self.curve_action_selection(owner, channel);
            let targets = curve_edit_targets(self.session.document(), self.animation, &selection);
            if let Ok(targets) = targets
                && let Ok(expected) =
                    curve_channels_for_targets(self.session.document(), self.animation, &targets)
            {
                let gesture = self.begin_curve_gesture(owner, *handle, rect, bounds);
                self.curves.edit = Some(HandleEdit {
                    gesture,
                    primary: channel.clone(),
                    selection,
                    targets,
                    expected,
                    normalized,
                    match_curves: normalized && self.curves.match_handles,
                    separate: self.curves.separate,
                    origin: pointer,
                    moved: false,
                });
                ui.memory_mut(|memory| {
                    memory.request_focus(if normalized {
                        Self::curves_id()
                    } else {
                        Self::graph_id()
                    })
                });
            }
        }
        let release =
            ui.input(|input| super::key_drag::primary_pointer_event(&input.events, false));
        if self
            .curves
            .edit
            .as_ref()
            .is_some_and(|edit| edit.gesture.owner == owner)
        {
            let pointer = release
                .map(|(point, _)| point)
                .or_else(|| ui.input(|input| input.pointer.interact_pos()));
            if let Some(pointer) = pointer {
                self.update_shared_curve_handle(pointer);
            }
            if release.is_some()
                && let Some(edit) = self.curves.edit.as_mut()
            {
                edit.gesture.finish_pending = true;
            }
        }
    }

    fn update_shared_curve_handle(&mut self, pointer: Pos2) {
        let Some(mut edit) = self.curves.edit.take() else {
            return;
        };
        edit.moved |= pointer.distance(edit.origin) >= 2.;
        if !edit.moved {
            self.curves.edit = Some(edit);
            return;
        }
        let Some(mut point) = Self::curve_gesture_coordinates(&edit.gesture, pointer) else {
            self.cancel_curve_gesture(edit.gesture.owner);
            return;
        };
        if edit.normalized {
            let start = f64::from(edit.primary.target.key.time());
            point[0] = start + point[0] * (f64::from(edit.primary.target.next_time) - start);
            point[1] = edit.primary.values[0]
                + point[1] * (edit.primary.values[1] - edit.primary.values[0]);
        }
        let command = Command::EditCurveHandle {
            animation: self.animation,
            selection: edit.selection.clone(),
            primary: CurveSelection {
                key: edit.primary.target.key.clone(),
                channel: edit.primary.channel,
            },
            handle: edit.gesture.handle,
            point,
            match_curves: edit.match_curves,
            separate: edit.separate,
        };
        match self.session.preview_transaction(command).and_then(|()| {
            curve_channels_for_targets(self.session.document(), self.animation, &edit.targets)
        }) {
            Ok(expected) => {
                edit.expected = expected;
                self.curves.edit = Some(edit);
                self.curves.error = None;
            }
            Err(error) => {
                self.cancel_curve_gesture(edit.gesture.owner);
                self.status = error.clone();
                self.curves.error = Some(error);
            }
        }
    }

    pub(super) fn scalar_curve_numeric_controls(
        &mut self,
        ui: &mut egui::Ui,
        channel: &CurveChannel,
    ) {
        self.graph_curve_numeric_controls(ui, CurveInputOwner::ScalarGraph, channel);
    }

    pub(super) fn graph_curve_numeric_controls(
        &mut self,
        ui: &mut egui::Ui,
        owner: CurveInputOwner,
        channel: &CurveChannel,
    ) {
        let Some(values) = value_handles(channel) else {
            return;
        };
        let selection = self.curve_action_selection(owner, channel);
        let percent = matches!(
            owner,
            CurveInputOwner::SlotGraph | CurveInputOwner::DeformGraph
        );
        let slider = owner == CurveInputOwner::SliderGraph;
        let fps = f64::from(self.session.document().fps);
        let start = f64::from(channel.target.key.time());
        let duration = f64::from(channel.target.next_time) - start;
        let value_factor = if slider {
            if matches!(
                channel.target.key,
                AnimationKey::Slider {
                    property: SliderProperty::Time,
                    ..
                }
            ) {
                fps
            } else {
                100.
            }
        } else if percent {
            100.
        } else {
            1.
        };
        let original_text: [String; 4] = std::array::from_fn(|index| {
            if index % 2 == 1 {
                (values[index] * value_factor).to_string()
            } else if slider {
                ((start + values[index] * duration) * fps).to_string()
            } else {
                values[index].to_string()
            }
        });
        if self.curves.draft.as_ref().is_none_or(|draft| {
            draft.owner != owner || draft.source != *channel || draft.selection != selection
        }) {
            self.curves.draft = Some(HandleDraft {
                owner,
                source: channel.clone(),
                selection,
                text: original_text.clone(),
                changed: false,
            });
        }
        let enabled = self.slot_authoring_input_available(ui.ctx());
        let horizontal = owner != CurveInputOwner::SlotGraph && ui.available_width() > 650.;
        let prefix = match owner {
            CurveInputOwner::DeformGraph => "graphDeform",
            CurveInputOwner::SlotGraph => "graphSlot",
            CurveInputOwner::SliderGraph => "graphSlider",
            _ => "graphScalar",
        };
        let fields = {
            let draft = self.curves.draft.as_mut().unwrap();
            let fields = curve_widgets::handle_fields(
                ui,
                prefix,
                &mut draft.text,
                draft.changed,
                enabled,
                horizontal,
                if slider
                    && matches!(
                        channel.target.key,
                        AnimationKey::Slider {
                            property: SliderProperty::Time,
                            ..
                        }
                    )
                {
                    [
                        "Host frame 1",
                        "Referenced frame 1",
                        "Host frame 2",
                        "Referenced frame 2",
                    ]
                } else if slider {
                    ["Host frame 1", "Mix 1 (%)", "Host frame 2", "Mix 2 (%)"]
                } else if percent {
                    ["Time 1", "Value 1 (%)", "Time 2", "Value 2 (%)"]
                } else {
                    ["Time 1", "Value 1", "Time 2", "Value 2"]
                },
            );
            draft.changed = fields.changed;
            fields
        };
        for (index, rect) in fields.rects.into_iter().enumerate() {
            self.record_target(&format!("{prefix}Value:{index}"), rect);
        }
        if fields.cancel || !enabled {
            self.curves.draft = None;
            self.curves.error = None;
            return;
        }
        if fields.apply {
            let draft = self.curves.draft.take().unwrap();
            let parsed: Option<Vec<f64>> = draft
                .text
                .iter()
                .enumerate()
                .map(|(index, text)| {
                    if *text == original_text[index] {
                        Some(values[index])
                    } else {
                        text.parse::<f64>()
                            .ok()
                            .filter(|value| value.is_finite())
                            .map(|value| {
                                if index % 2 == 1 {
                                    value / value_factor
                                } else if slider {
                                    (value / fps - start) / duration
                                } else {
                                    value
                                }
                            })
                    }
                })
                .collect();
            let Some(parsed) = parsed else {
                self.curves.error = Some("Bezier handles must be finite numbers.".into());
                return;
            };
            let start = f64::from(channel.target.key.time());
            let duration = f64::from(channel.target.next_time) - start;
            let mut commands = Vec::new();
            for handle in 0..2 {
                if parsed[handle * 2] != values[handle * 2]
                    || parsed[handle * 2 + 1] != values[handle * 2 + 1]
                {
                    commands.push(Command::EditCurveHandle {
                        animation: self.animation,
                        selection: draft.selection.clone(),
                        primary: CurveSelection {
                            key: channel.target.key.clone(),
                            channel: channel.channel,
                        },
                        handle,
                        point: [
                            start + parsed[handle * 2] * duration,
                            parsed[handle * 2 + 1],
                        ],
                        match_curves: false,
                        separate: self.curves.separate,
                    });
                }
            }
            if !commands.is_empty() {
                self.cancel_gesture();
                self.playing = false;
                self.dispatch(Command::Batch { commands });
                self.curves.error = None;
            }
        }
    }
}
