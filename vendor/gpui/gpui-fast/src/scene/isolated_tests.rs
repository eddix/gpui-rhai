use super::*;
use crate::size;
fn source() -> IsolatedGroupSource {
    let mut scene = Scene::default();
    scene.insert_primitive(Quad {
        bounds: Bounds {
            origin: point(ScaledPixels(0.), ScaledPixels(0.)),
            size: size(ScaledPixels(8.), ScaledPixels(8.)),
        },
        content_mask: ContentMask {
            bounds: Bounds {
                origin: point(ScaledPixels(0.), ScaledPixels(0.)),
                size: size(ScaledPixels(32.), ScaledPixels(32.)),
            },
        },
        background: crate::rgba(0xff0000ff).into(),
        ..Default::default()
    });
    IsolatedGroupSource::new(scene, size(crate::DevicePixels(8), crate::DevicePixels(8))).unwrap()
}
fn group() -> IsolatedGroup {
    IsolatedGroup::new(
        source(),
        TransformationMatrix {
            rotation_scale: [[0.8, -0.3], [0.3, 0.8]],
            translation: [10., 8.],
        },
        0.6,
        ContentMask {
            bounds: Bounds {
                origin: point(ScaledPixels(0.), ScaledPixels(0.)),
                size: size(ScaledPixels(64.), ScaledPixels(64.)),
            },
        },
    )
    .unwrap()
}
#[cfg_attr(test, test)]
pub(super) fn isolated_group_translation_preserves_source_and_one_inverse_mapping() {
    let original = group();
    let mut moved = original.clone();
    let delta = point(ScaledPixels(7.), ScaledPixels(-3.));
    moved.translate(delta);
    assert_eq!(original.source_identity(), moved.source_identity());
    assert_eq!(moved.bounds.origin, original.bounds.origin + delta);
    for p in [
        point(crate::px(15.), crate::px(12.)),
        point(crate::px(20.), crate::px(18.)),
    ] {
        let old = original.inverse().unwrap().apply(p);
        let new = moved
            .inverse()
            .unwrap()
            .apply(p + point(crate::px(7.), crate::px(-3.)));
        assert!((old.x.0 - new.x.0).abs() < 1e-5 && (old.y.0 - new.y.0).abs() < 1e-5);
    }
}
#[cfg_attr(test, test)]
pub(super) fn isolated_group_finish_is_idempotent_and_leaves_plain_spans_batched() {
    let mut scene = Scene::default();
    let mask = ContentMask {
        bounds: Bounds {
            origin: point(ScaledPixels(0.), ScaledPixels(0.)),
            size: size(ScaledPixels(320.), ScaledPixels(64.)),
        },
    };
    for n in 0..100 {
        scene.insert_primitive(Quad {
            bounds: Bounds {
                origin: point(ScaledPixels(n as f32 * 3.), ScaledPixels(0.)),
                size: size(ScaledPixels(2.), ScaledPixels(2.)),
            },
            content_mask: ContentMask {
                bounds: Bounds {
                    origin: point(ScaledPixels(0.), ScaledPixels(0.)),
                    size: size(ScaledPixels(320.), ScaledPixels(32.)),
                },
            },
            background: crate::rgba(0xffffffff).into(),
            ..Default::default()
        });
    }
    scene.insert_primitive(
        IsolatedGroup::new(
            source(),
            TransformationMatrix {
                translation: [305., 10.],
                ..TransformationMatrix::unit()
            },
            1.,
            mask,
        )
        .unwrap(),
    );
    scene.finish();
    let first = scene.batches().map(|b| b.label()).collect::<Vec<_>>();
    scene.finish();
    assert_eq!(
        first,
        scene.batches().map(|b| b.label()).collect::<Vec<_>>()
    );
    assert_eq!(first.len(), 2);
}
#[cfg_attr(test, test)]
pub(super) fn isolated_group_live_overlay_retains_background_tiles() {
    let mut scene = Scene::default();
    let mask = group().content_mask;
    scene.insert_primitive(Quad {
        bounds: mask.bounds,
        content_mask: mask,
        background: crate::rgba(0xffffffff).into(),
        ..Default::default()
    });
    scene.insert_primitive(group());
    scene.insert_primitive(Quad {
        bounds: Bounds {
            origin: point(ScaledPixels(11.), ScaledPixels(10.)),
            size: size(ScaledPixels(4.), ScaledPixels(4.)),
        },
        content_mask: mask,
        background: crate::rgba(0x00ff00ff).into(),
        ..Default::default()
    });
    scene.finish();
    let (base, overlay) = crate::fast::layers::overlay::lift(scene);
    assert!(base.isolated_groups.is_empty());
    assert_eq!(base.quads.len(), 1);
    assert_eq!(overlay.len(), 2);
    assert!(matches!(&overlay[0], Primitive::IsolatedGroup(_)));
    assert!(matches!(&overlay[1], Primitive::Quad(_)));
}
#[cfg(feature = "test-support")]
#[cfg_attr(test, test)]
pub(super) fn isolated_group_scoped_content_keeps_live_replay_instead_of_tile_pixels() {
    let mut scene = Scene::default();
    let mask = group().content_mask;
    scene.with_test_isolated_rounded_clip(mask.bounds, 6., |scene| {
        scene.insert_primitive(group());
    });
    scene.finish();
    let (base, overlay) = crate::fast::layers::overlay::lift(scene);
    assert!(overlay.is_empty());
    assert!(base.has_isolated_groups());
    assert!(
        base.paint_operations
            .iter()
            .any(|op| matches!(op, PaintOperation::Primitive(Primitive::IsolatedGroup(_))))
    );
}

#[cfg(feature = "test-support")]
impl Scene {
    /// Self-contained protocol assertions; no missing upstream asset fixtures are substituted.
    pub fn run_isolated_group_protocol_checks() {
        isolated_group_translation_preserves_source_and_one_inverse_mapping();
        isolated_group_finish_is_idempotent_and_leaves_plain_spans_batched();
        isolated_group_live_overlay_retains_background_tiles();
        isolated_group_scoped_content_keeps_live_replay_instead_of_tile_pixels();
    }
}
