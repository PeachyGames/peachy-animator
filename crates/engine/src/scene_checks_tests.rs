use crate::*;
use peachy_document::*;
use std::collections::BTreeMap;

fn fixture() -> Document {
    Document {
        constraints: vec![],
        format: FORMAT_VERSION,
        name: "Original checked evaluation fixture".into(),
        fps: 30,
        bones: vec![Bone {
            name: "root".into(),
            parent: None,
            length: 10.,
            setup: Transform::default(),
            inherit: peachy_document::Inherit::Normal,
        }],
        slots: vec![Slot {
            color: [255; 4],
            name: "body".into(),
            bone: 0,
            attachment: Some("triangle".into()),
        }],
        skins: BTreeMap::from([(
            "default".into(),
            BTreeMap::from([(
                "body".into(),
                BTreeMap::from([(
                    "triangle".into(),
                    Attachment::Mesh(Mesh {
                        name: None,
                        path: "original-triangle".into(),
                        vertices: MeshVertices::SlotLocal {
                            positions: vec![[0., 0.], [2., 0.], [0., 1.]],
                        },
                        uvs: vec![[0., 0.], [1., 0.], [0., 1.]],
                        triangles: vec![0, 1, 2],
                        hull: 3,
                        edges: vec![],
                        width: 0.,
                        height: 0.,
                    }),
                )]),
            )]),
        )]),
        animations: vec![Animation {
            ik_tracks: vec![],
            name: "pose".into(),
            tracks: vec![],
            physics: vec![],
            physics_resets: vec![],
            inherit_tracks: vec![],
            deform_tracks: vec![],
            attachment_tracks: vec![],
            color_tracks: vec![],
            events: vec![],
        }],
        skin_scope: Default::default(),
        events: vec![],
        reference_scale: 100.0,
    }
}

fn mesh(document: &mut Document) -> &mut Mesh {
    let Attachment::Mesh(mesh) = document
        .skins
        .get_mut("default")
        .unwrap()
        .get_mut("body")
        .unwrap()
        .get_mut("triangle")
        .unwrap()
    else {
        panic!()
    };
    mesh
}

#[test]
fn checked_evaluation_accepts_finite_wide_shear_and_reports_unrepresentable_overshoot() {
    let mut document = fixture();
    document.animations[0].tracks = vec![Track {
        bone: 0,
        property: Property::ShearX,
        keys: vec![
            Keyframe {
                time: 0.,
                value: -2e38,
                curve: Curve::Bezier([0.25, 0., 0.75, 1.]),
            },
            Keyframe {
                time: 1.,
                value: 2e38,
                curve: Curve::Linear,
            },
        ],
    }];
    document.validate().unwrap();
    let before = document.clone();
    for time in [0., 0.25, 0.5, 0.75, 1., 1.75] {
        let scene = checked_evaluate_scene(&document, Some(0), time, "default", None).unwrap();
        assert!(scene.locals[0].shear_x.is_finite());
        assert!(
            scene.attachments[0]
                .vertices
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
    }
    assert_eq!(
        checked_evaluate_scene(&document, Some(0), 0.5, "default", None)
            .unwrap()
            .locals[0]
            .shear_x,
        0.
    );
    assert_eq!(document, before);
    document.animations[0].tracks[0].keys[0].value = 0.;
    document.animations[0].tracks[0].keys[0].curve =
        Curve::Bezier([0.25, f32::MAX, 0.75, f32::MAX]);
    document.animations[0].tracks[0].keys[1].value = 2.;
    document.validate().unwrap();
    let error = checked_evaluate_scene(&document, Some(0), 0.5, "default", None)
        .err()
        .unwrap();
    assert!(
        error.contains("root") && error.contains("local transform") && error.contains("nonfinite")
    );
}

#[test]
fn checked_locals_reject_wrong_counts_nonfinite_previews_and_world_overflow() {
    let document = fixture();
    assert!(
        checked_setup_scene_from_locals(&document, "default", vec![])
            .err()
            .unwrap()
            .contains("one local transform")
    );
    assert!(
        checked_setup_scene_from_locals(
            &document,
            "default",
            vec![Transform {
                shear_y: f32::NAN,
                ..Default::default()
            }]
        )
        .err()
        .unwrap()
        .contains("local transform")
    );
    assert!(
        checked_evaluate_scene(
            &document,
            None,
            0.,
            "default",
            Some((1, Transform::default()))
        )
        .err()
        .unwrap()
        .contains("Unknown preview bone")
    );
    assert!(
        checked_evaluate_scene(
            &document,
            None,
            0.,
            "default",
            Some((
                0,
                Transform {
                    scale_x: f32::INFINITY,
                    ..Default::default()
                }
            ))
        )
        .is_err()
    );
    let mut large = document.clone();
    large.bones[0].setup.scale_x = f32::MAX;
    large.bones.push(Bone {
        name: "overflowing-child".into(),
        parent: Some(0),
        length: 0.,
        setup: Transform {
            scale_x: 2.,
            ..Default::default()
        },
        inherit: peachy_document::Inherit::Normal,
    });
    large.validate().unwrap();
    assert!(
        checked_evaluate_scene(&large, None, 0., "default", None)
            .err()
            .unwrap()
            .contains("overflowing-child: evaluated world transform")
    );
    assert_eq!(document, fixture());
}

#[test]
fn emitted_geometry_uv_and_unrepresentable_bounds_are_checked_before_shell_use() {
    let mut geometry = fixture();
    geometry.bones[0].setup.scale_x = f32::MAX;
    geometry.validate().unwrap();
    assert!(
        checked_evaluate_scene(&geometry, None, 0., "default", None)
            .err()
            .unwrap()
            .contains("geometry")
    );
    // UVs are copied to draw buffers; this also guards a caller that violates
    // the validated-document precondition without allowing bad output onward.
    let mut uv = fixture();
    mesh(&mut uv).uvs[0][1] = f32::INFINITY;
    assert!(
        checked_evaluate_scene(&uv, None, 0., "default", None)
            .err()
            .unwrap()
            .contains("UVs")
    );
    let mut extent = fixture();
    mesh(&mut extent).vertices = MeshVertices::SlotLocal {
        positions: vec![[-2e38, 0.], [2e38, 0.], [0., 1.]],
    };
    extent.validate().unwrap();
    assert!(
        checked_evaluate_scene(&extent, None, 0., "default", None)
            .err()
            .unwrap()
            .contains("bounds")
    );
}

#[test]
fn a_representable_large_bounds_center_uses_wide_intermediates_instead_of_failing_evaluation() {
    let mut document = fixture();
    mesh(&mut document).vertices = MeshVertices::SlotLocal {
        positions: vec![[2e38, 0.], [2.5e38, 0.], [2e38, 1.]],
    };
    document.validate().unwrap();
    let bounds = checked_evaluate_scene(&document, None, 0., "default", None)
        .unwrap()
        .bounds
        .unwrap();
    assert!(bounds.center()[0].is_finite());
    assert!((f64::from(bounds.center()[0]) - 2.25e38).abs() / 2.25e38 < 0.000001);
    assert!(bounds.width().is_finite());
}
